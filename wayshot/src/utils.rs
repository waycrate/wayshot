use clap::ValueEnum;
use eyre::{ContextCompat, Error, bail};
use image::{
    DynamicImage,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
#[cfg(feature = "jxl")]
use jpegxl_rs::encode::{EncoderResult, EncoderSpeed};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    io::Cursor,
    path::{Path, PathBuf},
    result::Result,
    str::FromStr,
};

use chrono::Local;
#[cfg(any(feature = "selector", feature = "color_picker"))]
use libwayshot::{
    Result as WayshotResult,
    region::{LogicalRegion, Position, Region, Size},
};

use crate::config::{Jxl, Png};

// ─── Region helpers ───────────────────────────────────────────────────────────

#[cfg(any(feature = "selector", feature = "color_picker"))]
pub fn waysip_to_region(
    size: libwaysip::Size,
    position: libwaysip::Position,
) -> WayshotResult<LogicalRegion> {
    let size = Size {
        width: size.width.try_into().map_err(|_| {
            libwayshot::Error::FreezeCallbackError("width cannot be negative".to_string())
        })?,
        height: size.height.try_into().map_err(|_| {
            libwayshot::Error::FreezeCallbackError("height cannot be negative".to_string())
        })?,
    };
    Ok(LogicalRegion {
        inner: Region {
            position: Position {
                x: position.x,
                y: position.y,
            },
            size,
        },
    })
}

/// Run WaySip area selection and return the chosen region. Used for both freeze and live paths.
#[cfg(feature = "selector")]
pub fn get_region_area(conn: &libwayshot::WayshotConnection) -> Result<LogicalRegion, String> {
    let info = libwaysip::WaySip::new()
        .with_connection(conn.conn.clone())
        .with_selection_type(libwaysip::SelectionType::Area)
        .get()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No area selected".to_string())?;
    waysip_to_region(info.size(), info.left_top_point()).map_err(|e| e.to_string())
}

/// Run WaySip point selection and return a 1×1 region. Used for both freeze and live paths.
#[cfg(feature = "color_picker")]
pub fn get_region_point(conn: &libwayshot::WayshotConnection) -> Result<LogicalRegion, String> {
    let info = libwaysip::WaySip::new()
        .with_connection(conn.conn.clone())
        .with_selection_type(libwaysip::SelectionType::Point)
        .get()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Failed to capture the point".to_string())?;
    waysip_to_region(
        libwaysip::Size {
            width: 1,
            height: 1,
        },
        info.left_top_point(),
    )
    .map_err(|e| e.to_string())
}

// ─── Encoding format ──────────────────────────────────────────────────────────

/// Supported image encoding formats.
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EncodingFormat {
    /// JPG/JPEG encoder.
    Jpg,
    /// PNG encoder (default).
    #[default]
    Png,
    /// PPM encoder.
    Ppm,
    /// QOI encoder.
    Qoi,
    /// WebP encoder.
    Webp,
    /// AVIF encoder. Requires the `avif` Cargo feature.
    #[cfg(feature = "avif")]
    Avif,
    /// JPEG-XL encoder. Requires the `jxl` Cargo feature.
    #[cfg(feature = "jxl")]
    Jxl,
}

impl From<EncodingFormat> for image::ImageFormat {
    fn from(format: EncodingFormat) -> Self {
        match format {
            EncodingFormat::Jpg => image::ImageFormat::Jpeg,
            EncodingFormat::Png => image::ImageFormat::Png,
            EncodingFormat::Ppm => image::ImageFormat::Pnm,
            EncodingFormat::Qoi => image::ImageFormat::Qoi,
            EncodingFormat::Webp => image::ImageFormat::WebP,
            #[cfg(feature = "avif")]
            EncodingFormat::Avif => image::ImageFormat::Avif,
            // JXL is handled via encode_image; this fallback should never be reached.
            #[cfg(feature = "jxl")]
            EncodingFormat::Jxl => image::ImageFormat::Png,
        }
    }
}

impl TryFrom<&PathBuf> for EncodingFormat {
    type Error = Error;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        value
            .extension()
            .wrap_err_with(|| {
                format!(
                    "no extension in {} to deduce encoding format",
                    value.display()
                )
            })
            .and_then(|ext| {
                ext.to_str().wrap_err_with(|| {
                    format!("extension in {} is not valid unicode", value.display())
                })
            })
            .and_then(|ext| ext.parse())
    }
}

impl Display for EncodingFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<&str>::into(*self))
    }
}

impl From<EncodingFormat> for &str {
    fn from(format: EncodingFormat) -> Self {
        match format {
            EncodingFormat::Jpg => "jpg",
            EncodingFormat::Png => "png",
            EncodingFormat::Ppm => "ppm",
            EncodingFormat::Qoi => "qoi",
            EncodingFormat::Webp => "webp",
            #[cfg(feature = "avif")]
            EncodingFormat::Avif => "avif",
            #[cfg(feature = "jxl")]
            EncodingFormat::Jxl => "jxl",
        }
    }
}

impl FromStr for EncodingFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "jpg" | "jpeg" => Self::Jpg,
            "png" => Self::Png,
            "ppm" => Self::Ppm,
            "qoi" => Self::Qoi,
            "webp" => Self::Webp,
            #[cfg(feature = "avif")]
            "avif" => Self::Avif,
            #[cfg(feature = "jxl")]
            "jxl" => Self::Jxl,
            _ => bail!("unsupported extension '{s}'"),
        })
    }
}

// ─── Path helpers ─────────────────────────────────────────────────────────────

pub fn get_absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir().unwrap_or_default().join(path)
    }
}

pub fn get_expanded_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    match shellexpand::full(&path_str) {
        Ok(expanded) => PathBuf::from(expanded.into_owned()),
        Err(_) => env::current_dir().unwrap_or_default(),
    }
}

pub fn get_default_file_name(filename_format: &str, encoding: EncodingFormat) -> PathBuf {
    PathBuf::from(format!(
        "{}.{encoding}",
        Local::now().format(filename_format)
    ))
}

pub fn get_full_file_name(path: &Path, filename_format: &str, encoding: EncodingFormat) -> PathBuf {
    let absolute = get_absolute_path(&get_expanded_path(path));
    if absolute.is_dir() {
        absolute.join(get_default_file_name(filename_format, encoding))
    } else {
        let base_dir = absolute
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap_or_default());
        let stem = absolute.file_stem().unwrap_or_default().to_string_lossy();
        base_dir.join(format!("{stem}.{encoding}"))
    }
}

// ─── Image encoding ───────────────────────────────────────────────────────────

/// Encode `image` to raw bytes using the given format and per-format config.
///
/// This is the single encoding path used for file output, stdout, and clipboard.
pub fn encode_image(
    image: &DynamicImage,
    encoding: EncodingFormat,
    jxl: &Jxl,
    png: &Png,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    #[cfg(not(feature = "jxl"))]
    let _ = jxl;

    match encoding {
        #[cfg(feature = "jxl")]
        EncodingFormat::Jxl => encode_to_jxl_bytes(
            image,
            jxl.get_lossless(),
            jxl.get_distance(),
            jxl.get_encoder_speed(),
        ),
        EncodingFormat::Png => encode_to_png_bytes(image, png.get_compression(), png.get_filter()),
        _ => {
            let mut buf = Cursor::new(Vec::new());
            image.write_to(&mut buf, encoding.into())?;
            Ok(buf.into_inner())
        }
    }
}

#[cfg(feature = "jxl")]
fn encode_to_jxl_bytes(
    image: &DynamicImage,
    lossless: bool,
    distance: f32,
    speed: EncoderSpeed,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Alpha channel causes artefacts in jpegxl-rs; screenshots don't need it.
    // See: https://github.com/inflation/jpegxl-rs/issues/96
    let pixels_rgb8 = image.to_rgb8();
    let pixels = pixels_rgb8.as_raw();
    let mut encoder = jpegxl_rs::encoder_builder()
        .lossless(lossless)
        .quality(distance)
        .speed(speed)
        .build()?;
    let EncoderResult { data, .. } =
        encoder.encode::<u8, u8>(pixels, image.width(), image.height())?;
    Ok(data.to_vec())
}

fn encode_to_png_bytes(
    image: &DynamicImage,
    compression: CompressionType,
    filter: FilterType,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = Cursor::new(Vec::new());
    image.write_with_encoder(PngEncoder::new_with_quality(&mut buf, compression, filter))?;
    Ok(buf.into_inner())
}

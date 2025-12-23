use clap::ValueEnum;
use eyre::{ContextCompat, Error, bail};
use notify_rust::Notification;

use image::DynamicImage;
use jpegxl_rs::encode::EncoderResult;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::Local;
use libwayshot::Result;
use libwayshot::region::{LogicalRegion, Position, Region, Size};

pub fn waysip_to_region(
    size: libwaysip::Size,
    position: libwaysip::Position,
) -> Result<LogicalRegion> {
    let size: Size = Size {
        width: size.width.try_into().map_err(|_| {
            libwayshot::Error::FreezeCallbackError("width cannot be negative".to_string())
        })?,
        height: size.height.try_into().map_err(|_| {
            libwayshot::Error::FreezeCallbackError("height cannot be negative".to_string())
        })?,
    };
    let position: Position = Position {
        x: position.x,
        y: position.y,
    };

    Ok(LogicalRegion {
        inner: Region { position, size },
    })
}

/// Supported image encoding formats.
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EncodingFormat {
    /// JPG/JPEG encoder.
    Jpg,
    /// PNG encoder.
    #[default]
    Png,
    /// PPM encoder.
    Ppm,
    /// Qut encoder.
    Qoi,
    /// WebP encoder,
    Webp,
    /// Avif encoder,
    Avif,
    /// JPEG-XL encoder,
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
            EncodingFormat::Avif => image::ImageFormat::Avif,
            // Note: JXL is handled separately via encode_to_jxl_bytes since image-rs doesn't support it yet
            // This fallback is only used if the code path somehow reaches here (shouldn't happen)
            EncodingFormat::Jxl => image::ImageFormat::Png,
        }
    }
}

impl TryFrom<&PathBuf> for EncodingFormat {
    type Error = Error;

    fn try_from(value: &PathBuf) -> std::result::Result<Self, Self::Error> {
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
            EncodingFormat::Avif => "avif",
            EncodingFormat::Jxl => "jxl",
        }
    }
}

impl FromStr for EncodingFormat {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "jpg" | "jpeg" => Self::Jpg,
            "png" => Self::Png,
            "ppm" => Self::Ppm,
            "qoi" => Self::Qoi,
            "webp" => Self::Webp,
            "avif" => Self::Avif,
            "jxl" => Self::Jxl,
            _ => bail!("unsupported extension '{s}'"),
        })
    }
}

pub fn get_absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
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
    let format = Local::now().format(filename_format);

    PathBuf::from(format!("{format}.{encoding}"))
}

pub fn get_full_file_name(path: &Path, filename_format: &str, encoding: EncodingFormat) -> PathBuf {
    let expanded_path = get_expanded_path(path);
    let absolute_path = get_absolute_path(&expanded_path);

    if absolute_path.is_dir() {
        absolute_path.join(get_default_file_name(filename_format, encoding))
    } else {
        let base_dir = absolute_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap_or_default());
        let stem = absolute_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        base_dir.join(format!("{stem}.{encoding}"))
    }
}

pub fn encode_to_jxl_bytes(
    image_buffer: &DynamicImage,
    lossless: bool,
    distance: f32,
    effort: u8,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let width = image_buffer.width();
    let height = image_buffer.height();

    // using buffer with alpha channel results in bad output and we don't need alpha on screenshot anyway
    // see: https://github.com/inflation/jpegxl-rs/issues/96
    let pixels_rgb8 = image_buffer.to_rgb8();
    let pixels = pixels_rgb8.as_raw();

    let mut encoder = jpegxl_rs::encoder_builder()
        .lossless(lossless)
        .quality(distance)
        .speed(effort)
        .build()?;
    let EncoderResult { data, .. } = encoder.encode::<u8, u8>(pixels, width, height)?;

    Ok(data.to_vec())
}

pub fn encode_to_jxl(
    image_buffer: &DynamicImage,
    path: &PathBuf,
    lossless: bool,
    distance: f32,
    effort: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = encode_to_jxl_bytes(image_buffer, lossless, distance, effort)?;
    let mut file = File::create(path)?;
    file.write_all(&data)?;

    Ok(())
}

const TIMEOUT: i32 = 5000;

#[derive(Debug, Clone)]
pub enum ShotResult {
    Output { name: String },
    Toplevel { name: String },
    Area,
    All,
}

pub fn send_notification(shot_result: Result<ShotResult, &Error>) {
    match shot_result {
        Ok(result) => {
            let body = match result {
                ShotResult::Output { name } => {
                    format!("Screenshot of output '{}' saved", name)
                }
                ShotResult::Toplevel { name } => {
                    format!("Screenshot of toplevel '{}' saved", name)
                }
                ShotResult::Area => "Screenshot of selected area saved".to_string(),
                ShotResult::All => "Screenshot of all outputs saved".to_string(),
            };
            let _ = Notification::new()
                .summary("Screenshot Taken")
                .body(&body)
                .timeout(TIMEOUT)
                .show();
        }
        Err(e) => {
            let _ = Notification::new()
                .summary("Screenshot Failed")
                .body(&e.to_string())
                .timeout(TIMEOUT)
                .show();
        }
    }
}

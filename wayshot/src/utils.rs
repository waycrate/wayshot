use clap::ValueEnum;
use eyre::{ContextCompat, Error, bail};

use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::Local;
use libwayshot::Result;
use libwayshot::region::{LogicalRegion, Position, Region, Size};
use libwaysip::get_area;

pub fn waysip_to_region(
    info: libwaysip::state::AreaInfo,
    selection_type: libwaysip::SelectionType,
) -> Result<LogicalRegion> {
    // Macro copied from waysip
    macro_rules! get_info {
        ($x: expr) => {
            match get_area(None, $x) {
                Ok(Some(info)) => info,
                Ok(None) => {
                    eprintln!("Get None, you cancel it");
                    // TODO: Have proper error types
                    return Err(libwayshot::Error::FreezeCallbackError(
                        "Failed to capture the area".to_string(),
                    ));
                }
                Err(e) => {
                    eprintln!("Error,{e}");
                    return Err(libwayshot::Error::FreezeCallbackError(e.to_string()));
                }
            }
        };
    }
    match selection_type {
        libwaysip::SelectionType::Point => {
            let size: Size = Size {
                width: info.width() as u32,
                height: info.height() as u32,
            };

            let position: Position = Position {
                x: info.left_top_point().x,
                y: info.left_top_point().y,
            };

            return Ok(LogicalRegion {
                inner: Region { position, size },
            });
        }
        libwaysip::SelectionType::Area => {
            let size: Size = Size {
                width: info.width() as u32,
                height: info.height() as u32,
            };
            let position: Position = Position {
                x: info.left_top_point().x,
                y: info.left_top_point().y,
            };

            return Ok(LogicalRegion {
                inner: Region { position, size },
            });
        }
        libwaysip::SelectionType::Screen => {
            let screen_info = info.selected_screen_info();
            let position: Position = Position {
                x: screen_info.get_position().x,
                y: screen_info.get_position().x,
            };
            let size: Size = Size {
                width: info.width() as u32,
                height: info.height() as u32,
            };
            return Ok(LogicalRegion {
                inner: Region { position, size },
            });
        }
    };
}

/// Supported image encoding formats.
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncodingFormat {
    /// JPG/JPEG encoder.
    Jpg,
    /// PNG encoder.
    Png,
    /// PPM encoder.
    Ppm,
    /// Qut encoder.
    Qoi,
    /// WebP encoder,
    Webp,
    /// Avif encoder,
    Avif,
}

impl Default for EncodingFormat {
    fn default() -> Self {
        Self::Png
    }
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

    PathBuf::from(format!("{}.{}", format, encoding))
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
        base_dir.join(format!("{}.{}", stem, encoding))
    }
}

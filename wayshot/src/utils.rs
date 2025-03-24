use clap::ValueEnum;
use eyre::{ContextCompat, Error, Result, bail};

use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::Local;
use libwayshot::region::{LogicalRegion, Position, Region, Size};

pub fn parse_geometry(g: &str) -> Result<LogicalRegion> {
    let tail = g.trim();
    let x_coordinate: i32;
    let y_coordinate: i32;
    let width: u32;
    let height: u32;

    let validation_error =
        "Invalid geometry provided.\nValid geometries:\n1) %d,%d %dx%d\n2) %d %d %d %d";

    if tail.contains(',') {
        // this accepts: "%d,%d %dx%d"
        let (head, tail) = tail.split_once(',').wrap_err(validation_error)?;
        x_coordinate = head.parse::<i32>()?;
        let (head, tail) = tail.split_once(' ').wrap_err(validation_error)?;
        y_coordinate = head.parse::<i32>()?;
        let (head, tail) = tail.split_once('x').wrap_err(validation_error)?;
        width = head.parse::<u32>()?;
        height = tail.parse::<u32>()?;
    } else {
        // this accepts: "%d %d %d %d"
        let (head, tail) = tail.split_once(' ').wrap_err(validation_error)?;
        x_coordinate = head.parse::<i32>()?;
        let (head, tail) = tail.split_once(' ').wrap_err(validation_error)?;
        y_coordinate = head.parse::<i32>()?;
        let (head, tail) = tail.split_once(' ').wrap_err(validation_error)?;
        width = head.parse::<u32>()?;
        height = tail.parse::<u32>()?;
    }

    Ok(LogicalRegion {
        inner: Region {
            position: Position {
                x: x_coordinate,
                y: y_coordinate,
            },
            size: Size { width, height },
        },
    })
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

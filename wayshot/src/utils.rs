use clap::ValueEnum;
use eyre::{bail, ContextCompat, Error, Result};

use std::{
    fmt::Display,
    path::PathBuf,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

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
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum EncodingFormat {
    /// JPG/JPEG encoder.
    Jpg,
    /// PNG encoder.
    Png,
    /// PPM encoder.
    Ppm,
    /// Qut encoder.
    Qoi,
}

impl Default for EncodingFormat {
    fn default() -> Self {
        Self::Png
    }
}

impl From<EncodingFormat> for image::ImageOutputFormat {
    fn from(format: EncodingFormat) -> Self {
        match format {
            EncodingFormat::Jpg => image::ImageFormat::Jpeg.into(),
            EncodingFormat::Png => image::ImageFormat::Png.into(),
            EncodingFormat::Ppm => image::ImageFormat::Pnm.into(),
            EncodingFormat::Qoi => image::ImageFormat::Qoi.into(),
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
            _ => bail!("unsupported extension '{s}'"),
        })
    }
}

pub fn get_default_file_name(extension: EncodingFormat) -> PathBuf {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs().to_string())
        .unwrap_or("unknown".into());

    format!("{time}-wayshot.{extension}").into()
}

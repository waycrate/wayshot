use std::{
    process::exit,
    time::{SystemTime, UNIX_EPOCH},
};

use libwayshot::CaptureRegion;

pub fn parse_geometry(g: &str) -> Option<CaptureRegion> {
    let tail = g.trim();
    let x_coordinate: i32;
    let y_coordinate: i32;
    let width: i32;
    let height: i32;

    if tail.contains(',') {
        // this accepts: "%d,%d %dx%d"
        let (head, tail) = tail.split_once(',')?;
        x_coordinate = head.parse::<i32>().ok()?;
        let (head, tail) = tail.split_once(' ')?;
        y_coordinate = head.parse::<i32>().ok()?;
        let (head, tail) = tail.split_once('x')?;
        width = head.parse::<i32>().ok()?;
        height = tail.parse::<i32>().ok()?;
    } else {
        // this accepts: "%d %d %d %d"
        let (head, tail) = tail.split_once(' ')?;
        x_coordinate = head.parse::<i32>().ok()?;
        let (head, tail) = tail.split_once(' ')?;
        y_coordinate = head.parse::<i32>().ok()?;
        let (head, tail) = tail.split_once(' ')?;
        width = head.parse::<i32>().ok()?;
        height = tail.parse::<i32>().ok()?;
    }

    Some(CaptureRegion {
        x_coordinate,
        y_coordinate,
        width,
        height,
    })
}

/// Supported image encoding formats.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EncodingFormat {
    /// Jpeg / jpg encoder.
    Jpg,
    /// Png encoder.
    Png,
    /// Ppm encoder.
    Ppm,
    /// Qoi encoder.
    Qoi,
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

pub fn get_default_file_name(extension: EncodingFormat) -> String {
    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_secs().to_string(),
        Err(_) => {
            tracing::error!("SystemTime before UNIX EPOCH!");
            // didn't think this place needed to exit
            String::from("")
        }
    };

    time + "-wayshot." + extension.into()
}
fn get_hour_minute_from_unix_seconds(seconds: u64) -> String {
    let total_minutes = seconds / 60;

    let mut current_hour = (((total_minutes / 60) % 24) + 5) % 24;

    let mut current_minute = (total_minutes % 60) + 30;
    // println!("{}", current_minute);
    if current_minute > 60 {
        current_hour += 1;
    }
    current_minute = current_minute % 60;
    if current_hour == 24 {
        current_hour = 0;
    }

    // println!("{}", total_minutes as f64 / 60.0);

    format!("{}:{}:{}", current_hour, current_minute, seconds % 60)
}
pub fn get_human_time_file_name(extension: EncodingFormat) -> String {
    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => get_hour_minute_from_unix_seconds(n.as_secs()),
        Err(_) => {
            tracing::error!("SystemTime before UNIX EPOCH!");
            String::from("")
        }
    };

    time + "-wayshot." + extension.into()
}

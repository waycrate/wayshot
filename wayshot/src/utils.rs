use std::{
    process::exit,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn parse_geometry(g: &str) -> Option<libwayshot::CaptureRegion> {
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

    Some(libwayshot::CaptureRegion {
        x_coordinate,
        y_coordinate,
        width,
        height,
    })
}

pub fn get_default_file_name(extension: libwayshot::EncodingFormat) -> String {
    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_secs().to_string(),
        Err(_) => {
            log::error!("SystemTime before UNIX EPOCH!");
            exit(1);
        }
    };

    time + "-wayshot." + extension.into()
}

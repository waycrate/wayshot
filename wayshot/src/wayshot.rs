use std::{
    cmp, env,
    error::Error,
    fs::File,
    io::{stdout, BufWriter, Cursor, Write},
    process::exit,
    time::{SystemTime, UNIX_EPOCH},
};

use libwayshot::CaptureRegion;
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_output::WlOutput, wl_registry},
    Connection, QueueHandle,
};

mod clap;
mod output;

// TODO: Create a xdg-shell surface, check for the enter event, grab the output from it.
//
// TODO: Patch multiple output bug via multiple images composited into 1.

fn parse_geometry(g: &str) -> Option<libwayshot::CaptureRegion> {
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

struct WayshotState {}

impl wayland_client::Dispatch<wl_registry::WlRegistry, GlobalListContents> for WayshotState {
    fn event(
        _: &mut WayshotState,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<WayshotState>,
    ) {
    }
}

struct MutiCaptureMessage {
    output: WlOutput,
    region: CaptureRegion,
}

enum CaptureWay {
    Region(CaptureRegion),
    WayOutput(WlOutput),
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = clap::set_flags().get_matches();
    env::set_var("RUST_LOG", "wayshot=info");

    if args.is_present("debug") {
        env::set_var("RUST_LOG", "wayshot=trace");
    }

    env_logger::init();
    log::trace!("Logger initialized.");

    let mut conn = Connection::connect_to_env().unwrap();
    let (mut globals, _) = registry_queue_init::<WayshotState>(&conn).unwrap();

    let mut cursor_overlay: i32 = 0;
    if args.is_present("cursor") {
        cursor_overlay = 1;
    }

    if args.is_present("listoutputs") {
        let valid_outputs = output::get_all_outputs(&mut globals, &mut conn);
        for output in valid_outputs {
            log::info!("{:#?}", output.name);
        }
        exit(1);
    }

    let capture_area = if args.is_present("slurp") {
        if args.value_of("slurp").unwrap() == "" {
            log::error!("Failed to recieve geometry.");
            exit(1);
        }
        CaptureWay::Region(
            parse_geometry(args.value_of("slurp").unwrap())
                .expect("Invalid geometry specification"),
        )
    } else if !args.is_present("output") {
        let (mut startx, mut starty) = (0, 0);
        let (mut endx, mut endy) = (0, 0);
        let outputinfos = output::get_all_outputs(&mut globals, &mut conn);
        for outputinfo in outputinfos {
            if outputinfo.dimensions.x < startx {
                startx = outputinfo.dimensions.x;
            }
            if outputinfo.dimensions.y < starty {
                starty = outputinfo.dimensions.y;
            }
            if outputinfo.dimensions.x + outputinfo.dimensions.width > endx {
                endx = outputinfo.dimensions.x + outputinfo.dimensions.width;
            }
            if outputinfo.dimensions.y + outputinfo.dimensions.height > endy {
                endy = outputinfo.dimensions.y + outputinfo.dimensions.height;
            }
        }
        CaptureWay::Region(CaptureRegion {
            x_coordinate: startx,
            y_coordinate: starty,
            width: endx - startx,
            height: endy - starty,
        })
    } else {
        CaptureWay::WayOutput(output::get_wloutput(
            args.value_of("output").unwrap().trim().to_string(),
            output::get_all_outputs(&mut globals, &mut conn),
        ))
    };

    let frame_copy: (Vec<libwayshot::FrameCopy>, Option<(i32, i32)>) = match capture_area {
        CaptureWay::Region(region) => {
            let mut framecopys = Vec::new();

            let outputs = output::get_all_outputs(&mut globals, &mut conn);
            let mut intersecting_outputs: Vec<MutiCaptureMessage> = Vec::new();
            for output in outputs.iter() {
                let x1: i32 = cmp::max(output.dimensions.x, region.x_coordinate);
                let y1: i32 = cmp::max(output.dimensions.y, region.y_coordinate);
                let x2: i32 = cmp::min(
                    output.dimensions.x + output.dimensions.width,
                    region.x_coordinate + region.width,
                );
                let y2: i32 = cmp::min(
                    output.dimensions.y + output.dimensions.height,
                    region.y_coordinate + region.height,
                );

                let width = x2 - x1;
                let height = y2 - y1;

                if !(width <= 0 || height <= 0) {
                    let true_x = region.x_coordinate - output.dimensions.x;
                    let true_y = region.y_coordinate - output.dimensions.y;
                    let true_region = CaptureRegion {
                        x_coordinate: true_x,
                        y_coordinate: true_y,
                        width: region.width,
                        height: region.height,
                    };
                    intersecting_outputs.push(MutiCaptureMessage {
                        output: output.wl_output.clone(),
                        region: true_region,
                    });
                }
            }
            if intersecting_outputs.is_empty() {
                log::error!("Provided capture region doesn't intersect with any outputs!");
                exit(1);
            }

            for ouput_info in intersecting_outputs {
                framecopys.push(libwayshot::capture_output_frame(
                    &mut globals,
                    &mut conn,
                    cursor_overlay,
                    ouput_info.output.clone(),
                    Some(ouput_info.region),
                )?);
            }
            (framecopys, Some((region.width, region.height)))
        }
        CaptureWay::WayOutput(output) => (
            vec![libwayshot::capture_output_frame(
                &mut globals,
                &mut conn,
                cursor_overlay,
                output,
                None,
            )?],
            None,
        ),
    };
    let extension = if args.is_present("extension") {
        let ext: &str = &args.value_of("extension").unwrap().trim().to_lowercase();
        match ext {
            "jpeg" | "jpg" => libwayshot::EncodingFormat::Jpg,
            "png" => libwayshot::EncodingFormat::Png,
            "ppm" => libwayshot::EncodingFormat::Ppm,
            _ => {
                log::error!("Invalid extension provided.\nValid extensions:\n1) jpeg\n2) jpg\n3) png\n4) ppm");
                exit(1);
            }
        }
    } else {
        libwayshot::EncodingFormat::Png
    };

    if extension != libwayshot::EncodingFormat::Png {
        log::debug!("Using custom extension: {:#?}", extension);
    }
    if frame_copy.0.len() == 1 {
        let frame_copy = &frame_copy.0[0];
        if args.is_present("stdout") {
            let stdout = stdout();
            let mut writer = BufWriter::new(stdout.lock());
            libwayshot::write_to_file(&mut writer, extension, frame_copy)?;
        } else {
            let path = if args.is_present("file") {
                args.value_of("file").unwrap().trim().to_string()
            } else {
                let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(n) => n.as_secs().to_string(),
                    Err(_) => {
                        log::error!("SystemTime before UNIX EPOCH!");
                        exit(1);
                    }
                };

                time + match extension {
                    libwayshot::EncodingFormat::Png => "-wayshot.png",
                    libwayshot::EncodingFormat::Jpg => "-wayshot.jpg",
                    libwayshot::EncodingFormat::Ppm => "-wayshot.ppm",
                }
            };

            libwayshot::write_to_file(&mut File::create(path)?, extension, frame_copy)?;
        }
    } else {
        let mut images = Vec::new();
        let (frame_copy, region) = frame_copy;
        let (width, height) = region.unwrap();
        for frame_copy in frame_copy {
            let mut buff = Cursor::new(Vec::new());
            libwayshot::write_to_file(&mut buff, extension, &frame_copy)?;
            let image = image::load_from_memory(buff.get_ref())?;
            let image = image::imageops::resize(
                &image,
                width as u32,
                height as u32,
                image::imageops::FilterType::Gaussian,
            );
            images.push(image);
        }
        use image::imageops::overlay;
        let mut image_bottom = images[0].clone();
        for image in images {
            overlay(&mut image_bottom, &image, 0, 0);
        }
        if args.is_present("stdout") {
            let stdout = stdout();
            let mut buff = Cursor::new(Vec::new());

            let mut writer = BufWriter::new(stdout.lock());
            image_bottom.write_to(&mut buff, extension)?;
            writer.write_all(buff.get_ref())?;
        } else {
            let path = if args.is_present("file") {
                args.value_of("file").unwrap().trim().to_string()
            } else {
                let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(n) => n.as_secs().to_string(),
                    Err(_) => {
                        log::error!("SystemTime before UNIX EPOCH!");
                        exit(1);
                    }
                };

                time + match extension {
                    libwayshot::EncodingFormat::Png => "-wayshot.png",
                    libwayshot::EncodingFormat::Jpg => "-wayshot.jpg",
                    libwayshot::EncodingFormat::Ppm => "-wayshot.ppm",
                }
            };

            image_bottom.save(path)?;
        }
    }
    Ok(())
}

use std::{
    cmp::{max, min},
    env,
    error::Error,
    fs::File,
    io::{stdout, BufWriter},
    process::exit,
    time::{SystemTime, UNIX_EPOCH},
};

use smithay_client_toolkit::reexports::client::{protocol::wl_output::WlOutput, Display};

use image::{imageops, ImageBuffer, Rgba};

mod backend;
mod clap;
mod output;

// TODO: Patch multiple output bug via multiple images composited into 1.

fn main() -> Result<(), Box<dyn Error>> {
    let args = clap::set_flags().get_matches();
    env::set_var("RUST_LOG", "wayshot=info");

    if args.is_present("debug") {
        env::set_var("RUST_LOG", "wayshot=trace");
    }

    env_logger::init();
    log::trace!("Logger initialized.");

    let display = Display::connect_to_env()?;
    let mut extension = backend::EncodingFormat::Png;
    let output: WlOutput;

    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(time) => time.as_secs().to_string(),
        Err(e) => {
            log::error!("Err: {:#?}", e);
            exit(1);
        }
    };
    let mut path = time
        + match extension {
            backend::EncodingFormat::Png => "-wayshot.png",
            backend::EncodingFormat::Jpg => "-wayshot.jpg",
        };

    let mut cursor_overlay: i32 = 0;
    if args.is_present("cursor") {
        cursor_overlay = 1;
    }

    if args.is_present("listoutputs") {
        let valid_outputs = output::get_valid_outputs(display);
        for output in valid_outputs {
            let (_, info) = output;
            log::info!("{:#?}", info.name);
        }
        exit(1);
    }

    if args.is_present("output") {
        output = output::get_wloutput(
            args.value_of("output").unwrap().trim().to_string(),
            output::get_valid_outputs(display.clone()),
        )
    } else {
        (output, _) = output::get_valid_outputs(display.clone())
            .first()
            .unwrap()
            .clone();
    }
    if args.is_present("extension") {
        let ext = args.value_of("extension").unwrap().trim();
        match ext {
            "jpeg" | "jpg" => {
                extension = backend::EncodingFormat::Jpg;
            }
            "png" => {}
            _ => {
                log::error!(
                    "Invalid extension provided.\nValid extensions:\n1) jpeg\n2) jpg\n3) png"
                );
                exit(1);
            }
        }
    }

    let frame_copy: backend::FrameCopy = if args.is_present("slurp") {
        if args.value_of("slurp").unwrap() == "" {
            log::error!("Failed to recieve geometry.");
            exit(1);
        }
        // Cleaning, parsing and gathering the coordinate data.
        let slurp: Vec<_> = args
            .value_of("slurp")
            .unwrap()
            .trim()
            .split(' ')
            .collect::<Vec<_>>()
            .iter()
            .map(|i| i.parse::<i32>().unwrap())
            .collect();

        let intersecting_output_boxes = parse_intersecting_region(
            display.clone(),
            backend::CaptureRegion {
                x_coordinate: slurp[0],
                y_coordinate: slurp[1],
                width: slurp[2],
                height: slurp[3],
            },
        );

        let mut frames: Vec<(backend::FrameCopy, (i32, i32))> = Vec::new();
        for (capture_region, output, offset) in intersecting_output_boxes {
            frames.push((
                backend::capture_output_frame(
                    display.clone(),
                    cursor_overlay,
                    output,
                    Some(capture_region),
                )?,
                offset,
            ));
        }

        let mut image_buffer: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::<Rgba<u8>, _>::new(slurp[2] as u32, slurp[3] as u32);
        let mut image_buffers: Vec<(ImageBuffer<Rgba<u8>, _>, (i32, i32))> = Vec::new();

        // Iterating over each frame and making an image_buffer out of each image.
        for (frame, offset) in frames {
            image_buffers.push((
                ImageBuffer::<Rgba<u8>, _>::from_raw(
                    frame.frame_format.width,
                    frame.frame_format.height,
                    frame.frame_mmap,
                )
                .unwrap(),
                offset,
            ));
        }

        // Iterating over each buffer and blitting them as needed.
        for (buffer, offset) in image_buffers {
            imageops::replace(&mut image_buffer, &buffer, offset.0.into(), offset.1.into());
        }

        if !(args.is_present("stdout")) {
            image_buffer.save_with_format(
                path,
                match extension {
                    backend::EncodingFormat::Png => image::ImageFormat::Png,
                    backend::EncodingFormat::Jpg => image::ImageFormat::Jpeg,
                },
            )?;
        }
        exit(1);
    } else {
        backend::capture_output_frame(display, cursor_overlay, output, None)?
    };

    if args.is_present("stdout") {
        let stdout = stdout();
        let writer = BufWriter::new(stdout.lock());
        backend::write_to_file(writer, extension, frame_copy)?;
    } else {
        if args.is_present("file") {
            path = args.value_of("file").unwrap().trim().to_string();
        }
        backend::write_to_file(File::create(path)?, extension, frame_copy)?;
    }
    Ok(())
}

/// Get the intersecting regions of the box with respect to the valid outputs.
fn parse_intersecting_region(
    display: Display,
    capture_region: backend::CaptureRegion,
) -> Vec<(backend::CaptureRegion, WlOutput, (i32, i32))> {
    let valid_outputs = output::get_valid_outputs(display);
    let mut intersecting_outputs: Vec<(backend::CaptureRegion, WlOutput, (i32, i32))> = Vec::new();

    // Check for empty boxes.
    if capture_region.width <= 0 || capture_region.height <= 0 {
        log::error!("Capture region from slurp flag did not return a whole number.");
        exit(1);
    }

    for (valid_output, output_info) in valid_outputs {
        for mode in output_info.modes {
            if mode.is_current {
                let x1 = max(capture_region.x_coordinate, output_info.location.0);
                let y1 = max(capture_region.y_coordinate, output_info.location.1);
                let x2 = min(
                    capture_region.x_coordinate + capture_region.width,
                    output_info.location.0 + mode.dimensions.0,
                );
                let y2 = min(
                    capture_region.y_coordinate + capture_region.height,
                    output_info.location.1 + mode.dimensions.1,
                );
                intersecting_outputs.push((
                    backend::CaptureRegion {
                        x_coordinate: x1,
                        y_coordinate: y1,
                        width: x2 - x1,
                        height: y2 - y1,
                    },
                    valid_output.clone(),
                    output_info.location,
                ));
            }
        }
    }
    intersecting_outputs
}

use std::{
    cmp, env,
    error::Error,
    fs::File,
    io::{stdout, BufWriter, Write},
    process::exit,
    time::{SystemTime, UNIX_EPOCH},
};

use image::{ImageBuffer, Rgba};
use wayland_client::{protocol::wl_output::WlOutput, Display};

mod backend;
mod clap;
mod output;

// TODO: Create a xdg-shell surface, check for the enter event, grab the output from it.
//
// TODO: Patch multiple output bug via multiple images composited into 1.
// TODO: Handle flipped monitors.

fn main() -> Result<(), Box<dyn Error>> {
    // Setting up clap.
    let args = clap::set_flags().get_matches();
    env::set_var("RUST_LOG", "wayshot=info");

    // Setting debug logs.
    if args.is_present("debug") {
        env::set_var("RUST_LOG", "wayshot=trace");
    }

    env_logger::init();
    log::trace!("Logger initialized.");

    // Connecting to wayland display.
    let display = Display::connect_to_env()?;

    // Determining encoding format.
    let extension: backend::EncodingFormat = if args.is_present("extension") {
        let ext = args.value_of("extension").unwrap().trim();
        match ext {
            "jpeg" | "jpg" => {
                log::debug!(
                    "Using custom extension: {:#?}",
                    backend::EncodingFormat::Jpg
                );
                backend::EncodingFormat::Jpg
            }
            "png" => backend::EncodingFormat::Png,
            _ => unreachable!(),
        }
    } else {
        backend::EncodingFormat::Png
    };

    // Determining the file which we're supposed to write to.
    // Hence the Box<dyn Write> which basically stores a pointer
    // to a file in memory that implements the write trait.
    let file: Box<dyn Write> = if args.is_present("stdout") {
        // Stdout takes priority.
        Box::new(BufWriter::new(stdout().lock()))
    } else if args.is_present("file") {
        // If a file name is given, then use it.
        Box::new(File::create(args.value_of("file").unwrap().trim())?)
    } else {
        // Else, determine your own file name.
        let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs().to_string(),
            Err(_) => {
                log::error!("SystemTime before UNIX EPOCH!");
                exit(1);
            }
        };

        Box::new(File::create(
            time + match extension {
                backend::EncodingFormat::Png => "-wayshot.png",
                backend::EncodingFormat::Jpg => "-wayshot.jpg",
            },
        )?)
    };

    let mut cursor_overlay: i32 = 0;
    if args.is_present("cursor") {
        cursor_overlay = 1;
    }

    if args.is_present("listoutputs") {
        let valid_outputs = output::get_all_outputs(display);
        for output in valid_outputs {
            log::info!("{:#?}", output.name);
        }
        exit(1);
    }

    let output: WlOutput = if args.is_present("output") {
        output::get_wloutput(
            args.value_of("output").unwrap().trim().to_string(),
            output::get_all_outputs(display.clone()),
        )
    } else {
        output::get_all_outputs(display.clone())
            .first()
            .unwrap()
            .wl_output
            .clone()
    };

    let frame_copy: backend::FrameCopy = if args.is_present("slurp") {
        if args.value_of("slurp").unwrap() == "" {
            log::error!("Failed to recieve geometry.");
            exit(1);
        }
        let slurp: Vec<_> = args.value_of("slurp").unwrap().trim().split(' ').collect();
        let slurp: Vec<i32> = slurp.iter().map(|i| i.parse::<i32>().unwrap()).collect();
        let x_coordinate = slurp[0];
        let y_coordinate = slurp[1];
        let width = slurp[2];
        let height = slurp[3];

        let outputs = output::get_all_outputs(display.clone());
        let mut intersecting_outputs: Vec<(i32, i32, i32, i32, output::OutputInfo)> = Vec::new();

        for output in outputs {
            let x1: i32 = cmp::max(output.dimensions.x, x_coordinate);
            let y1: i32 = cmp::max(output.dimensions.y, y_coordinate);
            let x2: i32 = cmp::min(
                output.dimensions.x + output.dimensions.width,
                x_coordinate + width,
            );
            let y2: i32 = cmp::min(
                output.dimensions.y + output.dimensions.height,
                y_coordinate + height,
            );

            let width = x2 - x1;
            let height = y2 - y1;

            let y_offset = y2 - (y1 + height);
            let x_offset = x2 - (y2 + width);

            if !(width <= 0 || height <= 0) {
                intersecting_outputs.push((x1, y1, width, height, output));
            }
        }

        if intersecting_outputs.is_empty() {
            log::error!("Provided capture region doesn't intersect with any outputs!");
            exit(1);
        } else {
            log::debug!(
                "Region intersects with the following outputs: {:#?}",
                intersecting_outputs
            );
        }

        let mut frames: Vec<(i32, i32, backend::FrameCopy)> = Vec::new();
        for (x, y, width, height, output) in intersecting_outputs {
            frames.push((
                width,
                height,
                backend::capture_output_frame(
                    display.clone(),
                    cursor_overlay,
                    output.wl_output,
                    Some(backend::CaptureRegion {
                        x_coordinate: x,
                        y_coordinate: y,
                        width,
                        height,
                    }),
                )?,
            ));
        }

        let mut image_buffers: Vec<ImageBuffer<Rgba<u8>, _>> = Vec::new();
        for (width, height, frame) in frames {
            image_buffers.push(
                ImageBuffer::<Rgba<u8>, _>::from_raw(
                    width.try_into().unwrap(),
                    height.try_into().unwrap(),
                    frame.frame_mmap,
                )
                .unwrap(),
            );
        }

        let mut composited_frame = image_buffers.pop().unwrap();
        for buffer in image_buffers {
            image::imageops::overlay(&mut composited_frame, &buffer, 0, 0);
        }
        composited_frame.save_with_format(
            "frame.png",
            match extension {
                backend::EncodingFormat::Png => image::ImageFormat::Png,
                backend::EncodingFormat::Jpg => image::ImageFormat::Jpeg,
            },
        )?;

        exit(1);
    } else {
        backend::capture_output_frame(display, cursor_overlay, output, None)?
    };

    backend::write_to_file(file, extension, frame_copy)?;
    Ok(())
}

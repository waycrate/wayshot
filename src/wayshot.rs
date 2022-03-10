use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
    ImageEncoder,
};
use std::{
    error::Error,
    fs::File,
    io::{stdout, BufWriter, Write},
    process::exit,
    time::{SystemTime, UNIX_EPOCH},
};

use smithay_client_toolkit::reexports::client::Display;

mod backend;
mod clap;
mod output;

// TODO: Cleanup this file.
// TODO: Make wayshot.rs feature complete.
// TODO: Patch multiple output bug via multiple images composited into 1.
//
// NOTE: This file is just a temporary one for testing the functions in backend.rs
fn main() -> Result<(), Box<dyn Error>> {
    let display = Display::connect_to_env()?; // Connect to wayland environment.
    let (output, _) = output::get_valid_outputs(display.clone()) // Get the first output we can find.
        .first() // This is just for testing reasons.
        .unwrap()
        .clone();
    let args = clap::set_flags().get_matches(); // Getting all args from clap
    let frame_copy: backend::FrameCopy;

    if args.is_present("slurp") {
        if args.value_of("slurp").unwrap() == "" {
            log::error!("Failed to recieve geometry.");
            exit(1);
        }
        let slurp: Vec<_> = args.value_of("slurp").unwrap().trim().split(" ").collect();
        let slurp: Vec<i32> = slurp.iter().map(|i| i.parse::<i32>().unwrap()).collect();
        let x_coordinate = slurp[0];
        let y_coordinate = slurp[1];
        let width = slurp[2];
        let height = slurp[3];
        frame_copy = backend::grab_head_frame(
            display.clone(),
            1,
            output,
            Some(backend::CaptureRegion {
                x_coordinate,
                y_coordinate,
                width,
                height,
            }),
        )?;
    } else {
        frame_copy = backend::grab_head_frame(display.clone(), 1, output, None)?;
    }

    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_secs().to_string(),
        Err(_) => {
            log::error!("SystemTime before UNIX EPOCH!");
            exit(1);
        }
    };

    let mut extension = "png"; // default extension and encoder

    if args.is_present("extension") {
        let ext = args.value_of("extension").unwrap().trim();
        if ext == "jpeg" || ext == "jpg" || ext == "png" {
            extension = ext;
            log::debug!("Using custom extension: {}", extension);
        } else {
            log::error!("Invalid extension provided.\nValid extensions:\n1) jpeg\n2) jpg\n3) png");
            exit(1);
        }
    }

    let mut path: &str = &(time + "-wayshot." + extension);

    if args.is_present("file") {
        path = args.value_of("file").unwrap().trim()
    }

    match extension {
        "jpeg" | "jpg" => {
            if args.is_present("stdout") {
                let stdout = stdout();
                let guard = stdout.lock();
                let mut writer = BufWriter::new(guard);
                JpegEncoder::new(&mut writer).write_image(
                    &frame_copy.frame_mmap,
                    frame_copy.frame_format.width,
                    frame_copy.frame_format.height,
                    frame_copy.frame_color_type,
                )?;
                writer.flush()?;
            } else {
                JpegEncoder::new(File::create(path)?).write_image(
                    &frame_copy.frame_mmap,
                    frame_copy.frame_format.width,
                    frame_copy.frame_format.height,
                    frame_copy.frame_color_type,
                )?;
            }
        }

        "png" => {
            if args.is_present("stdout") {
                let stdout = stdout();
                let guard = stdout.lock();
                let mut writer = BufWriter::new(guard);
                PngEncoder::new(&mut writer).write_image(
                    &frame_copy.frame_mmap,
                    frame_copy.frame_format.width,
                    frame_copy.frame_format.height,
                    frame_copy.frame_color_type,
                )?;
                writer.flush()?;
            } else {
                PngEncoder::new(File::create(path)?).write_image(
                    &frame_copy.frame_mmap,
                    frame_copy.frame_format.width,
                    frame_copy.frame_format.height,
                    frame_copy.frame_color_type,
                )?;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

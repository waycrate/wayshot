use std::{
    env,
    error::Error,
    fs::File,
    io::{stdout, BufWriter},
    process::exit,
    time::{SystemTime, UNIX_EPOCH},
};

use smithay_client_toolkit::{
    output::OutputInfo,
    reexports::client::{protocol::wl_output::WlOutput, Display},
};

use image::ImageBuffer;

mod backend;
mod clap;
mod output;

// TODO: Create a transparent layer_shell in the background, pass None as WlOutput as that makes
// compositors choose the currently focused monitor. Once done check the enter event for the
// WlOutput object and then feed that to the screencopy capture output function.
//
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
    let frame_copy: backend::FrameCopy;
    let output: WlOutput;

    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(time) => time.as_nanos().to_string(),
        Err(e) => {
            log::error!("Err: {:#?}", e);
            exit(1);
        }
    };
    let mut path = (time
        + match extension {
            backend::EncodingFormat::Png => "-wayshot.png",
            backend::EncodingFormat::Jpg => "-wayshot.jpg",
        })
    .to_string();

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
        output = get_wloutput(
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

        let valid_outputs = output::get_valid_outputs(display.clone());
        let mut frames: Vec<backend::FrameCopy> = Vec::new();
        for (output, _) in valid_outputs {
            frames.push(backend::capture_output_frame(
                display.clone(),
                cursor_overlay,
                output,
                Some(backend::CaptureRegion {
                    x_coordinate,
                    y_coordinate,
                    width,
                    height,
                }),
            )?);
        }
        // TODO: This line. The algorithm for area detection is in another branch.
        for frame in frames {
            ImageBuffer::from_raw(
                frame.frame_format.width,
                frame.frame_format.height,
                &frame.frame_mmap,
            );
        }
        exit(1);
    } else {
        frame_copy = backend::capture_output_frame(display.clone(), cursor_overlay, output, None)?;
    }

    if args.is_present("stdout") {
        let stdout = stdout();
        let writer = BufWriter::new(stdout.lock());
        backend::write_to_file(writer, extension, frame_copy)?;
    } else {
        if args.is_present("file") {
            path = args.value_of("file").unwrap().trim().to_string();
        } else {
        }

        backend::write_to_file(File::create(path)?, extension, frame_copy)?;
    }
    Ok(())
}

/// Get a wl_output object from the output name.
fn get_wloutput(name: String, valid_outputs: Vec<(WlOutput, OutputInfo)>) -> WlOutput {
    for device in valid_outputs.clone() {
        let (output_device, info) = device;
        if info.name == name {
            return output_device;
        }
    }
    println!("Error: No output of name \"{}\" was found", name);
    exit(1);
}

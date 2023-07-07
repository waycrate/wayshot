use std::{
    cmp, env,
    error::Error,
    io::{stdout, BufWriter, Cursor, Write},
    process::exit,
};

use image::imageops::overlay;
use libwayshot::CaptureRegion;
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_output::{self, WlOutput},
        wl_registry,
    },
    Connection, QueueHandle,
};

mod clap;
mod image_util;
mod output;
mod utils;

// TODO: Create a xdg-shell surface, check for the enter event, grab the output from it.

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

struct IntersectingOutput {
    output: WlOutput,
    region: CaptureRegion,
    transform: wl_output::Transform,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = clap::set_flags().get_matches();
    env::set_var("RUST_LOG", "wayshot=info");

    if args.get_flag("debug") {
        env::set_var("RUST_LOG", "wayshot=trace");
    }

    env_logger::init();
    log::trace!("Logger initialized.");

    let extension = if let Some(extension) = args.get_one::<String>("extension") {
        let ext = extension.trim().to_lowercase();
        log::debug!("Using custom extension: {:#?}", ext);

        match ext.as_str() {
            "jpeg" | "jpg" => libwayshot::EncodingFormat::Jpg,
            "png" => libwayshot::EncodingFormat::Png,
            "ppm" => libwayshot::EncodingFormat::Ppm,
            "qoi" => libwayshot::EncodingFormat::Qoi,
            _ => {
                log::error!("Invalid extension provided.\nValid extensions:\n1) jpeg\n2) jpg\n3) png\n4) ppm\n5) qoi");
                exit(1);
            }
        }
    } else {
        libwayshot::EncodingFormat::Png
    };

    let mut file_is_stdout: bool = false;
    let mut file_path: Option<String> = None;

    if args.get_flag("stdout") {
        file_is_stdout = true;
    } else if let Some(filepath) = args.get_one::<String>("file") {
        file_path = Some(filepath.trim().to_string());
    } else {
        file_path = Some(utils::get_default_file_name(extension));
    }

    let mut conn = Connection::connect_to_env().unwrap();
    let (mut globals, _) = registry_queue_init::<WayshotState>(&conn).unwrap();

    if args.get_flag("listoutputs") {
        let valid_outputs = output::get_all_outputs(&mut globals, &mut conn);
        for output in valid_outputs {
            log::info!("{:#?}", output.name);
        }
        exit(1);
    }

    let mut cursor_overlay: i32 = 0;
    if args.get_flag("cursor") {
        cursor_overlay = 1;
    }

    let capture_area = if let Some(slurpregion) = args.get_one::<String>("slurp") {
        match utils::parse_geometry(slurpregion) {
            Some(region) => (wl_output::Transform::Normal, region),
            None => {
                log::error!("Invalid geometry specification");
                exit(1);
            }
        }
    } else if let Some(output_name) = args.get_one::<String>("output") {
        let outputs = output::get_all_outputs(&mut globals, &mut conn);
        let mut capture_info = None;
        for output in outputs {
            if &output.name == output_name {
                capture_info = Some((
                    output.transform,
                    CaptureRegion {
                        x_coordinate: output.dimensions.x,
                        y_coordinate: output.dimensions.y,
                        width: output.dimensions.width,
                        height: output.dimensions.height,
                    },
                ))
            }
        }

        if capture_info.is_none() {
            log::error!("No output found!\n");
            exit(1);
        }

        capture_info.unwrap()
    } else {
        let output = &output::get_all_outputs(&mut globals, &mut conn)[0];
        (
            output.transform,
            CaptureRegion {
                x_coordinate: output.dimensions.x,
                y_coordinate: output.dimensions.y,
                width: output.dimensions.width,
                height: output.dimensions.height,
            },
        )
    };

    let frame_copy: (Vec<libwayshot::FrameCopy>, Option<(i32, i32)>) = {
        let transform = capture_area.0;
        let region = capture_area.1;

        let mut framecopys: Vec<libwayshot::FrameCopy> = Vec::new();

        let outputs = output::get_all_outputs(&mut globals, &mut conn);
        let mut intersecting_outputs: Vec<IntersectingOutput> = Vec::new();
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
                intersecting_outputs.push(IntersectingOutput {
                    output: output.wl_output.clone(),
                    region: true_region,
                    transform,
                });
            }
        }
        if intersecting_outputs.is_empty() {
            log::error!("Provided capture region doesn't intersect with any outputs!");
            exit(1);
        }

        for intersecting_output in intersecting_outputs {
            framecopys.push(libwayshot::capture_output_frame(
                &mut globals,
                &mut conn,
                cursor_overlay,
                intersecting_output.output.clone(),
                intersecting_output.transform,
                Some(intersecting_output.region),
            )?);
        }
        (framecopys, Some((region.width, region.height)))
    };

    let mut composited_image;
    let mut buffer;

    if frame_copy.0.len() == 1 {
        let (width, height) = frame_copy.1.unwrap();
        let frame_copy = &frame_copy.0[0];

        buffer = Cursor::new(Vec::new());
        libwayshot::write_to_file(&mut buffer, extension, frame_copy)?;

        let image = image::load_from_memory(buffer.get_ref())?;
        composited_image = image_util::rotate_image_buffer(
            &image,
            frame_copy.transform,
            width as u32,
            height as u32,
        );
    } else {
        let mut images = Vec::new();
        let (frame_copy, region) = frame_copy;
        let (width, height) = region.unwrap();
        for frame_copy in frame_copy {
            buffer = Cursor::new(Vec::new());
            libwayshot::write_to_file(&mut buffer, extension, &frame_copy)?;
            let image = image::load_from_memory(buffer.get_ref())?;
            let image = image_util::rotate_image_buffer(
                &image,
                frame_copy.transform,
                width as u32,
                height as u32,
            );
            images.push(image);
        }
        composited_image = images[0].clone();
        for image in images {
            overlay(&mut composited_image, &image, 0, 0);
        }
    }

    if file_is_stdout {
        let stdout = stdout();
        let mut buffer = Cursor::new(Vec::new());

        let mut writer = BufWriter::new(stdout.lock());
        composited_image.write_to(&mut buffer, extension)?;

        writer.write_all(buffer.get_ref())?;
    } else {
        composited_image.save(file_path.unwrap())?;
    }

    Ok(())
}

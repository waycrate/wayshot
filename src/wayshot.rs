use std::{
    cell::RefCell,
    env,
    error::Error,
    fs::File,
    io::{stdout, BufWriter, Write},
    os::unix::prelude::FromRawFd,
    process::exit,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};

use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
    ColorType::Rgba8,
    ImageEncoder,
};
use memmap2::MmapMut;

use smithay_client_toolkit::{
    output::OutputInfo,
    reexports::{
        client::{
            protocol::{wl_output::WlOutput, wl_shm, wl_shm::Format},
            Display, GlobalManager, Main,
        },
        protocols::wlr::unstable::screencopy::v1::client::{
            zwlr_screencopy_frame_v1, zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
            zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
        },
    },
};

mod clap;
mod output;
mod shm;

#[derive(Debug, Copy, Clone)]
struct FrameFormat {
    format: Format,
    width: u32,
    height: u32,
    stride: u32,
}

#[derive(Debug, Copy, Clone)]
enum FrameState {
    Failed,
    Finished,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = clap::set_flags().get_matches();
    env::set_var("RUST_LOG", "wayshot=info");

    if args.is_present("debug") {
        env::set_var("RUST_LOG", "wayshot=trace");
    }

    env_logger::init();
    log::trace!("Logger initialized.");

    let display = Display::connect_to_env()?;
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let globals = GlobalManager::new(&attached_display);
    event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!())?;

    let valid_outputs = output::get_valid_outputs(display.clone());
    let (mut output, _): (WlOutput, OutputInfo) = valid_outputs.first().unwrap().clone();

    let frame_formats: Rc<RefCell<Vec<FrameFormat>>> = Rc::new(RefCell::new(Vec::new()));
    let frame_state: Rc<RefCell<Option<FrameState>>> = Rc::new(RefCell::new(None));
    let frame_buffer_done = Rc::new(AtomicBool::new(false));

    let screencopy_manager = globals.instantiate_exact::<ZwlrScreencopyManagerV1>(3)?;

    let frame: Main<ZwlrScreencopyFrameV1>;
    let mut cursor_overlay = 0;
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
        let mut is_present = false;
        let valid_outputs = output::get_valid_outputs(display);

        for device in valid_outputs {
            let (output_device, info) = device;
            if info.name == args.value_of("output").unwrap().trim() {
                is_present = true;
                output = output_device.clone();
            }
        }
        if !is_present {
            log::error!(
                "\"{}\" is not a valid output.",
                args.value_of("output").unwrap().trim()
            );
            exit(1);
        }
    }

    if args.is_present("slurp") {
        if args.value_of("slurp").unwrap() == "" {
            log::error!("Failed to recieve geometry.");
            exit(1);
        }
        let slurp: Vec<_> = args.value_of("slurp").unwrap().trim().split(" ").collect();
        let slurp: Vec<i32> = slurp.iter().map(|i| i.parse::<i32>().unwrap()).collect();
        frame = screencopy_manager.capture_output_region(
            cursor_overlay,
            &output,
            slurp[0],
            slurp[1],
            slurp[2],
            slurp[3],
        );
    } else {
        frame = screencopy_manager.capture_output(cursor_overlay, &output);
    }

    frame.quick_assign({
        let frame_formats = frame_formats.clone();
        let frame_state = frame_state.clone();
        let frame_buffer_done = frame_buffer_done.clone();
        move |_frame, event, _| {
            match event {
                zwlr_screencopy_frame_v1::Event::Buffer {
                    format,
                    width,
                    height,
                    stride,
                } => {
                    log::debug!("Received buffer event");
                    frame_formats.borrow_mut().push(FrameFormat {
                        format,
                        width,
                        height,
                        stride,
                    });
                }
                zwlr_screencopy_frame_v1::Event::Flags { .. } => {
                    log::debug!("Received flags event");
                }
                zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                    log::debug!("Received ready event");
                    frame_state.borrow_mut().replace(FrameState::Finished);
                }
                zwlr_screencopy_frame_v1::Event::Failed => {
                    log::debug!("Received failed event");
                    frame_state.borrow_mut().replace(FrameState::Failed);
                }
                zwlr_screencopy_frame_v1::Event::Damage { .. } => {
                    log::debug!("Received Damaga event");
                }
                zwlr_screencopy_frame_v1::Event::LinuxDmabuf { .. } => {
                    log::debug!("Received LinuxDmaBuf event");
                }
                zwlr_screencopy_frame_v1::Event::BufferDone => {
                    log::debug!("Received bufferdone event");
                    frame_buffer_done.store(true, Ordering::SeqCst);
                }
                _ => unreachable!(),
            };
        }
    });

    while !frame_buffer_done.load(Ordering::SeqCst) {
        event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!())?;
    }

    log::debug!(
        "Received compositor frame buffer formats: {:#?}",
        frame_formats
    );

    let frame_format = frame_formats
        .borrow()
        .iter()
        .filter(|f| {
            matches!(
                f.format,
                wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 | wl_shm::Format::Xbgr8888
            )
        })
        .nth(0)
        .copied();

    log::debug!("Selected frame buffer format: {:#?}", frame_format);

    let frame_format = match frame_format {
        Some(format) => format,
        None => {
            log::error!("No suitable frame format found");
            exit(1);
        }
    };

    let frame_bytes = frame_format.stride * frame_format.height;

    let mem_fd = shm::create_shm_fd()?;
    let mem_file = unsafe { File::from_raw_fd(mem_fd) };
    mem_file.set_len(frame_bytes as u64)?;

    let shm = globals.instantiate_exact::<wl_shm::WlShm>(1)?;
    let pool = shm.create_pool(mem_fd, frame_bytes as i32);
    let buffer = pool.create_buffer(
        0,
        frame_format.width as i32,
        frame_format.height as i32,
        frame_format.stride as i32,
        frame_format.format,
    );

    frame.copy(&buffer);

    loop {
        event_queue.sync_roundtrip(&mut (), |_, _, _| {})?;

        if let Some(state) = frame_state.borrow_mut().take() {
            match state {
                FrameState::Failed => {
                    log::error!("Frame copy failed");
                    break;
                }
                FrameState::Finished => {
                    let mut mmap = unsafe { MmapMut::map_mut(&mem_file)? };
                    let data = &mut *mmap;
                    let color_type = match frame_format.format {
                        wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 => {
                            for chunk in data.chunks_exact_mut(4) {
                                // swap in place (b with r)
                                let tmp = chunk[0];
                                chunk[0] = chunk[2];
                                chunk[2] = tmp;
                            }
                            Rgba8
                        }
                        wl_shm::Format::Xbgr8888 => Rgba8,
                        other => {
                            log::error!("Unsupported buffer format: {:?}", other);
                            log::error!("You can send a feature request for the above format to the mailing list for wayshot over at https://sr.ht/~shinyzenith/wayshot.");
                            break;
                        }
                    };

                    let time = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                        // getting current time for the file name
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
                                    &mmap,
                                    frame_format.width,
                                    frame_format.height,
                                    color_type,
                                )?;
                                writer.flush()?;
                            }
                            JpegEncoder::new(File::create(path)?).write_image(
                                &mmap,
                                frame_format.width,
                                frame_format.height,
                                color_type,
                            )?;
                        }

                        "png" => {
                            if args.is_present("stdout") {
                                let stdout = stdout();
                                let guard = stdout.lock();
                                let mut writer = BufWriter::new(guard);
                                PngEncoder::new(&mut writer).write_image(
                                    &mmap,
                                    frame_format.width,
                                    frame_format.height,
                                    color_type,
                                )?;
                                writer.flush()?;
                            }
                            PngEncoder::new(File::create(path)?).write_image(
                                &mmap,
                                frame_format.width,
                                frame_format.height,
                                color_type,
                            )?;
                        }
                        _ => unreachable!(),
                    }

                    break;
                }
            }
        }
    }
    Ok(())
}

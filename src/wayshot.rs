use std::{
    cell::RefCell,
    ffi::CStr,
    fs::File,
    io::Write,
    os::unix::prelude::{FromRawFd, RawFd},
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{arg, App};

use anyhow::{bail, Result};
use image::{codecs::png::PngEncoder, ImageEncoder};
use memmap2::MmapMut;
use nix::{
    errno::Errno,
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};
use tracing::debug;

use smithay_client_toolkit::{
    environment,
    environment::Environment,
    output::{with_output_info, OutputHandler, OutputInfo, XdgOutputHandler},
    reexports::{
        client::{
            protocol::{wl_output::WlOutput, wl_shm, wl_shm::Format},
            Display, GlobalManager, Main,
        },
        protocols::{
            unstable::xdg_output::v1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1,
            wlr::unstable::screencopy::v1::client::{
                zwlr_screencopy_frame_v1, zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
                zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
            },
        },
    },
};

struct ValidOutputs {
    outputs: OutputHandler,
    xdg_output: XdgOutputHandler,
}

environment! {ValidOutputs,
    singles = [
        ZxdgOutputManagerV1 => xdg_output,
    ],
    multis = [
        WlOutput => outputs,
    ]
}

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

fn main() -> Result<()> {
    let args = set_flags().get_matches();
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .init();
    }

    let display = Display::connect_to_env()?;
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let globals = GlobalManager::new(&attached_display);
    event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!())?;

    let valid_outputs = get_valid_outputs(display.clone());
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
        let valid_outputs = get_valid_outputs(display);
        for output in valid_outputs {
            let (_, info) = output;
            println!("{:#?}", info.name);
        }
        std::process::exit(1);
    }

    if args.is_present("output") {
        let mut is_present = false;
        let valid_outputs = get_valid_outputs(display);

        for device in valid_outputs {
            let (output_device, info) = device;
            if info.name == args.value_of("output").unwrap().trim() {
                is_present = true;
                output = output_device.clone();
            }
        }
        if !is_present {
            bail!(
                "\"{}\" is not a valid output.",
                args.value_of("output").unwrap().trim()
            );
        }
    }

    if args.is_present("slurp") {
        if args.value_of("slurp").unwrap() == "" {
            bail!("Failed to recieve geometry.");
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
                    frame_formats.borrow_mut().push(FrameFormat {
                        format,
                        width,
                        height,
                        stride,
                    });
                }
                zwlr_screencopy_frame_v1::Event::Flags { .. } => {}
                zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                    frame_state.borrow_mut().replace(FrameState::Finished);
                }
                zwlr_screencopy_frame_v1::Event::Failed => {
                    frame_state.borrow_mut().replace(FrameState::Failed);
                }
                zwlr_screencopy_frame_v1::Event::Damage { .. } => {}
                zwlr_screencopy_frame_v1::Event::LinuxDmabuf { .. } => {}
                zwlr_screencopy_frame_v1::Event::BufferDone => {
                    frame_buffer_done.store(true, Ordering::SeqCst);
                }
                _ => unreachable!(),
            };
        }
    });

    while !frame_buffer_done.load(Ordering::SeqCst) {
        event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!())?;
    }

    debug!(formats = ?frame_formats, "received compositor frame buffer formats");

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

    debug!(format = ?frame_format, "selected frame buffer format");

    let frame_format = match frame_format {
        Some(format) => format,
        None => bail!("no suitable frame format found"),
    };

    let frame_bytes = frame_format.stride * frame_format.height;

    let mem_fd = create_shm_fd()?;
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

    let result = loop {
        event_queue.sync_roundtrip(&mut (), |_, _, _| {})?;

        if let Some(state) = frame_state.borrow_mut().take() {
            match state {
                FrameState::Failed => {
                    break Err(anyhow::anyhow!("frame copy failed"));
                }
                FrameState::Finished => {
                    let mut mmap = unsafe { MmapMut::map_mut(&mem_file)? };
                    let stdout = std::io::stdout();
                    let guard = stdout.lock();
                    let mut writer = std::io::BufWriter::new(guard);
                    let data = &mut *mmap;
                    let color_type = match frame_format.format {
                        wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 => {
                            for chunk in data.chunks_exact_mut(4) {
                                let tmp = chunk[0];
                                chunk[0] = chunk[2];
                                chunk[2] = tmp;
                            }
                            image::ColorType::Rgba8
                        }
                        wl_shm::Format::Xbgr8888 => image::ColorType::Rgba8,
                        other => {
                            break Err(anyhow::anyhow!("Unsupported buffer format: {:?}", other))
                        }
                    };
                    PngEncoder::new(&mut writer).write_image(
                        &mmap,
                        frame_format.width,
                        frame_format.height,
                        color_type,
                    )?;
                    writer.flush()?;
                    break Ok(());
                }
            }
        }
    };
    result
}

fn create_shm_fd() -> std::io::Result<RawFd> {
    // Only try memfd on linux
    #[cfg(target_os = "linux")]
    loop {
        match memfd::memfd_create(
            CStr::from_bytes_with_nul(b"wayshot\0").unwrap(),
            memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // this is only an optimization, so ignore errors
                let _ = fcntl::fcntl(
                    fd,
                    fcntl::F_ADD_SEALS(
                        fcntl::SealFlag::F_SEAL_SHRINK | fcntl::SealFlag::F_SEAL_SEAL,
                    ),
                );
                return Ok(fd);
            }
            Err(nix::Error::Sys(Errno::EINTR)) => continue,
            Err(nix::Error::Sys(Errno::ENOSYS)) => break,
            Err(nix::Error::Sys(errno)) => return Err(std::io::Error::from(errno)),
            Err(err) => unreachable!(err),
        }
    }

    // Fallback to using shm_open
    let sys_time = SystemTime::now();
    let mut mem_file_handle = format!(
        "/wayshot-{}",
        sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
    );
    loop {
        match mman::shm_open(
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(nix::Error::Sys(errno)) => match unistd::close(fd) {
                    Ok(_) => return Err(std::io::Error::from(errno)),
                    Err(nix::Error::Sys(errno)) => return Err(std::io::Error::from(errno)),
                    Err(err) => panic!("{}", err),
                },
                Err(err) => panic!("{}", err),
            },
            Err(nix::Error::Sys(Errno::EEXIST)) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = format!(
                    "/wayshot-{}",
                    sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                );
                continue;
            }
            Err(nix::Error::Sys(Errno::EINTR)) => continue,
            Err(nix::Error::Sys(errno)) => return Err(std::io::Error::from(errno)),
            Err(err) => unreachable!(err),
        }
    }
}

fn set_flags() -> App<'static> {
    let app = App::new("wayshot")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Simple screenshot tool for wlroots based compositors.")
        .arg(
            arg!(-s --slurp <GEOMETRY>)
                .required(false)
                .takes_value(true)
                .help("Choose a portion of your display to screenshot using slurp."),
        )
        .arg(
            arg!(-l - -listoutputs)
                .required(false)
                .takes_value(false)
                .help("List all valid outputs."),
        )
        .arg(
            arg!(-o --output <OUTPUT>)
                .required(false)
                .takes_value(true)
                .conflicts_with("slurp")
                .help("Choose a particular display to screenshot."),
        )
        .arg(
            arg!(-c - -cursor)
                .required(false)
                .takes_value(false)
                .help("Enable cursor in screenshots."),
        );
    app
}

fn get_valid_outputs(display: Display) -> Vec<(WlOutput, OutputInfo)> {
    let mut queue = display.create_event_queue();
    let attached_display = display.attach(queue.token());

    let (outputs, xdg_output) = XdgOutputHandler::new_output_handlers();
    let mut valid_outputs: Vec<(WlOutput, OutputInfo)> = Vec::new();

    let env = Environment::new(
        &attached_display,
        &mut &mut queue,
        ValidOutputs {
            outputs,
            xdg_output,
        },
    )
    .unwrap();

    queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

    for output in env.get_all_outputs() {
        with_output_info(&output, |info| {
            if info.obsolete == false {
                valid_outputs.push((output.clone(), info.clone()));
            } else {
                output.release();
            }
        });
    }
    valid_outputs
}

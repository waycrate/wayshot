use std::{
    cell::RefCell,
    error::Error,
    ffi::CStr,
    fs::File,
    io::Write,
    os::unix::prelude::FromRawFd,
    os::unix::prelude::RawFd,
    process::exit,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use nix::{
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};

use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
    ColorType,
    ColorType::Rgba8,
    ImageEncoder,
};

use memmap2::MmapMut;

use smithay_client_toolkit::reexports::{
    client::{
        protocol::{wl_output::WlOutput, wl_shm, wl_shm::Format},
        Display, GlobalManager, Main,
    },
    protocols::wlr::unstable::screencopy::v1::client::{
        zwlr_screencopy_frame_v1, zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
    },
};

/// Type of frame supported by the compositor. For now we only support Argb8888, Xrgb8888, and
/// Xbgr8888.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FrameFormat {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

/// State of the frame after attemting to copy it's data to a wl_buffer.
#[derive(Debug, Copy, Clone, PartialEq)]
enum FrameState {
    /// Compositor returned a failed event on calling `frame.copy`.
    Failed,
    /// Compositor sent a Ready event on calling `frame.copy`.
    Finished,
}

/// The copied frame comprising of the FrameFormat, ColorType (Rgba8), and a memory backed shm
/// file that holds the image data in it.
#[derive(Debug)]
pub struct FrameCopy {
    pub frame_format: FrameFormat,
    pub frame_color_type: ColorType,
    pub frame_mmap: MmapMut,
}

/// Struct to store region capture details.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CaptureRegion {
    /// X coordinate of the area to capture.
    pub x_coordinate: i32,
    /// y coordinate of the area to capture.
    pub y_coordinate: i32,
    /// Width of the capture area.
    pub width: i32,
    /// Height of the capture area.
    pub height: i32,
}

/// Supported image encoding formats.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EncodingFormat {
    /// Jpeg / jpg encoder.
    Jpg,
    /// Png encoder.
    Png,
}

/// Get a FrameCopy instance with screenshot pixel data for any wl_output object.
pub fn capture_output_frame(
    display: Display,
    cursor_overlay: i32,
    output: WlOutput,
    capture_region: Option<CaptureRegion>,
) -> Result<FrameCopy, Box<dyn Error>> {
    // Connecting to wayland environment.
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    // Instantiating the global manager.
    let globals = GlobalManager::new(&attached_display);
    event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!())?;

    let frame_formats: Rc<RefCell<Vec<FrameFormat>>> = Rc::new(RefCell::new(Vec::new()));
    let frame_state: Rc<RefCell<Option<FrameState>>> = Rc::new(RefCell::new(None));
    let frame_buffer_done = Rc::new(AtomicBool::new(false));

    // Instantiating screencopy manager.
    let screencopy_manager = match globals.instantiate_exact::<ZwlrScreencopyManagerV1>(3) {
        Ok(x) => x,
        Err(e) => {
            log::error!("Failed to create screencopy manager. Does your compositor implement ZwlrScreencopy?");
            panic!("{:#?}", e);
        }
    };

    // Capture output.
    let frame: Main<ZwlrScreencopyFrameV1> = if let Some(region) = capture_region {
        screencopy_manager.capture_output_region(
            cursor_overlay,
            &output,
            region.x_coordinate,
            region.y_coordinate,
            region.width,
            region.height,
        )
    } else {
        screencopy_manager.capture_output(cursor_overlay, &output)
    };

    // Assign callback to frame.
    frame.quick_assign({
        // Clone data to mutate their values in the callback.
        let frame_formats = frame_formats.clone();
        let frame_state = frame_state.clone();
        let frame_buffer_done = frame_buffer_done.clone();
        move |_, event, _| {
            match event {
                zwlr_screencopy_frame_v1::Event::Buffer {
                    format,
                    width,
                    height,
                    stride,
                } => {
                    log::debug!("Received Buffer event");
                    frame_formats.borrow_mut().push(FrameFormat {
                        format,
                        width,
                        height,
                        stride,
                    })
                }
                zwlr_screencopy_frame_v1::Event::Flags { .. } => {
                    log::debug!("Received Flags event");
                }
                zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                    // If the frame is successfully copied, a “flags” and a “ready” events are sent. Otherwise, a “failed” event is sent.
                    // This is useful when we call .copy on the frame object.
                    log::debug!("Received Ready event");
                    frame_state.borrow_mut().replace(FrameState::Finished);
                }
                zwlr_screencopy_frame_v1::Event::Failed => {
                    log::debug!("Received Failed event");
                    frame_state.borrow_mut().replace(FrameState::Failed);
                }
                zwlr_screencopy_frame_v1::Event::Damage { .. } => {
                    log::debug!("Received Damage event");
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

    // Empty internal event buffer until buffer_done is set to true which is when the Buffer done
    // event is fired, aka the capture from the compositor is succesful.
    while !frame_buffer_done.load(Ordering::SeqCst) {
        event_queue.dispatch(&mut (), |_, _, _| unreachable!())?;
    }

    log::debug!(
        "Received compositor frame buffer formats: {:#?}",
        frame_formats
    );
    // Filter advertised wl_shm formats and select the first one that matches.
    let frame_format = frame_formats
        .borrow()
        .iter()
        .find(|frame| {
            matches!(
                frame.format,
                wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 | wl_shm::Format::Xbgr8888
            )
        })
        .copied();
    log::debug!("Selected frame buffer format: {:#?}", frame_format);

    // Check if frame format exists.
    let frame_format = match frame_format {
        Some(format) => format,
        None => {
            log::error!("No suitable frame format found");
            exit(1);
        }
    };

    // Bytes of data in the frame = stride * height.
    let frame_bytes = frame_format.stride * frame_format.height;

    // Create an in memory file and return it's file descriptor.
    let mem_fd = create_shm_fd()?;
    let mem_file = unsafe { File::from_raw_fd(mem_fd) };
    mem_file.set_len(frame_bytes as u64)?;

    // Instantiate shm global.
    let shm = globals.instantiate_exact::<wl_shm::WlShm>(1)?;
    let shm_pool = shm.create_pool(mem_fd, frame_bytes as i32);
    let buffer = shm_pool.create_buffer(
        0,
        frame_format.width as i32,
        frame_format.height as i32,
        frame_format.stride as i32,
        frame_format.format,
    );

    // Copy the pixel data advertised by the compositor into the buffer we just created.
    frame.copy(&buffer);

    // On copy the Ready / Failed events are fired by the frame object, so here we check for them.
    loop {
        event_queue.dispatch(&mut (), |_, _, _| {})?;

        // Basically reads, if frame state is not None then...
        if let Some(state) = frame_state.borrow_mut().take() {
            match state {
                FrameState::Failed => {
                    log::error!("Frame copy failed");
                    exit(1);
                }
                FrameState::Finished => {
                    // Create a writeable memory map backed by a mem_file.
                    let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file)? };
                    let data = &mut *frame_mmap;
                    let frame_color_type = match frame_format.format {
                        wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 => {
                            // Swap out b with r as these formats are in little endian notation.
                            for chunk in data.chunks_exact_mut(4) {
                                chunk.swap(0, 2);
                            }
                            Rgba8
                        }
                        wl_shm::Format::Xbgr8888 => Rgba8,
                        unsupported_format => {
                            log::error!("Unsupported buffer format: {:?}", unsupported_format);
                            log::error!("You can send a feature request for the above format to the mailing list for wayshot over at https://sr.ht/~shinyzenith/wayshot.");
                            exit(1);
                        }
                    };
                    return Ok(FrameCopy {
                        frame_format,
                        frame_color_type,
                        frame_mmap,
                    });
                }
            }
        }
    }
}

/// Return a RawFd to a shm file. We use memfd create on linux and shm_open for BSD support.
/// You don't need to mess around with this function, it is only used by
/// capture_output_frame.
fn create_shm_fd() -> std::io::Result<RawFd> {
    // Only try memfd on linux.
    #[cfg(target_os = "linux")]
    loop {
        // Create a file that closes on succesful execution and seal it's operations.
        match memfd::memfd_create(
            CStr::from_bytes_with_nul(b"wayshot\0").unwrap(),
            memfd::MemFdCreateFlag::MFD_CLOEXEC | memfd::MemFdCreateFlag::MFD_ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // This is only an optimization, so ignore errors.
                // F_SEAL_SRHINK = File cannot be reduced in size.
                // F_SEAL_SEAL = Prevent further calls to fcntl().
                let _ = fcntl::fcntl(
                    fd,
                    fcntl::F_ADD_SEALS(
                        fcntl::SealFlag::F_SEAL_SHRINK | fcntl::SealFlag::F_SEAL_SEAL,
                    ),
                );
                return Ok(fd);
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(nix::errno::Errno::ENOSYS) => break,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }

    // Fallback to using shm_open.
    let sys_time = SystemTime::now();
    let mut mem_file_handle = format!(
        "/wayshot-{}",
        sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
    );
    loop {
        match mman::shm_open(
            // O_CREAT = Create file if does not exist.
            // O_EXCL = Error if create and file exists.
            // O_RDWR = Open for reading and writing.
            // O_CLOEXEC = Close on succesful execution.
            // S_IRUSR = Set user read permission bit .
            // S_IWUSR = Set user write permission bit.
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(errno) => match unistd::close(fd) {
                    Ok(_) => return Err(std::io::Error::from(errno)),
                    Err(errno) => return Err(std::io::Error::from(errno)),
                },
            },
            Err(nix::errno::Errno::EEXIST) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = format!(
                    "/wayshot-{}",
                    sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                );
                continue;
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }
}

/// Write an instance of FrameCopy to anything that implements Write trait. Eg: Stdout or a file
/// on the disk.
pub fn write_to_file(
    mut output_file: impl Write,
    encoding_format: EncodingFormat,
    frame_copy: FrameCopy,
) -> Result<(), Box<dyn Error>> {
    log::debug!(
        "Writing to disk with encoding format: {:#?}",
        encoding_format
    );
    match encoding_format {
        EncodingFormat::Jpg => {
            JpegEncoder::new(&mut output_file).write_image(
                &frame_copy.frame_mmap,
                frame_copy.frame_format.width,
                frame_copy.frame_format.height,
                frame_copy.frame_color_type,
            )?;
            let _ = output_file.flush()?;
        }
        EncodingFormat::Png => {
            PngEncoder::new(&mut output_file).write_image(
                &frame_copy.frame_mmap,
                frame_copy.frame_format.width,
                frame_copy.frame_format.height,
                frame_copy.frame_color_type,
            )?;
            let _ = output_file.flush()?;
        }
    }

    Ok(())
}

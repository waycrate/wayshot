mod convert;

use std::{
    error::Error,
    ffi::CStr,
    fs::File,
    io::Write,
    os::unix::prelude::FromRawFd,
    os::unix::prelude::RawFd,
    process::exit,
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use nix::{
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};

use image::{
    codecs::{
        jpeg::JpegEncoder,
        png::PngEncoder,
        pnm::{self, PnmEncoder},
    },
    ColorType, ImageEncoder,
};

use memmap2::MmapMut;

use wayland_client::{
    delegate_noop,
    globals::GlobalList,
    protocol::{
        wl_buffer::WlBuffer, wl_output::WlOutput, wl_shm, wl_shm::Format, wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
    },
    Connection, Dispatch, QueueHandle,
    WEnum::Value,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1, zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::convert::create_converter;

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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EncodingFormat {
    /// Jpeg / jpg encoder.
    Jpg,
    /// Png encoder.
    Png,
    /// Ppm encoder
    Ppm,
}

impl From<EncodingFormat> for image::ImageOutputFormat {
    fn from(format: EncodingFormat) -> Self {
        match format {
            EncodingFormat::Jpg => image::ImageFormat::Jpeg.into(),
            EncodingFormat::Png => image::ImageFormat::Png.into(),
            EncodingFormat::Ppm => image::ImageFormat::Pnm.into(),
        }
    }
}

impl From<EncodingFormat> for &str {
    fn from(format: EncodingFormat) -> Self {
        match format {
            EncodingFormat::Jpg => "jpg",
            EncodingFormat::Png => "png",
            EncodingFormat::Ppm => "ppm",
        }
    }
}

struct CaptureFrameState {
    formats: Vec<FrameFormat>,
    state: Option<FrameState>,
    buffer_done: AtomicBool,
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for CaptureFrameState {
    fn event(
        frame: &mut Self,
        _: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                log::debug!("Received Buffer event");
                if let Value(f) = format {
                    frame.formats.push(FrameFormat {
                        format: f,
                        width,
                        height,
                        stride,
                    })
                } else {
                    log::debug!("Received Buffer event with unidentified format");
                    exit(1);
                }
            }
            zwlr_screencopy_frame_v1::Event::Flags { .. } => {
                log::debug!("Received Flags event");
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                // If the frame is successfully copied, a “flags” and a “ready” events are sent. Otherwise, a “failed” event is sent.
                // This is useful when we call .copy on the frame object.
                log::debug!("Received Ready event");
                frame.state.replace(FrameState::Finished);
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                log::debug!("Received Failed event");
                frame.state.replace(FrameState::Failed);
            }
            zwlr_screencopy_frame_v1::Event::Damage { .. } => {
                log::debug!("Received Damage event");
            }
            zwlr_screencopy_frame_v1::Event::LinuxDmabuf { .. } => {
                log::debug!("Received LinuxDmaBuf event");
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => {
                log::debug!("Received bufferdone event");
                frame.buffer_done.store(true, Ordering::SeqCst);
            }
            _ => unreachable!(),
        };
    }
}

delegate_noop!(CaptureFrameState: ignore WlShm);
delegate_noop!(CaptureFrameState: ignore WlShmPool);
delegate_noop!(CaptureFrameState: ignore WlBuffer);
delegate_noop!(CaptureFrameState: ignore ZwlrScreencopyManagerV1);

/// Get a FrameCopy instance with screenshot pixel data for any wl_output object.
pub fn capture_output_frame(
    globals: &mut GlobalList,
    conn: &mut Connection,
    cursor_overlay: i32,
    output: WlOutput,
    capture_region: Option<CaptureRegion>,
) -> Result<FrameCopy, Box<dyn Error>> {
    // Connecting to wayland environment.
    let mut state = CaptureFrameState {
        formats: Vec::new(),
        state: None,
        buffer_done: AtomicBool::new(false),
    };
    let mut event_queue = conn.new_event_queue::<CaptureFrameState>();
    let qh = event_queue.handle();

    // Instantiating screencopy manager.
    let screencopy_manager = match globals.bind::<ZwlrScreencopyManagerV1, _, _>(&qh, 3..=3, ()) {
        Ok(x) => x,
        Err(e) => {
            log::error!("Failed to create screencopy manager. Does your compositor implement ZwlrScreencopy?");
            panic!("{:#?}", e);
        }
    };

    // Capture output.
    let frame: ZwlrScreencopyFrameV1 = if let Some(region) = capture_region {
        screencopy_manager.capture_output_region(
            cursor_overlay,
            &output,
            region.x_coordinate,
            region.y_coordinate,
            region.width,
            region.height,
            &qh,
            (),
        )
    } else {
        screencopy_manager.capture_output(cursor_overlay, &output, &qh, ())
    };

    // Empty internal event buffer until buffer_done is set to true which is when the Buffer done
    // event is fired, aka the capture from the compositor is succesful.
    while !state.buffer_done.load(Ordering::SeqCst) {
        event_queue.blocking_dispatch(&mut state)?;
    }

    log::debug!(
        "Received compositor frame buffer formats: {:#?}",
        state.formats
    );
    // Filter advertised wl_shm formats and select the first one that matches.
    let frame_format = state
        .formats
        .iter()
        .find(|frame| {
            matches!(
                frame.format,
                wl_shm::Format::Xbgr2101010
                    | wl_shm::Format::Abgr2101010
                    | wl_shm::Format::Argb8888
                    | wl_shm::Format::Xrgb8888
                    | wl_shm::Format::Xbgr8888
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
    let shm = globals.bind::<WlShm, _, _>(&qh, 1..=1, ()).unwrap();
    let shm_pool = shm.create_pool(mem_fd, frame_bytes as i32, &qh, ());
    let buffer = shm_pool.create_buffer(
        0,
        frame_format.width as i32,
        frame_format.height as i32,
        frame_format.stride as i32,
        frame_format.format,
        &qh,
        (),
    );

    // Copy the pixel data advertised by the compositor into the buffer we just created.
    frame.copy(&buffer);

    // On copy the Ready / Failed events are fired by the frame object, so here we check for them.
    loop {
        // Basically reads, if frame state is not None then...
        if let Some(state) = state.state {
            match state {
                FrameState::Failed => {
                    log::error!("Frame copy failed");
                    exit(1);
                }
                FrameState::Finished => {
                    // Create a writeable memory map backed by a mem_file.
                    let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file)? };
                    let data = &mut *frame_mmap;
                    let frame_color_type = if let Some(converter) =
                        create_converter(frame_format.format)
                    {
                        converter.convert_inplace(data)
                    } else {
                        log::error!("Unsupported buffer format: {:?}", frame_format.format);
                        log::error!("You can send a feature request for the above format to the mailing list for wayshot over at https://sr.ht/~shinyzenith/wayshot.");
                        exit(1);
                    };
                    return Ok(FrameCopy {
                        frame_format,
                        frame_color_type,
                        frame_mmap,
                    });
                }
            }
        }

        event_queue.blocking_dispatch(&mut state)?;
    }
}

/// Return a RawFd to a shm file. We use memfd create on linux and shm_open for BSD support.
/// You don't need to mess around with this function, it is only used by
/// capture_output_frame.
fn create_shm_fd() -> std::io::Result<RawFd> {
    // Only try memfd on linux and freebsd.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
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
    mut output_file: &mut impl Write,
    encoding_format: EncodingFormat,
    frame_copy: &FrameCopy,
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
            output_file.flush()?;
        }
        EncodingFormat::Png => {
            PngEncoder::new(&mut output_file).write_image(
                &frame_copy.frame_mmap,
                frame_copy.frame_format.width,
                frame_copy.frame_format.height,
                frame_copy.frame_color_type,
            )?;
            output_file.flush()?;
        }
        EncodingFormat::Ppm => {
            let rgb8_data = if let ColorType::Rgba8 = frame_copy.frame_color_type {
                let mut data = Vec::with_capacity(
                    (3 * frame_copy.frame_format.width * frame_copy.frame_format.height) as _,
                );
                for chunk in frame_copy.frame_mmap.chunks_exact(4) {
                    data.extend_from_slice(&chunk[..3]);
                }
                data
            } else {
                unimplemented!("Currently only ColorType::Rgba8 is supported")
            };

            PnmEncoder::new(&mut output_file)
                .with_subtype(pnm::PnmSubtype::Pixmap(pnm::SampleEncoding::Binary))
                .write_image(
                    &rgb8_data,
                    frame_copy.frame_format.width,
                    frame_copy.frame_format.height,
                    ColorType::Rgb8,
                )?;
            output_file.flush()?;
        }
    }

    Ok(())
}

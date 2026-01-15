use std::{
    ffi::CString,
    os::fd::OwnedFd,
    time::{SystemTime, UNIX_EPOCH},
};

use gbm::BufferObject;
use image::{ColorType, DynamicImage, ImageBuffer, Pixel};
use memmap2::MmapMut;
use r_egl_wayland::r_egl as egl;
use rustix::{
    fs::{self, SealFlags},
    io, shm,
};
use wayland_client::protocol::{
    wl_buffer::WlBuffer, wl_output, wl_shm::Format, wl_shm_pool::WlShmPool,
};

use crate::{
    Error, Result,
    convert::create_converter,
    region::{LogicalRegion, Size},
};

pub struct FrameGuard {
    pub buffer: WlBuffer,
    pub shm_pool: WlShmPool,
    pub size: Size,
}

impl Drop for FrameGuard {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.shm_pool.destroy();
    }
}

pub struct DMAFrameGuard {
    pub buffer: WlBuffer,
}
impl Drop for DMAFrameGuard {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

pub struct EGLImageGuard<'a, T: r_egl_wayland::EGL1_5> {
    pub image: egl::Image,
    pub(crate) egl_instance: &'a egl::Instance<T>,
    pub(crate) egl_display: egl::Display,
}

impl<T: egl::api::EGL1_5> Drop for EGLImageGuard<'_, T> {
    fn drop(&mut self) {
        self.egl_instance
            .destroy_image(self.egl_display, self.image)
            .unwrap_or_else(|e| {
                tracing::error!("EGLimage destruction had error: {e}");
            });
    }
}

/// Type of frame supported by the compositor. For now we only support Argb8888, Xrgb8888, and
/// Xbgr8888.
///
/// See `zwlr_screencopy_frame_v1::Event::Buffer` as it's retrieved from there.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FrameFormat {
    pub format: Format,
    /// Size of the frame in pixels. This will always be in "landscape" so a
    /// portrait 1080x1920 frame will be 1920x1080 and will need to be rotated!
    pub size: Size,
    /// Stride is the number of bytes between the start of a row and the start of the next row.
    pub stride: u32,
}

/// Type of DMABUF frame supported by the compositor
///
/// See `zwlr_screencopy_frame_v1::Event::linux_dmabuf` as it's retrieved from there.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DMAFrameFormat {
    pub format: u32,
    /// Size of the frame in pixels. This will always be in "landscape" so a
    /// portrait 1080x1920 frame will be 1920x1080 and will need to be rotated!
    pub size: Size,
}

impl FrameFormat {
    /// Returns the size of the frame in bytes, which is the stride * height.
    pub fn byte_size(&self) -> u64 {
        self.stride as u64 * self.size.height as u64
    }
}

#[tracing::instrument(skip(frame_data))]
fn create_image_buffer<P>(
    frame_format: &FrameFormat,
    frame_data: &FrameData,
) -> Result<ImageBuffer<P, Vec<P::Subpixel>>>
where
    P: Pixel<Subpixel = u8>,
{
    tracing::debug!("Creating image buffer");
    match frame_data {
        FrameData::Mmap(frame_mmap) => ImageBuffer::from_vec(
            frame_format.size.width,
            frame_format.size.height,
            frame_mmap.to_vec(),
        )
        .ok_or(Error::BufferTooSmall),
        FrameData::GBMBo(_) => todo!(),
    }
}

#[derive(Debug)]
pub enum FrameData {
    Mmap(MmapMut),
    GBMBo(BufferObject<()>),
}
/// The copied frame comprising of the FrameFormat, ColorType (Rgba8), and a memory backed shm
/// file that holds the image data in it.
#[derive(Debug)]
pub struct FrameCopy {
    pub frame_format: FrameFormat,
    pub frame_color_type: ColorType,
    pub frame_data: FrameData,
    pub transform: wl_output::Transform,
    /// Logical region with the transform already applied.
    pub logical_region: LogicalRegion,
    pub physical_size: Size,
}

impl FrameCopy {
    pub(crate) fn get_image(&mut self) -> Result<DynamicImage, Error> {
        let frame_color_type = match create_converter(self.frame_format.format) {
            Some(converter) => {
                let FrameData::Mmap(raw) = &mut self.frame_data else {
                    return Err(Error::InvalidColor);
                };
                converter.convert_inplace(raw)
            }
            _ => {
                tracing::error!("Unsupported buffer format: {:?}", self.frame_format.format);
                tracing::error!(
                    "You can send a feature request for the above format to the mailing list for wayshot over at https://sr.ht/~shinyzenith/wayshot."
                );
                return Err(Error::NoSupportedBufferFormat);
            }
        };
        self.frame_color_type = frame_color_type;
        let image: DynamicImage = (&*self).try_into()?;
        Ok(image)
    }
}

impl TryFrom<&FrameCopy> for DynamicImage {
    type Error = Error;

    fn try_from(value: &FrameCopy) -> Result<Self> {
        Ok(match value.frame_color_type {
            ColorType::Rgb8 => {
                Self::ImageRgb8(create_image_buffer(&value.frame_format, &value.frame_data)?)
            }
            ColorType::Rgba8 => {
                Self::ImageRgba8(create_image_buffer(&value.frame_format, &value.frame_data)?)
            }
            _ => return Err(Error::InvalidColor),
        })
    }
}

fn get_mem_file_handle() -> String {
    format!(
        "/libwayshot-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.subsec_nanos().to_string())
            .unwrap_or("unknown".into())
    )
}

/// Return a RawFd to a shm file. We use memfd create on linux and shm_open for BSD support.
/// You don't need to mess around with this function, it is only used by
/// capture_output_frame.
pub fn create_shm_fd() -> std::io::Result<OwnedFd> {
    // Only try memfd on linux and freebsd.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    loop {
        // Create a file that closes on successful execution and seal it's operations.
        match fs::memfd_create(
            CString::new("libwayshot")?.as_c_str(),
            fs::MemfdFlags::CLOEXEC | fs::MemfdFlags::ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // This is only an optimization, so ignore errors.
                // F_SEAL_SRHINK = File cannot be reduced in size.
                // F_SEAL_SEAL = Prevent further calls to fcntl().
                let _ = fs::fcntl_add_seals(&fd, fs::SealFlags::SHRINK | SealFlags::SEAL);
                return Ok(fd);
            }
            Err(io::Errno::INTR) => continue,
            Err(io::Errno::NOSYS) => break,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }

    // Fallback to using shm_open.
    let mut mem_file_handle = get_mem_file_handle();
    loop {
        let open_result = shm::open(
            mem_file_handle.as_str(),
            shm::OFlags::CREATE | shm::OFlags::EXCL | shm::OFlags::RDWR,
            fs::Mode::RUSR | fs::Mode::WUSR,
        );
        // O_CREAT = Create file if does not exist.
        // O_EXCL = Error if create and file exists.
        // O_RDWR = Open for reading and writing.
        // O_CLOEXEC = Close on successful execution.
        // S_IRUSR = Set user read permission bit .
        // S_IWUSR = Set user write permission bit.
        match open_result {
            Ok(fd) => match shm::unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(errno) => return Err(std::io::Error::from(errno)),
            },
            Err(io::Errno::EXIST) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = get_mem_file_handle();
                continue;
            }
            Err(io::Errno::INTR) => continue,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }
}

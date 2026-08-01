use std::{
    ffi::CString,
    os::fd::OwnedFd,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "dmabuf")]
use gbm::BufferObject;
use image::{ColorType, DynamicImage, EncodableLayout, Rgb, RgbImage, Rgba, RgbaImage};
use memmap2::MmapMut;
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

#[cfg(feature = "dmabuf")]
pub struct DMAFrameGuard {
    pub buffer: WlBuffer,
}
#[cfg(feature = "dmabuf")]
impl Drop for DMAFrameGuard {
    fn drop(&mut self) {
        self.buffer.destroy();
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

#[derive(Debug)]
pub enum FrameData {
    Mmap(MmapMut),
    #[cfg(feature = "dmabuf")]
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
    pub(crate) color_converted: bool,
}

impl FrameCopy {
    pub(crate) fn convert_color_inplace(&mut self) -> Result<ColorType, Error> {
        if self.color_converted {
            return Ok(self.frame_color_type);
        }
        let frame_color_type = match create_converter(self.frame_format.format) {
            Some(converter) => {
                #[cfg_attr(not(feature = "dmabuf"), allow(irrefutable_let_patterns))]
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
        self.color_converted = true;
        Ok(frame_color_type)
    }

    pub(crate) fn get_image(&mut self) -> Result<DynamicImage, Error> {
        self.convert_color_inplace()?;
        let image: DynamicImage = (&*self).try_into()?;
        Ok(image)
    }
}

impl TryFrom<&FrameCopy> for DynamicImage {
    type Error = Error;

    fn try_from(value: &FrameCopy) -> Result<Self> {
        value.allocate_image()
    }
}

impl FrameCopy {
    fn allocate_image(&self) -> Result<DynamicImage> {
        match self.frame_color_type {
            ColorType::Rgb8 => self.allocate_image_rgb8(),
            ColorType::Rgba8 => self.allocate_image_rgba8(),
            _ => Err(Error::InvalidColor),
        }
    }

    fn data(&self) -> &[u8] {
        match &self.frame_data {
            FrameData::Mmap(frame_mmap) => frame_mmap.as_bytes(),
            #[cfg(feature = "dmabuf")]
            FrameData::GBMBo(_) => unimplemented!("it is still not used, and todo"),
        }
    }

    fn pixel_position(&self, index: usize) -> (u32, u32) {
        let index = index as u32;
        let Size { width, height } = self.frame_format.size;
        match self.transform {
            wl_output::Transform::Normal => (index % width, index / width),
            wl_output::Transform::_90 => (height - index / width - 1, index % width),
            wl_output::Transform::_180 => (width - index % width - 1, height - index / width - 1),
            wl_output::Transform::_270 => (index / width, width - index % width - 1),
            wl_output::Transform::Flipped => (width - index % width - 1, index / width),
            wl_output::Transform::Flipped90 => (index / width, index % width),
            wl_output::Transform::Flipped180 => (index % width, height - index / width - 1),
            wl_output::Transform::Flipped270 => {
                (height - index / width - 1, width - index % width - 1)
            }
            _ => unreachable!(),
        }
    }
    fn image_shape(&self) -> (u32, u32) {
        match self.transform {
            wl_output::Transform::Normal
            | wl_output::Transform::_180
            | wl_output::Transform::Flipped
            | wl_output::Transform::Flipped180 => {
                (self.frame_format.size.width, self.frame_format.size.height)
            }
            _ => (self.frame_format.size.height, self.frame_format.size.width),
        }
    }

    fn allocate_image_rgba8(&self) -> Result<DynamicImage> {
        let (width, height) = self.image_shape();
        let mut img = RgbaImage::new(width, height);
        for (index, pixel) in self.data().chunks(4).enumerate() {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            let a = pixel[3];
            let (x, y) = self.pixel_position(index);
            img.put_pixel(x, y, Rgba([r, g, b, a]));
        }
        Ok(DynamicImage::ImageRgba8(img))
    }

    fn allocate_image_rgb8(&self) -> Result<DynamicImage> {
        let (width, height) = self.image_shape();
        let mut img = RgbImage::new(width, height);
        for (index, pixel) in self.data().chunks(3).enumerate() {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            let (x, y) = self.pixel_position(index);
            img.put_pixel(x, y, Rgb([r, g, b]));
        }
        Ok(DynamicImage::ImageRgb8(img))
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

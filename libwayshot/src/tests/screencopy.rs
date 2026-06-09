use crate::region::{LogicalRegion, Size};
use crate::screencopy::{DMAFrameFormat, FrameCopy, FrameData, FrameFormat, create_shm_fd};
use image::ColorType;
use memmap2::MmapOptions;
use wayland_client::protocol::{wl_output, wl_shm};

#[test]
fn frame_format_byte_size() {
    let format = FrameFormat {
        format: wl_shm::Format::Argb8888,
        size: Size {
            width: 100,
            height: 200,
        },
        stride: 400,
    };
    assert_eq!(format.byte_size(), 400 * 200);
}

#[test]
fn frame_format_byte_size_small() {
    let format = FrameFormat {
        format: wl_shm::Format::Xrgb8888,
        size: Size {
            width: 2,
            height: 2,
        },
        stride: 8,
    };
    assert_eq!(format.byte_size(), 16);
}

fn make_frame_copy(
    format: wl_shm::Format,
    width: u32,
    height: u32,
    color_converted: bool,
) -> FrameCopy {
    let stride = width * 4;
    let len = (stride * height) as usize;
    let mmap = MmapOptions::new().len(len).map_anon().unwrap();
    FrameCopy {
        frame_format: FrameFormat {
            format,
            size: Size { width, height },
            stride,
        },
        frame_color_type: ColorType::Rgba8,
        frame_data: FrameData::Mmap(mmap),
        transform: wl_output::Transform::Normal,
        logical_region: LogicalRegion::default(),
        physical_size: Size { width, height },
        color_converted,
    }
}

#[test]
fn dma_frame_format_fields() {
    let fmt = DMAFrameFormat {
        format: 0x34325241, // AR24
        size: Size {
            width: 1920,
            height: 1080,
        },
    };
    assert_eq!(fmt.format, 0x34325241);
    assert_eq!(fmt.size.width, 1920);
    assert_eq!(fmt.size.height, 1080);
}

#[test]
fn frame_copy_convert_color_already_converted_is_idempotent() {
    let mut fc = make_frame_copy(wl_shm::Format::Argb8888, 4, 4, true);
    let result = fc.convert_color_inplace();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ColorType::Rgba8);
}

#[test]
fn frame_copy_convert_color_supported_format_succeeds() {
    let mut fc = make_frame_copy(wl_shm::Format::Argb8888, 4, 4, false);
    let result = fc.convert_color_inplace();
    assert!(result.is_ok());
    assert!(fc.color_converted);
}

#[test]
fn frame_copy_convert_color_unsupported_format_errors() {
    let stride = 4 * 4;
    let mmap = MmapOptions::new().len(stride * 4).map_anon().unwrap();
    let mut fc = FrameCopy {
        frame_format: FrameFormat {
            format: wl_shm::Format::Argb2101010, // unsupported
            size: Size {
                width: 4,
                height: 4,
            },
            stride: stride as u32,
        },
        frame_color_type: ColorType::Rgba8,
        frame_data: FrameData::Mmap(mmap),
        transform: wl_output::Transform::Normal,
        logical_region: LogicalRegion::default(),
        physical_size: Size {
            width: 4,
            height: 4,
        },
        color_converted: false,
    };
    assert!(fc.convert_color_inplace().is_err());
}

#[test]
fn frame_copy_try_from_rgba8_produces_image() {
    use image::DynamicImage;
    let fc = make_frame_copy(wl_shm::Format::Xbgr8888, 2, 2, false);
    // color_converted=false but frame_color_type=Rgba8 and format is ConvertNone
    // TryFrom uses frame_color_type, not format
    let result = DynamicImage::try_from(&fc);
    assert!(result.is_ok());
    let img = result.unwrap();
    assert_eq!(img.width(), 2);
    assert_eq!(img.height(), 2);
}

#[test]
fn frame_copy_try_from_invalid_color_type_errors() {
    use image::DynamicImage;
    let stride = 4 * 4;
    let mmap = MmapOptions::new().len(stride * 4).map_anon().unwrap();
    let fc = FrameCopy {
        frame_format: FrameFormat {
            format: wl_shm::Format::Xbgr8888,
            size: Size {
                width: 4,
                height: 4,
            },
            stride: stride as u32,
        },
        frame_color_type: ColorType::L8, // unsupported
        frame_data: FrameData::Mmap(mmap),
        transform: wl_output::Transform::Normal,
        logical_region: LogicalRegion::default(),
        physical_size: Size {
            width: 4,
            height: 4,
        },
        color_converted: false,
    };
    assert!(DynamicImage::try_from(&fc).is_err());
}

#[test]
fn frame_copy_get_image_succeeds_for_supported_format() {
    let mut fc = make_frame_copy(wl_shm::Format::Abgr8888, 4, 4, false);
    let result = fc.get_image();
    assert!(result.is_ok());
    let img = result.unwrap();
    assert_eq!(img.width(), 4);
    assert_eq!(img.height(), 4);
}

#[test]
fn frame_copy_into_mmap_rgba_image_buffer_succeeds() {
    let fc = make_frame_copy(wl_shm::Format::Xbgr8888, 4, 4, false);
    // frame_color_type is Rgba8, so this should succeed
    let result = fc.into_mmap_rgba_image_buffer();
    assert!(result.is_ok());
    let buf = result.unwrap();
    assert_eq!(buf.width(), 4);
    assert_eq!(buf.height(), 4);
}

#[test]
fn frame_copy_into_mmap_rgba_image_buffer_fails_for_non_rgba8() {
    let stride = 4 * 3;
    let mmap = MmapOptions::new().len(stride * 4).map_anon().unwrap();
    let fc = FrameCopy {
        frame_format: FrameFormat {
            format: wl_shm::Format::Bgr888,
            size: Size {
                width: 4,
                height: 4,
            },
            stride: stride as u32,
        },
        frame_color_type: ColorType::Rgb8,
        frame_data: FrameData::Mmap(mmap),
        transform: wl_output::Transform::Normal,
        logical_region: LogicalRegion::default(),
        physical_size: Size {
            width: 4,
            height: 4,
        },
        color_converted: true,
    };
    assert!(fc.into_mmap_rgba_image_buffer().is_err());
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[test]
fn create_shm_fd_returns_valid_fd() {
    let result = create_shm_fd();
    assert!(result.is_ok());
}

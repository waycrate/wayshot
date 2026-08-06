use crate::region::{LogicalRegion, Size};
use crate::screencopy::{
    DMAFrameFormat, FrameCopy, FrameData, FrameFormat, FrameGuard, create_shm_fd,
};
use image::ColorType;
use memmap2::MmapOptions;
use std::os::unix::net::UnixStream;
use wayland_backend::client::Backend;
use wayland_client::Proxy;
use wayland_client::protocol::{wl_buffer::WlBuffer, wl_output, wl_shm, wl_shm_pool::WlShmPool};

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

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[test]
fn create_shm_fd_returns_valid_fd() {
    let result = create_shm_fd();
    assert!(result.is_ok());
}

fn dummy_conn() -> wayland_client::Connection {
    let (client, server) = UnixStream::pair().expect("unix stream");
    Box::leak(Box::new(server));
    let backend = Backend::connect(client).expect("backend");
    wayland_client::Connection::from_backend(backend)
}

fn inert<T: Proxy>(conn: &wayland_client::Connection) -> T {
    T::inert(conn.backend().downgrade())
}

fn make_frame_copy_rgb8(width: u32, height: u32, transform: wl_output::Transform) -> FrameCopy {
    let stride = width * 3;
    let len = (stride * height) as usize;
    let mmap = MmapOptions::new().len(len).map_anon().unwrap();
    FrameCopy {
        frame_format: FrameFormat {
            format: wl_shm::Format::Bgr888,
            size: Size { width, height },
            stride,
        },
        frame_color_type: ColorType::Rgb8,
        frame_data: FrameData::Mmap(mmap),
        transform,
        logical_region: LogicalRegion::default(),
        physical_size: Size { width, height },
        color_converted: true,
    }
}

fn make_frame_copy_with_transform(
    width: u32,
    height: u32,
    transform: wl_output::Transform,
) -> FrameCopy {
    let stride = width * 4;
    let len = (stride * height) as usize;
    let mmap = MmapOptions::new().len(len).map_anon().unwrap();
    FrameCopy {
        frame_format: FrameFormat {
            format: wl_shm::Format::Xbgr8888,
            size: Size { width, height },
            stride,
        },
        frame_color_type: ColorType::Rgba8,
        frame_data: FrameData::Mmap(mmap),
        transform,
        logical_region: LogicalRegion::default(),
        physical_size: Size { width, height },
        color_converted: true,
    }
}

#[test]
fn frame_copy_try_from_rgb8_produces_image() {
    use image::DynamicImage;
    let fc = make_frame_copy_rgb8(3, 2, wl_output::Transform::Normal);
    let img = DynamicImage::try_from(&fc).expect("rgb8 image");
    assert_eq!(img.width(), 3);
    assert_eq!(img.height(), 2);
}

#[test]
fn pixel_position_and_image_shape_cover_all_transforms() {
    use image::DynamicImage;
    // Use a non-square image so width/height swaps are observable.
    let transforms = [
        (wl_output::Transform::Normal, (3, 2)),
        (wl_output::Transform::_90, (2, 3)),
        (wl_output::Transform::_180, (3, 2)),
        (wl_output::Transform::_270, (2, 3)),
        (wl_output::Transform::Flipped, (3, 2)),
        (wl_output::Transform::Flipped90, (2, 3)),
        (wl_output::Transform::Flipped180, (3, 2)),
        (wl_output::Transform::Flipped270, (2, 3)),
    ];
    for (transform, (expected_width, expected_height)) in transforms {
        let fc = make_frame_copy_with_transform(3, 2, transform);
        let img = DynamicImage::try_from(&fc)
            .unwrap_or_else(|_| panic!("failed for transform {transform:?}"));
        assert_eq!(img.width(), expected_width, "width for {transform:?}");
        assert_eq!(img.height(), expected_height, "height for {transform:?}");
    }
}

#[test]
fn frame_guard_drop_destroys_buffer_and_pool() {
    let conn = dummy_conn();
    let buffer: WlBuffer = inert(&conn);
    let shm_pool: WlShmPool = inert(&conn);
    let guard = FrameGuard {
        buffer,
        shm_pool,
        size: Size {
            width: 1,
            height: 1,
        },
        transform: None,
    };
    drop(guard);
}

#[cfg(feature = "dmabuf")]
#[test]
fn dma_frame_guard_drop_destroys_buffer() {
    use crate::screencopy::DMAFrameGuard;

    let conn = dummy_conn();
    let buffer: WlBuffer = inert(&conn);
    let guard = DMAFrameGuard { buffer };
    drop(guard);
}

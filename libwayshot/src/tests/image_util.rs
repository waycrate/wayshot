use crate::image_util::{prepare_mmap_rgba_image, rotate_image_buffer};
use crate::region::Size;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use memmap2::MmapOptions;
use wayland_client::protocol::wl_output::Transform;

fn make_image(w: u32, h: u32) -> DynamicImage {
    let buf: RgbaImage =
        ImageBuffer::from_raw(w, h, (0..w * h * 4).map(|i| i as u8).collect()).unwrap();
    DynamicImage::ImageRgba8(buf)
}

fn make_mmap_image(w: u32, h: u32) -> ImageBuffer<Rgba<u8>, memmap2::MmapMut> {
    let len = (w * h * 4) as usize;
    let mmap = MmapOptions::new().len(len).map_anon().unwrap();
    ImageBuffer::from_raw(w, h, mmap).unwrap()
}

#[test]
fn rotate_image_buffer_normal_unchanged() {
    let image = make_image(10, 20);
    let logical_size = Size {
        width: 10,
        height: 20,
    };
    let out = rotate_image_buffer(image, Transform::Normal, logical_size, 1.0);
    assert_eq!(out.width(), 10);
    assert_eq!(out.height(), 20);
}

#[test]
fn rotate_image_buffer_90_swaps_dimensions() {
    let image = make_image(10, 20);
    let logical_size = Size {
        width: 10,
        height: 20,
    };
    let out = rotate_image_buffer(image, Transform::_90, logical_size, 2.0);
    assert_eq!(out.width(), 20);
    assert_eq!(out.height(), 10);
}

#[test]
fn rotate_image_buffer_180_same_dimensions() {
    let image = make_image(8, 6);
    let logical_size = Size {
        width: 8,
        height: 6,
    };
    let out = rotate_image_buffer(image, Transform::_180, logical_size, 1.0);
    assert_eq!(out.width(), 8);
    assert_eq!(out.height(), 6);
}

#[test]
fn rotate_image_buffer_270_swaps_dimensions() {
    let image = make_image(12, 14);
    let logical_size = Size {
        width: 12,
        height: 14,
    };
    let out = rotate_image_buffer(image, Transform::_270, logical_size, 1.0);
    assert_eq!(out.width(), 14);
    assert_eq!(out.height(), 12);
}

#[test]
fn rotate_image_buffer_flipped_same_dimensions() {
    let image = make_image(5, 5);
    let logical_size = Size {
        width: 5,
        height: 5,
    };
    let out = rotate_image_buffer(image, Transform::Flipped, logical_size, 1.0);
    assert_eq!(out.width(), 5);
    assert_eq!(out.height(), 5);
}

#[test]
fn rotate_image_buffer_flipped90_swaps_dimensions() {
    let image = make_image(10, 20);
    let logical_size = Size {
        width: 20,
        height: 10,
    };
    let out = rotate_image_buffer(image, Transform::Flipped90, logical_size, 1.0);
    assert_eq!(out.width(), 20);
    assert_eq!(out.height(), 10);
}

#[test]
fn rotate_image_buffer_flipped180_same_dimensions() {
    let image = make_image(8, 6);
    let logical_size = Size {
        width: 8,
        height: 6,
    };
    let out = rotate_image_buffer(image, Transform::Flipped180, logical_size, 1.0);
    assert_eq!(out.width(), 8);
    assert_eq!(out.height(), 6);
}

#[test]
fn rotate_image_buffer_flipped270_swaps_dimensions() {
    let image = make_image(10, 20);
    let logical_size = Size {
        width: 20,
        height: 10,
    };
    let out = rotate_image_buffer(image, Transform::Flipped270, logical_size, 1.0);
    assert_eq!(out.width(), 20);
    assert_eq!(out.height(), 10);
}

#[test]
fn rotate_image_buffer_scales_up_when_scale_ratio_exceeds_current() {
    // Image is 10x10, logical_size is 10x10, but max_scale=2.0 means we expect upscaling
    let image = make_image(10, 10);
    let logical_size = Size {
        width: 10,
        height: 10,
    };
    let out = rotate_image_buffer(image, Transform::_90, logical_size, 2.0);
    // After _90 rotation: 10x10 stays 10x10
    // scale = 10/10 = 1.0, scaling_left = 2.0/1.0 = 2.0 > 1.0 → scale up to 20x20
    assert_eq!(out.width(), 20);
    assert_eq!(out.height(), 20);
}

#[test]
fn prepare_mmap_rgba_image_normal_no_scale_returns_mmap_variant() {
    use crate::image_util::PreparedImage;
    let image = make_mmap_image(10, 10);
    let logical_size = Size {
        width: 10,
        height: 10,
    };
    let result = prepare_mmap_rgba_image(image, Transform::Normal, logical_size, 1.0);
    assert!(matches!(result, PreparedImage::RgbaMmap(_)));
}

#[test]
fn prepare_mmap_rgba_image_normal_with_scale_returns_dynamic_variant() {
    use crate::image_util::PreparedImage;
    let image = make_mmap_image(10, 10);
    // max_scale=2.0, current image scale=1.0 → scaling_left=2.0 → needs resize
    let logical_size = Size {
        width: 10,
        height: 10,
    };
    let result = prepare_mmap_rgba_image(image, Transform::Normal, logical_size, 2.0);
    assert!(matches!(result, PreparedImage::Dynamic(_)));
}

#[test]
fn prepare_mmap_rgba_image_rotated_90_returns_dynamic_variant() {
    use crate::image_util::PreparedImage;
    let image = make_mmap_image(10, 20);
    // After _90 rotation width becomes 20, logical_size.width=20 → scale=1.0
    let logical_size = Size {
        width: 20,
        height: 10,
    };
    let result = prepare_mmap_rgba_image(image, Transform::_90, logical_size, 1.0);
    assert!(matches!(result, PreparedImage::Dynamic(_)));
}

#[test]
fn prepare_mmap_rgba_image_flipped_returns_dynamic_variant() {
    use crate::image_util::PreparedImage;
    let image = make_mmap_image(8, 6);
    let logical_size = Size {
        width: 8,
        height: 6,
    };
    let result = prepare_mmap_rgba_image(image, Transform::Flipped, logical_size, 1.0);
    assert!(matches!(result, PreparedImage::Dynamic(_)));
}

use crate::image_util::resize_image_buffer;
use crate::region::Size;
use image::{DynamicImage, RgbaImage};

fn make_image(width: u32, height: u32) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::new(width, height))
}

#[test]
fn no_scaling_needed_returns_same_size() {
    let image = make_image(4, 4);
    let logical_size = Size {
        width: 4,
        height: 4,
    };
    let resized = resize_image_buffer(image, logical_size, 1.0);
    assert_eq!(resized.width(), 4);
    assert_eq!(resized.height(), 4);
}

#[test]
fn large_scaling_left_uses_lanczos3_branch() {
    // image width 4, logical width 2 => scale 2.0; max_scale 4.0 => scaling_left 2.0 (>= threshold).
    let image = make_image(4, 4);
    let logical_size = Size {
        width: 2,
        height: 2,
    };
    let resized = resize_image_buffer(image, logical_size, 4.0);
    assert_eq!(resized.width(), 8);
    assert_eq!(resized.height(), 8);
}

#[test]
fn integer_dpi_uses_triangle_branch() {
    // image width 6, logical width 4 => scale 1.5; max_scale 2.0 (integer) => scaling_left 1.333.
    let image = make_image(6, 6);
    let logical_size = Size {
        width: 4,
        height: 4,
    };
    let resized = resize_image_buffer(image, logical_size, 2.0);
    assert_eq!(resized.width(), 8);
    assert_eq!(resized.height(), 8);
}

#[test]
fn non_integer_dpi_uses_catmull_rom_branch() {
    // image width 6, logical width 4 => scale 1.5; max_scale 1.75 (non-integer) => scaling_left 1.1666...
    let image = make_image(6, 6);
    let logical_size = Size {
        width: 4,
        height: 4,
    };
    let resized = resize_image_buffer(image, logical_size, 1.75);
    assert_eq!(resized.width(), 7);
    assert_eq!(resized.height(), 7);
}

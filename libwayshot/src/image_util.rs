use crate::region::Size;
use image::{DynamicImage, imageops::FilterType};

/// When we still need to upscale this much to align with the composite (`scaling_left` > 1),
/// prefer a stronger filter — e.g. mixed-DPI layouts with a large correction factor.
const SCALING_LEFT_THRESHOLD: f64 = 2.0;

fn resize_filter_for_scale(max_scale: f64, scaling_left: f64) -> FilterType {
    if scaling_left >= SCALING_LEFT_THRESHOLD {
        return FilterType::Lanczos3;
    }
    let is_integer_dpi = (max_scale - max_scale.round()).abs() < 1e-3;
    if is_integer_dpi {
        FilterType::Triangle
    } else {
        FilterType::CatmullRom
    }
}

fn scaling_left(rotated_width: u32, logical_size: Size, max_scale: f64) -> f64 {
    tracing::trace!(
        "Rotated width: {rotated_width}, logical width: {}",
        logical_size.width
    );
    let scale = rotated_width as f64 / logical_size.width as f64;
    let scaling_left = max_scale / scale;
    tracing::debug!("Current scale: {scale}, scaling left (max/current): {scaling_left}");
    scaling_left
}

#[tracing::instrument(skip(image))]
pub(crate) fn resize_image_buffer(
    image: DynamicImage,
    logical_size: Size,
    max_scale: f64,
) -> DynamicImage {
    let scaling_left = scaling_left(image.width(), logical_size, max_scale);
    if scaling_left <= 1.0 {
        tracing::debug!("No scaling left to do");
        return image;
    }

    let new_width = (image.width() as f64 * scaling_left).round() as u32;
    let new_height = (image.height() as f64 * scaling_left).round() as u32;
    let filter = resize_filter_for_scale(max_scale, scaling_left);
    tracing::debug!("Resizing image to {new_width}x{new_height} with {filter:?}");
    image::imageops::resize(&image, new_width, new_height, filter).into()
}

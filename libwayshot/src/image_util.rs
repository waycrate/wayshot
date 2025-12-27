use image::DynamicImage;
use wayland_client::protocol::wl_output::Transform;

use crate::region::Size;

#[tracing::instrument(skip(image))]
pub(crate) fn rotate_image_buffer(
    image: DynamicImage,
    transform: Transform,
    // Includes transform already.
    logical_size: Size,
    max_scale: f64,
) -> DynamicImage {
    let rotated_image = match transform {
        Transform::_90 => image::imageops::rotate90(&image).into(),
        Transform::_180 => image::imageops::rotate180(&image).into(),
        Transform::_270 => image::imageops::rotate270(&image).into(),
        Transform::Flipped => image::imageops::flip_horizontal(&image).into(),
        Transform::Flipped90 => {
            let flipped_buffer = image::imageops::flip_horizontal(&image);
            image::imageops::rotate90(&flipped_buffer).into()
        }
        Transform::Flipped180 => {
            let flipped_buffer = image::imageops::flip_horizontal(&image);
            image::imageops::rotate180(&flipped_buffer).into()
        }
        Transform::Flipped270 => {
            let flipped_buffer = image::imageops::flip_horizontal(&image);
            image::imageops::rotate270(&flipped_buffer).into()
        }
        _ => image,
    };

    let rotated_width = rotated_image.width();
    tracing::trace!(
        "Rotated width: {rotated_width}, logical width: {}",
        logical_size.width
    );
    let scale = rotated_width as f64 / logical_size.width as f64;
    // The amount of scaling left to perform.
    let scaling_left = max_scale / scale;
    tracing::debug!("Current scale: {scale}, scaling left (max/current): {scaling_left}");
    if scaling_left <= 1.0 {
        tracing::debug!("No scaling left to do");
        return rotated_image;
    }

    let new_width = (rotated_image.width() as f64 * scaling_left).round() as u32;
    let new_height = (rotated_image.height() as f64 * scaling_left).round() as u32;
    tracing::debug!("Resizing image to {new_width}x{new_height}");
    image::imageops::resize(
        &rotated_image,
        new_width,
        new_height,
        image::imageops::FilterType::Gaussian,
    )
    .into()
}

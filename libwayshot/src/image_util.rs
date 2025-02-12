use image::DynamicImage;
use wayland_client::protocol::wl_output::Transform;

use crate::region::Size;

#[tracing::instrument(skip(image))]
pub(crate) fn rotate_image_buffer(
    image: DynamicImage,
    transform: Transform,
    logical_size: Size,
    max_scale: f64,
) -> DynamicImage {
    // TODO Better document whether width and height are before or after the transform.
    // Perhaps this should be part of a cleanup of the FrameCopy struct.
    let (logical_width, _logical_height) = match transform {
        Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => {
            (logical_size.height, logical_size.width)
        }
        _ => (logical_size.width, logical_size.height),
    };
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

    let scale = rotated_image.width() as f64 / logical_width as f64;
    // The amount of scaling left to perform.
    let scaling_left = max_scale / scale;
    if scaling_left <= 1.0 {
        tracing::debug!("No scaling left to do");
        return rotated_image;
    }

    tracing::debug!("Scaling left to do: {scaling_left}");
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

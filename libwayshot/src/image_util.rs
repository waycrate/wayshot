use image::{DynamicImage, GenericImageView};
use wayland_client::protocol::wl_output::Transform;

pub(crate) fn rotate_image_buffer(
    image: DynamicImage,
    transform: Transform,
    width: u32,
    height: u32,
) -> DynamicImage {
    // TODO Better document whether width and height are before or after the transform.
    // Perhaps this should be part of a cleanup of the FrameCopy struct.
    let (width, height) = match transform {
        Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => {
            (height, width)
        }
        _ => (width, height),
    };
    let final_image = match transform {
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

    if final_image.dimensions() == (width, height) {
        return final_image;
    }

    image::imageops::resize(
        &final_image,
        width,
        height,
        image::imageops::FilterType::Gaussian,
    )
    .into()
}

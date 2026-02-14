use image::{
    DynamicImage, ImageBuffer, Rgba,
    imageops::{FilterType, replace},
};
use memmap2::MmapMut;
use wayland_client::protocol::wl_output::Transform;

use crate::region::Size;

pub(crate) enum PreparedImage {
    Dynamic(DynamicImage),
    RgbaMmap(ImageBuffer<Rgba<u8>, MmapMut>),
}

impl PreparedImage {
    pub(crate) fn replace_into(self, composite_image: &mut DynamicImage, x: i64, y: i64) {
        match self {
            Self::Dynamic(image) => replace(composite_image, &image, x, y),
            Self::RgbaMmap(image) => replace(composite_image, &image, x, y),
        }
    }
}

fn transformed_width(width: u32, height: u32, transform: Transform) -> u32 {
    match transform {
        Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => height,
        _ => width,
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

    let scaling_left = scaling_left(rotated_image.width(), logical_size, max_scale);
    if scaling_left <= 1.0 {
        tracing::debug!("No scaling left to do");
        return rotated_image;
    }

    let new_width = (rotated_image.width() as f64 * scaling_left).round() as u32;
    let new_height = (rotated_image.height() as f64 * scaling_left).round() as u32;
    tracing::debug!("Resizing image to {new_width}x{new_height}");
    image::imageops::resize(&rotated_image, new_width, new_height, FilterType::Gaussian).into()
}

#[tracing::instrument(skip(image))]
pub(crate) fn prepare_mmap_rgba_image(
    image: ImageBuffer<Rgba<u8>, MmapMut>,
    transform: Transform,
    // Includes transform already.
    logical_size: Size,
    max_scale: f64,
) -> PreparedImage {
    let scaling_left = scaling_left(
        transformed_width(image.width(), image.height(), transform),
        logical_size,
        max_scale,
    );

    if transform == Transform::Normal {
        if scaling_left <= 1.0 {
            tracing::debug!("No transform or scaling left to do");
            return PreparedImage::RgbaMmap(image);
        }

        let new_width = (image.width() as f64 * scaling_left).round() as u32;
        let new_height = (image.height() as f64 * scaling_left).round() as u32;
        tracing::debug!("Resizing image to {new_width}x{new_height}");
        return PreparedImage::Dynamic(
            image::imageops::resize(&image, new_width, new_height, FilterType::Gaussian).into(),
        );
    }

    let rotated_image = match transform {
        Transform::_90 => image::imageops::rotate90(&image),
        Transform::_180 => image::imageops::rotate180(&image),
        Transform::_270 => image::imageops::rotate270(&image),
        Transform::Flipped => image::imageops::flip_horizontal(&image),
        Transform::Flipped90 => {
            let flipped_buffer = image::imageops::flip_horizontal(&image);
            image::imageops::rotate90(&flipped_buffer)
        }
        Transform::Flipped180 => {
            let flipped_buffer = image::imageops::flip_horizontal(&image);
            image::imageops::rotate180(&flipped_buffer)
        }
        Transform::Flipped270 => {
            let flipped_buffer = image::imageops::flip_horizontal(&image);
            image::imageops::rotate270(&flipped_buffer)
        }
        _ => unreachable!("Transform::Normal handled earlier"),
    };

    if scaling_left <= 1.0 {
        tracing::debug!("No scaling left to do");
        return PreparedImage::Dynamic(rotated_image.into());
    }

    let new_width = (rotated_image.width() as f64 * scaling_left).round() as u32;
    let new_height = (rotated_image.height() as f64 * scaling_left).round() as u32;
    tracing::debug!("Resizing image to {new_width}x{new_height}");
    PreparedImage::Dynamic(
        image::imageops::resize(&rotated_image, new_width, new_height, FilterType::Gaussian).into(),
    )
}

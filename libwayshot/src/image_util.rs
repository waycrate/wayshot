use image::{GenericImageView, ImageBuffer, Pixel};
use wayland_client::protocol::wl_output::Transform;

pub(crate) fn rotate_image_buffer<T: GenericImageView>(
    image: &T,
    transform: Transform,
    width: u32,
    height: u32,
) -> ImageBuffer<T::Pixel, Vec<<T::Pixel as Pixel>::Subpixel>>
where
    T::Pixel: 'static,
    <T::Pixel as Pixel>::Subpixel: 'static,
{
    let final_buffer: ImageBuffer<
        <T as GenericImageView>::Pixel,
        Vec<<<T as GenericImageView>::Pixel as Pixel>::Subpixel>,
    > = match transform {
        Transform::_90 => image::imageops::rotate90(image),
        Transform::_180 => image::imageops::rotate180(image),
        Transform::_270 => image::imageops::rotate270(image),
        Transform::Flipped => image::imageops::flip_horizontal(image),
        Transform::Flipped90 => {
            let flipped_buffer = image::imageops::flip_horizontal(image);
            image::imageops::rotate90(&flipped_buffer)
        }
        Transform::Flipped180 => {
            let flipped_buffer = image::imageops::flip_horizontal(image);
            image::imageops::rotate180(&flipped_buffer)
        }
        Transform::Flipped270 => {
            let flipped_buffer = image::imageops::flip_horizontal(image);
            image::imageops::rotate270(&flipped_buffer)
        }
        _ => {
            let final_buffer = image::imageops::resize(
                image,
                width,
                height,
                image::imageops::FilterType::Gaussian,
            );
            return final_buffer;
        }
    };

    image::imageops::resize(
        &final_buffer,
        width,
        height,
        image::imageops::FilterType::Gaussian,
    )
}

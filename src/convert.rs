use image::ColorType;
use wayland_client::protocol::wl_shm;

pub trait ConvertInPlace {
    /// Convert raw image data into output type, return said type
    fn convert_in_place(&self, data: &mut [u8]) -> ColorType;
}

pub trait ConvertCopy {
    /// Convert raw image data into output type, return said type
    fn convert_copy(&self, data: &[u8]) -> (ColorType, Vec<u8>);
}

#[derive(Default)]
struct ConvertBGR10 {}

#[derive(Default)]
struct ConvertNone {}

#[derive(Default)]
struct ConvertRGB8 {}

#[derive(Default)]
struct ConvertRGB565 {}

const SHIFT10BITS_1: u32 = 20;
const SHIFT10BITS_2: u32 = 10;

/// Creates format converter based of input format, return None if conversion
/// isn't possible. Conversion is happening inplace.
pub fn create_in_place_converter(format: wl_shm::Format) -> Option<Box<dyn ConvertInPlace>> {
    match format {
        wl_shm::Format::Xbgr8888 | wl_shm::Format::Abgr8888 => {
            Some(Box::new(ConvertNone::default()))
        }
        wl_shm::Format::Xrgb8888 | wl_shm::Format::Argb8888 => {
            Some(Box::new(ConvertRGB8::default()))
        }
        wl_shm::Format::Xbgr2101010 | wl_shm::Format::Abgr2101010 => {
            Some(Box::new(ConvertBGR10::default()))
        }
        _ => None,
    }
}

pub fn create_copy_converter(format: wl_shm::Format) -> Option<Box<dyn ConvertCopy>> {
    match format {
        wl_shm::Format::Rgb565 => Some(Box::new(ConvertRGB565::default())),
        _ => None,
    }
}

impl ConvertInPlace for ConvertNone {
    fn convert_in_place(&self, _data: &mut [u8]) -> ColorType {
        ColorType::Rgba8
    }
}

impl ConvertInPlace for ConvertRGB8 {
    fn convert_in_place(&self, data: &mut [u8]) -> ColorType {
        for chunk in data.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
        ColorType::Rgba8
    }
}

/// Simple conversion from 10 to 8 bits for one channel
fn convert10_to_8(color: u32) -> u8 {
    ((color >> 2) & 255) as u8
}

impl ConvertInPlace for ConvertBGR10 {
    fn convert_in_place(&self, data: &mut [u8]) -> ColorType {
        for chunk in data.chunks_exact_mut(4) {
            let pixel = ((chunk[3] as u32) << 24)
                | ((chunk[2] as u32) << 16)
                | ((chunk[1] as u32) << 8)
                | chunk[0] as u32;
            let r = convert10_to_8(pixel >> SHIFT10BITS_1);
            let g = convert10_to_8(pixel >> SHIFT10BITS_2);
            let b = convert10_to_8(pixel);
            chunk[0] = b;
            chunk[1] = g;
            chunk[2] = r;
            chunk[3] = 255;
        }
        ColorType::Rgba8
    }
}

impl ConvertCopy for ConvertRGB565 {
    fn convert_copy(&self, data: &[u8]) -> (ColorType, Vec<u8>) {
        let mut out = Vec::with_capacity(2 * data.len());
        for chunk in data.chunks_exact(2) {
            let src = (chunk[1] as u16) << 8 | (chunk[0] as u16);
            let r5 = ((src >> 11) & 0x1f) as u8;
            let g6 = ((src >> 5) & 0x3f) as u8;
            let b5 = (src & 0x1f) as u8;
            let r8 = (r5 << 3) | (r5 >> 2);
            let g8 = (g6 << 2) | (g6 >> 4);
            let b8 = (b5 << 3) | (b5 >> 2);
            let dst = [r8, g8, b8, 255];
            out.extend_from_slice(&dst);
        }
        (ColorType::Rgba8, out)
    }
}

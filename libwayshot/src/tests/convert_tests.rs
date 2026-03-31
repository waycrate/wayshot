use crate::convert::{Convert, create_converter};
use image::ColorType;
use wayland_client::protocol::wl_shm;

#[test]
fn create_converter_returns_none_for_unknown_format() {
    // Argb2101010 is not in the supported list (we support Abgr2101010, Xbgr2101010)
    let unsupported = wl_shm::Format::Argb2101010;
    assert!(create_converter(unsupported).is_none());
}

#[test]
fn create_converter_xbgr8888_returns_some() {
    assert!(create_converter(wl_shm::Format::Xbgr8888).is_some());
    assert!(create_converter(wl_shm::Format::Abgr8888).is_some());
}

#[test]
fn create_converter_xrgb8888_returns_some() {
    assert!(create_converter(wl_shm::Format::Xrgb8888).is_some());
    assert!(create_converter(wl_shm::Format::Argb8888).is_some());
}

#[test]
fn create_converter_bgr10_returns_some() {
    assert!(create_converter(wl_shm::Format::Xbgr2101010).is_some());
    assert!(create_converter(wl_shm::Format::Abgr2101010).is_some());
}

#[test]
fn create_converter_bgr888_returns_some() {
    assert!(create_converter(wl_shm::Format::Bgr888).is_some());
}

#[test]
fn convert_none_produces_rgba8() {
    let converter: Box<dyn Convert> = create_converter(wl_shm::Format::Xbgr8888).unwrap();
    let mut data = vec![0x11, 0x22, 0x33, 0x44];
    let out = converter.convert_inplace(&mut data);
    assert_eq!(out, ColorType::Rgba8);
    assert_eq!(data, vec![0x11, 0x22, 0x33, 0x44]);
}

#[test]
fn convert_rgb8_swaps_r_and_b() {
    let converter = create_converter(wl_shm::Format::Xrgb8888).unwrap();
    let mut data = vec![0x11, 0x22, 0x33, 0x44];
    let out = converter.convert_inplace(&mut data);
    assert_eq!(out, ColorType::Rgba8);
    assert_eq!(data[0], 0x33);
    assert_eq!(data[1], 0x22);
    assert_eq!(data[2], 0x11);
    assert_eq!(data[3], 0x44);
}

#[test]
fn convert_rgb8_multiple_pixels() {
    let converter = create_converter(wl_shm::Format::Argb8888).unwrap();
    let mut data = vec![0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd];
    converter.convert_inplace(&mut data);
    assert_eq!(data[0], 0x33);
    assert_eq!(data[2], 0x11);
    assert_eq!(data[4], 0xcc);
    assert_eq!(data[6], 0xaa);
}

#[test]
fn convert_bgr10_produces_rgba8() {
    let converter = create_converter(wl_shm::Format::Abgr2101010).unwrap();
    let mut data = vec![0x00, 0x00, 0x00, 0xFF];
    let out = converter.convert_inplace(&mut data);
    assert_eq!(out, ColorType::Rgba8);
    assert_eq!(data[3], 255);
}

#[test]
fn convert_bgr888_produces_rgb8() {
    let converter = create_converter(wl_shm::Format::Bgr888).unwrap();
    let mut data = vec![0x01, 0x02, 0x03];
    let out = converter.convert_inplace(&mut data);
    assert_eq!(out, ColorType::Rgb8);
}

#[test]
fn convert_bgr10_black_pixel_gives_black_output() {
    let converter = create_converter(wl_shm::Format::Abgr2101010).unwrap();
    // All-zero pixel: R=0, G=0, B=0, A=0
    let mut data = vec![0x00u8, 0x00, 0x00, 0x00];
    converter.convert_inplace(&mut data);
    assert_eq!(data[0], 0); // B
    assert_eq!(data[1], 0); // G
    assert_eq!(data[2], 0); // R
    assert_eq!(data[3], 255); // A always 255
}

#[test]
fn convert_bgr10_max_alpha_does_not_affect_output_alpha() {
    let converter = create_converter(wl_shm::Format::Xbgr2101010).unwrap();
    // Full alpha (top 2 bits), zero RGB
    let mut data = vec![0x00u8, 0x00, 0x00, 0xC0];
    converter.convert_inplace(&mut data);
    assert_eq!(data[3], 255); // alpha is always forced to 255
}

#[test]
fn convert_rgb8_empty_data_no_panic() {
    let converter = create_converter(wl_shm::Format::Argb8888).unwrap();
    let mut data: Vec<u8> = vec![];
    converter.convert_inplace(&mut data); // should not panic
    assert!(data.is_empty());
}

#[test]
fn convert_none_empty_data_no_panic() {
    let converter = create_converter(wl_shm::Format::Xbgr8888).unwrap();
    let mut data: Vec<u8> = vec![];
    converter.convert_inplace(&mut data);
    assert!(data.is_empty());
}

use crate::clipboard::mime_type_for_encoding;
use crate::utils::EncodingFormat;
use wl_clipboard_rs::copy::MimeType;

fn mime_str(encoding: EncodingFormat) -> String {
    match mime_type_for_encoding(encoding) {
        MimeType::Specific(s) => s,
        other => panic!("expected a specific mime type, got {other:?}"),
    }
}

#[test]
fn png_maps_to_image_png() {
    assert_eq!(mime_str(EncodingFormat::Png), "image/png");
}

#[cfg(feature = "jpeg")]
#[test]
fn jpeg_maps_to_image_jpeg() {
    assert_eq!(mime_str(EncodingFormat::Jpg), "image/jpeg");
}

#[cfg(feature = "webp")]
#[test]
fn webp_maps_to_image_webp() {
    assert_eq!(mime_str(EncodingFormat::Webp), "image/webp");
}

#[cfg(feature = "qoi")]
#[test]
fn qoi_maps_to_image_qoi() {
    assert_eq!(mime_str(EncodingFormat::Qoi), "image/qoi");
}

#[cfg(feature = "pnm")]
#[test]
fn ppm_maps_to_portable_pixmap() {
    assert_eq!(mime_str(EncodingFormat::Ppm), "image/x-portable-pixmap");
}

#[cfg(feature = "avif")]
#[test]
fn avif_maps_to_image_avif() {
    assert_eq!(mime_str(EncodingFormat::Avif), "image/avif");
}

#[cfg(feature = "jxl")]
#[test]
fn jxl_maps_to_image_jxl() {
    assert_eq!(mime_str(EncodingFormat::Jxl), "image/jxl");
}

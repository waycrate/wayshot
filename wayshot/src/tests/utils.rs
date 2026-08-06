use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::config::{Jxl, Png};
use crate::utils::{
    self, EncodingFormat, get_absolute_path, get_default_file_name, get_expanded_path,
    get_full_file_name, parse_slurp_geometry,
};

// ─── parse_slurp_geometry ───────────────────────────────────────────────────

#[test]
fn parse_slurp_geometry_valid() {
    let region = parse_slurp_geometry("100,200 300x400").expect("should parse");
    assert_eq!(region.inner.position.x, 100);
    assert_eq!(region.inner.position.y, 200);
    assert_eq!(region.inner.size.width, 300);
    assert_eq!(region.inner.size.height, 400);
}

#[test]
fn parse_slurp_geometry_negative_position() {
    let region = parse_slurp_geometry("-10,-20 5x5").expect("should parse");
    assert_eq!(region.inner.position.x, -10);
    assert_eq!(region.inner.position.y, -20);
}

#[test]
fn parse_slurp_geometry_trims_whitespace() {
    let region = parse_slurp_geometry("  1,2 3x4  ").expect("should parse");
    assert_eq!(region.inner.position.x, 1);
    assert_eq!(region.inner.size.width, 3);
}

#[test]
fn parse_slurp_geometry_rejects_empty() {
    assert!(parse_slurp_geometry("").is_err());
    assert!(parse_slurp_geometry("   ").is_err());
}

#[test]
fn parse_slurp_geometry_rejects_missing_size() {
    assert!(parse_slurp_geometry("1,2").is_err());
}

#[test]
fn parse_slurp_geometry_rejects_missing_comma_in_position() {
    assert!(parse_slurp_geometry("1 3x4").is_err());
}

#[test]
fn parse_slurp_geometry_rejects_missing_x_in_size() {
    assert!(parse_slurp_geometry("1,2 3-4").is_err());
}

#[test]
fn parse_slurp_geometry_rejects_non_numeric_fields() {
    assert!(parse_slurp_geometry("a,2 3x4").is_err());
    assert!(parse_slurp_geometry("1,b 3x4").is_err());
    assert!(parse_slurp_geometry("1,2 cx4").is_err());
    assert!(parse_slurp_geometry("1,2 3xd").is_err());
}

#[test]
fn parse_slurp_geometry_rejects_zero_size() {
    assert!(parse_slurp_geometry("1,2 0x4").is_err());
    assert!(parse_slurp_geometry("1,2 4x0").is_err());
}

// ─── waysip_to_region ───────────────────────────────────────────────────────

#[cfg(any(feature = "selector", feature = "color_picker"))]
#[test]
fn waysip_to_region_converts_positive_size() {
    let size = libwaysip::Size {
        width: 100,
        height: 50,
    };
    let position = libwaysip::Position { x: 5, y: -5 };
    let region = utils::waysip_to_region(size, position).expect("valid size should convert");
    assert_eq!(region.inner.size.width, 100);
    assert_eq!(region.inner.size.height, 50);
    assert_eq!(region.inner.position.x, 5);
    assert_eq!(region.inner.position.y, -5);
}

#[cfg(any(feature = "selector", feature = "color_picker"))]
#[test]
fn waysip_to_region_rejects_negative_width() {
    let size = libwaysip::Size {
        width: -1,
        height: 50,
    };
    let position = libwaysip::Position { x: 0, y: 0 };
    assert!(utils::waysip_to_region(size, position).is_err());
}

#[cfg(any(feature = "selector", feature = "color_picker"))]
#[test]
fn waysip_to_region_rejects_negative_height() {
    let size = libwaysip::Size {
        width: 10,
        height: -1,
    };
    let position = libwaysip::Position { x: 0, y: 0 };
    assert!(utils::waysip_to_region(size, position).is_err());
}

// ─── path helpers ───────────────────────────────────────────────────────────

#[test]
fn get_absolute_path_keeps_absolute_paths() {
    assert_eq!(
        get_absolute_path(Path::new("/tmp/foo")),
        PathBuf::from("/tmp/foo")
    );
}

#[test]
fn get_absolute_path_joins_relative_to_cwd() {
    let resolved = get_absolute_path(Path::new("foo.png"));
    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("foo.png"));
}

#[test]
fn get_expanded_path_leaves_plain_paths_untouched() {
    assert_eq!(
        get_expanded_path(Path::new("/tmp/foo")),
        PathBuf::from("/tmp/foo")
    );
}

#[test]
fn get_expanded_path_falls_back_to_cwd_on_unresolved_var() {
    let resolved = get_expanded_path(Path::new("$WAYSHOT_TEST_DEFINITELY_UNSET_VARIABLE_XYZ/foo"));
    assert_eq!(resolved, std::env::current_dir().unwrap_or_default());
}

#[test]
fn get_default_file_name_appends_encoding_extension() {
    let name = get_default_file_name("literal-name", EncodingFormat::Png);
    assert_eq!(name, PathBuf::from("literal-name.png"));
}

#[test]
fn get_full_file_name_for_directory_uses_default_name() {
    let dir = std::env::temp_dir();
    let full = get_full_file_name(&dir, "wayshot-test-literal", EncodingFormat::Png);
    assert_eq!(full, dir.join("wayshot-test-literal.png"));
}

#[test]
fn get_full_file_name_for_file_path_replaces_extension() {
    let full = get_full_file_name(
        Path::new("/tmp/some-nonexistent-file.jpg"),
        "ignored-format",
        EncodingFormat::Png,
    );
    assert_eq!(full, PathBuf::from("/tmp/some-nonexistent-file.png"));
}

// ─── EncodingFormat ─────────────────────────────────────────────────────────

#[test]
fn encoding_format_from_str_png() {
    assert_eq!(
        EncodingFormat::from_str("png").unwrap(),
        EncodingFormat::Png
    );
}

#[test]
fn encoding_format_from_str_rejects_unknown() {
    assert!(EncodingFormat::from_str("bogus").is_err());
}

#[test]
fn encoding_format_display_round_trips_through_from_str() {
    let displayed = EncodingFormat::Png.to_string();
    assert_eq!(displayed, "png");
    assert_eq!(
        EncodingFormat::from_str(&displayed).unwrap(),
        EncodingFormat::Png
    );
}

#[test]
fn encoding_format_try_from_pathbuf_no_extension_errors() {
    let path = PathBuf::from("no_extension_here");
    let result: Result<EncodingFormat, _> = (&path).try_into();
    assert!(result.is_err());
}

#[test]
fn encoding_format_try_from_pathbuf_valid_extension() {
    let path = PathBuf::from("shot.png");
    let result: Result<EncodingFormat, _> = (&path).try_into();
    assert_eq!(result.unwrap(), EncodingFormat::Png);
}

#[test]
fn encoding_format_try_from_pathbuf_unsupported_extension_errors() {
    let path = PathBuf::from("shot.txt");
    let result: Result<EncodingFormat, _> = (&path).try_into();
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn encoding_format_try_from_pathbuf_non_utf8_extension_errors() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // 0xFF is not valid UTF-8 in any position.
    let path = PathBuf::from(OsStr::from_bytes(b"shot.\xff\xff"));
    let result: Result<EncodingFormat, _> = (&path).try_into();
    assert!(result.is_err());
}

#[test]
fn encoding_format_into_image_format() {
    let fmt: image::ImageFormat = EncodingFormat::Png.into();
    assert_eq!(fmt, image::ImageFormat::Png);
}

#[cfg(feature = "jpeg")]
#[test]
fn encoding_format_jpeg_variants() {
    assert_eq!(
        EncodingFormat::from_str("jpg").unwrap(),
        EncodingFormat::Jpg
    );
    assert_eq!(
        EncodingFormat::from_str("jpeg").unwrap(),
        EncodingFormat::Jpg
    );
    assert_eq!(Into::<&str>::into(EncodingFormat::Jpg), "jpg");
}

#[cfg(feature = "webp")]
#[test]
fn encoding_format_webp_variant() {
    assert_eq!(
        EncodingFormat::from_str("webp").unwrap(),
        EncodingFormat::Webp
    );
    assert_eq!(Into::<&str>::into(EncodingFormat::Webp), "webp");
    let fmt: image::ImageFormat = EncodingFormat::Webp.into();
    assert_eq!(fmt, image::ImageFormat::WebP);
}

#[cfg(feature = "qoi")]
#[test]
fn encoding_format_qoi_variant() {
    assert_eq!(
        EncodingFormat::from_str("qoi").unwrap(),
        EncodingFormat::Qoi
    );
    assert_eq!(Into::<&str>::into(EncodingFormat::Qoi), "qoi");
    let fmt: image::ImageFormat = EncodingFormat::Qoi.into();
    assert_eq!(fmt, image::ImageFormat::Qoi);
}

#[cfg(feature = "pnm")]
#[test]
fn encoding_format_pnm_variant() {
    assert_eq!(
        EncodingFormat::from_str("ppm").unwrap(),
        EncodingFormat::Ppm
    );
    assert_eq!(Into::<&str>::into(EncodingFormat::Ppm), "ppm");
    let fmt: image::ImageFormat = EncodingFormat::Ppm.into();
    assert_eq!(fmt, image::ImageFormat::Pnm);
}

#[cfg(feature = "avif")]
#[test]
fn encoding_format_avif_variant() {
    assert_eq!(
        EncodingFormat::from_str("avif").unwrap(),
        EncodingFormat::Avif
    );
    assert_eq!(Into::<&str>::into(EncodingFormat::Avif), "avif");
    let fmt: image::ImageFormat = EncodingFormat::Avif.into();
    assert_eq!(fmt, image::ImageFormat::Avif);
}

#[cfg(feature = "jxl")]
#[test]
fn encoding_format_jxl_variant() {
    assert_eq!(
        EncodingFormat::from_str("jxl").unwrap(),
        EncodingFormat::Jxl
    );
    assert_eq!(Into::<&str>::into(EncodingFormat::Jxl), "jxl");
    // Jxl has no `image::ImageFormat` counterpart; encode_image() never routes
    // through this conversion for Jxl, but the fallback arm still exists and
    // should map to something harmless (Png) rather than panicking.
    let fmt: image::ImageFormat = EncodingFormat::Jxl.into();
    assert_eq!(fmt, image::ImageFormat::Png);
}

// ─── encode_image ───────────────────────────────────────────────────────────

fn sample_image() -> image::DynamicImage {
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        4,
        4,
        image::Rgba([10, 20, 30, 255]),
    ))
}

#[test]
fn encode_image_png_round_trips() {
    let image = sample_image();
    let bytes = utils::encode_image(
        &image,
        EncodingFormat::Png,
        &Jxl::default(),
        &Png::default(),
    )
    .expect("png encoding should succeed");
    assert!(!bytes.is_empty());
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .expect("should decode back");
    assert_eq!(decoded.width(), 4);
    assert_eq!(decoded.height(), 4);
}

#[cfg(feature = "jpeg")]
#[test]
fn encode_image_jpeg_produces_bytes() {
    let image = sample_image();
    let bytes = utils::encode_image(
        &image,
        EncodingFormat::Jpg,
        &Jxl::default(),
        &Png::default(),
    )
    .expect("jpeg encoding should succeed");
    assert!(!bytes.is_empty());
}

#[cfg(feature = "jxl")]
#[test]
fn encode_image_jxl_lossless_and_lossy_produce_bytes() {
    let image = sample_image();
    let lossless_jxl = Jxl {
        lossless: Some(true),
        ..Jxl::default()
    };
    let lossless = utils::encode_image(&image, EncodingFormat::Jxl, &lossless_jxl, &Png::default())
        .expect("lossless jxl encoding should succeed");
    assert!(!lossless.is_empty());

    let lossy_jxl = Jxl {
        lossless: Some(false),
        ..Jxl::default()
    };
    let lossy = utils::encode_image(&image, EncodingFormat::Jxl, &lossy_jxl, &Png::default())
        .expect("lossy jxl encoding should succeed");
    assert!(!lossy.is_empty());
}

// ─── shell completions ──────────────────────────────────────────────────────

#[cfg(feature = "completions")]
#[test]
fn print_completions_does_not_panic_for_every_shell() {
    use crate::cli::Shell;
    for shell in [
        Shell::Bash,
        Shell::Elvish,
        Shell::Fish,
        Shell::Pwsh,
        Shell::Zsh,
        Shell::Nushell,
    ] {
        utils::print_completions(shell);
    }
}

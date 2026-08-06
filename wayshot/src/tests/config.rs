use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::config::{Base, Config, Encoding, File, Geometry, Png, PngCompression};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique path under the OS temp dir, so parallel tests don't clash.
fn temp_path(name: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "wayshot-config-test-{}-{}-{}.toml",
        std::process::id(),
        n,
        name
    ))
}

fn write_temp_toml(name: &str, contents: &str) -> PathBuf {
    let path = temp_path(name);
    let mut f = std::fs::File::create(&path).expect("create temp config file");
    f.write_all(contents.as_bytes())
        .expect("write temp config file");
    path
}

// ─── Config::default ───────────────────────────────────────────────────────

#[test]
fn config_default_fills_every_section() {
    let config = Config::default();
    assert!(config.base.is_some());
    assert!(config.file.is_some());
    assert!(config.geometry.is_some());
    assert!(config.encoding.is_some());
    assert!(config.notification.is_some());
}

#[test]
fn base_default_values() {
    let base = Base::default();
    assert_eq!(base.output, None);
    assert_eq!(base.cursor, Some(false));
    assert_eq!(base.freeze, Some(true));
    assert_eq!(base.delay, None);
    assert_eq!(base.clipboard, Some(false));
    assert_eq!(base.file, Some(true));
    assert_eq!(base.stdout, Some(false));
    assert_eq!(base.log_level, Some("info".to_string()));
    assert_eq!(base.notifications, Some(true));
}

#[test]
fn file_default_values() {
    let file = File::default();
    assert_eq!(
        file.name_format,
        Some("wayshot-%Y_%m_%d-%H_%M_%S".to_string())
    );
    assert!(file.path.is_some());
}

#[test]
fn geometry_default_values() {
    let geometry = Geometry::default();
    assert_eq!(geometry.foreground_color, Some("#000000ff".to_string()));
    assert_eq!(geometry.background_color, Some("#66666680".to_string()));
}

#[test]
fn encoding_default_has_jxl_and_png() {
    let encoding = Encoding::default();
    assert!(encoding.jxl.is_some());
    assert!(encoding.png.is_some());
}

// ─── Base::get_log_level ───────────────────────────────────────────────────

#[cfg(feature = "logger")]
#[test]
fn get_log_level_maps_known_strings() {
    use tracing::Level;

    let case = |s: &str, expected: Level| {
        let base = Base {
            log_level: Some(s.to_string()),
            ..Base::default()
        };
        assert_eq!(base.get_log_level(), expected);
    };
    case("trace", Level::TRACE);
    case("debug", Level::DEBUG);
    case("info", Level::INFO);
    case("warn", Level::WARN);
    case("error", Level::ERROR);
}

#[cfg(feature = "logger")]
#[test]
fn get_log_level_falls_back_to_info_for_unknown_or_missing() {
    use tracing::Level;

    let unknown = Base {
        log_level: Some("verbose".to_string()),
        ..Base::default()
    };
    assert_eq!(unknown.get_log_level(), Level::INFO);

    let missing = Base {
        log_level: None,
        ..Base::default()
    };
    assert_eq!(missing.get_log_level(), Level::INFO);
}

// ─── Jxl getters ────────────────────────────────────────────────────────────

#[cfg(feature = "jxl")]
#[test]
fn jxl_getters_use_provided_values() {
    use crate::config::Jxl;

    let jxl = Jxl {
        lossless: Some(true),
        distance: Some(2.5),
        effort: Some(9),
    };
    assert!(jxl.get_lossless());
    assert_eq!(jxl.get_distance(), 2.5);
    assert_eq!(jxl.get_effort(), 9);
}

#[cfg(feature = "jxl")]
#[test]
fn jxl_getters_fall_back_to_defaults_when_unset() {
    use crate::config::Jxl;

    let jxl = Jxl {
        lossless: None,
        distance: None,
        effort: None,
    };
    assert!(!jxl.get_lossless());
    assert_eq!(jxl.get_distance(), 1.0);
    assert_eq!(jxl.get_effort(), 7);
}

#[cfg(feature = "jxl")]
#[test]
fn jxl_get_effort_clamps_out_of_range() {
    use crate::config::Jxl;

    let too_low = Jxl {
        effort: Some(0),
        ..Jxl::default()
    };
    assert_eq!(too_low.get_effort(), 1);

    let too_high = Jxl {
        effort: Some(255),
        ..Jxl::default()
    };
    assert_eq!(too_high.get_effort(), 10);
}

// ─── Png getters ────────────────────────────────────────────────────────────

#[test]
fn png_get_compression_named_variants() {
    use image::codecs::png::CompressionType;

    let case = |name: &str, expected: CompressionType| {
        let png = Png {
            compression: Some(PngCompression::Named(name.to_string())),
            filter: None,
        };
        assert_eq!(
            std::mem::discriminant(&png.get_compression()),
            std::mem::discriminant(&expected)
        );
    };
    case("default", CompressionType::Default);
    case("best", CompressionType::Best);
    case("fast", CompressionType::Fast);
    case("none", CompressionType::Uncompressed);
    case("uncompressed", CompressionType::Uncompressed);
    case("something-else", CompressionType::Default);
}

#[test]
fn png_get_compression_level_variants() {
    use image::codecs::png::CompressionType;

    let in_range = Png {
        compression: Some(PngCompression::Level(5)),
        filter: None,
    };
    assert_eq!(
        std::mem::discriminant(&in_range.get_compression()),
        std::mem::discriminant(&CompressionType::Level(5))
    );

    let out_of_range = Png {
        compression: Some(PngCompression::Level(200)),
        filter: None,
    };
    assert_eq!(
        std::mem::discriminant(&out_of_range.get_compression()),
        std::mem::discriminant(&CompressionType::Default)
    );
}

#[test]
fn png_get_compression_defaults_when_unset() {
    use image::codecs::png::CompressionType;

    let png = Png {
        compression: None,
        filter: None,
    };
    assert_eq!(
        std::mem::discriminant(&png.get_compression()),
        std::mem::discriminant(&CompressionType::Default)
    );
}

#[test]
fn png_get_filter_variants() {
    use image::codecs::png::FilterType;

    let case = |name: &str, expected: FilterType| {
        let png = Png {
            compression: None,
            filter: Some(name.to_string()),
        };
        assert_eq!(
            std::mem::discriminant(&png.get_filter()),
            std::mem::discriminant(&expected)
        );
    };
    case("none", FilterType::NoFilter);
    case("sub", FilterType::Sub);
    case("up", FilterType::Up);
    case("avg", FilterType::Avg);
    case("paeth", FilterType::Paeth);
    case("adaptive", FilterType::Adaptive);
    case("unknown", FilterType::Adaptive);
}

#[test]
fn png_get_filter_defaults_when_unset() {
    use image::codecs::png::FilterType;

    let png = Png {
        compression: None,
        filter: None,
    };
    assert_eq!(
        std::mem::discriminant(&png.get_filter()),
        std::mem::discriminant(&FilterType::Adaptive)
    );
}

// ─── Config::load ───────────────────────────────────────────────────────────

#[test]
fn load_returns_none_for_missing_file() {
    let path = temp_path("does-not-exist");
    assert!(Config::load(&path).is_none());
}

#[test]
fn load_returns_none_for_invalid_toml() {
    let path = write_temp_toml("invalid", "this is not [ valid toml");
    let result = Config::load(&path);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_none());
}

#[test]
fn load_parses_a_full_config_file() {
    let toml = r##"
[base]
output = "eDP-1"
cursor = true
freeze = false
delay = 100
clipboard = true
file = true
stdout = false
log_level = "debug"
notifications = false

[file]
path = "/tmp"
name_format = "custom-%s"
encoding = "png"

[geometry]
foreground_color = "#111111ff"
background_color = "#222222ff"
"##;
    let path = write_temp_toml("full", toml);
    let config = Config::load(&path);
    let _ = std::fs::remove_file(&path);
    let config = config.expect("valid config should parse");

    let base = config.base.expect("base section present");
    assert_eq!(base.output.as_deref(), Some("eDP-1"));
    assert_eq!(base.cursor, Some(true));
    assert_eq!(base.freeze, Some(false));
    assert_eq!(base.delay, Some(100));
    assert_eq!(base.clipboard, Some(true));
    assert_eq!(base.notifications, Some(false));

    let file = config.file.expect("file section present");
    assert_eq!(file.name_format.as_deref(), Some("custom-%s"));

    let geometry = config.geometry.expect("geometry section present");
    assert_eq!(geometry.foreground_color.as_deref(), Some("#111111ff"));
    assert_eq!(geometry.background_color.as_deref(), Some("#222222ff"));
}

#[test]
fn load_parses_a_partial_config_file() {
    // Missing sections should just be None; consumers fall back to defaults.
    let toml = r#"
[base]
cursor = true
"#;
    let path = write_temp_toml("partial", toml);
    let config = Config::load(&path).expect("valid config should parse");
    let _ = std::fs::remove_file(&path);

    assert!(config.base.is_some());
    assert!(config.file.is_none());
    assert!(config.geometry.is_none());
    assert!(config.encoding.is_none());
    assert!(config.notification.is_none());
}

#[test]
fn get_default_path_ends_with_wayshot_config_toml_or_is_empty() {
    // dirs::config_local_dir() may be unavailable in some sandboxed
    // environments; get_default_path() falls back to an empty PathBuf then.
    let path = Config::get_default_path();
    let s = path.to_string_lossy();
    assert!(
        s.is_empty() || s.ends_with("wayshot/config.toml") || s.ends_with("wayshot\\config.toml")
    );
}

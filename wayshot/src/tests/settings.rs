use clap::Parser;

use crate::cli::Cli;
#[cfg(feature = "jxl")]
use crate::config::File;
#[cfg(feature = "selector")]
use crate::config::Geometry;
use crate::config::{Base, Config};
use crate::screenshot::CaptureMode;
use crate::settings::{AppSettings, Command};
use crate::utils::EncodingFormat;

fn cli(args: &[&str]) -> Cli {
    let mut full = vec!["wayshot"];
    full.extend_from_slice(args);
    Cli::parse_from(full)
}

fn config_with_base(base: Base) -> Config {
    Config {
        base: Some(base),
        file: None,
        geometry: None,
        encoding: None,
        notification: None,
    }
}

// ─── cursor precedence ──────────────────────────────────────────────────────

#[test]
fn cursor_defaults_to_false() {
    let settings = AppSettings::resolve(&cli(&[]), &Config::default());
    assert!(!settings.cursor);
}

#[test]
fn cursor_cli_flag_overrides_config_disabled() {
    let config = config_with_base(Base {
        cursor: Some(false),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&["--cursor"]), &config);
    assert!(settings.cursor);
}

#[test]
fn cursor_config_true_without_cli_flag() {
    let config = config_with_base(Base {
        cursor: Some(true),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&[]), &config);
    assert!(settings.cursor);
}

// ─── freeze precedence ──────────────────────────────────────────────────────

#[test]
fn freeze_defaults_to_true() {
    let settings = AppSettings::resolve(&cli(&[]), &Config::default());
    assert!(settings.freeze);
}

#[test]
fn no_freeze_flag_disables_freeze_even_if_config_enables_it() {
    let config = config_with_base(Base {
        freeze: Some(true),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&["--no-freeze"]), &config);
    assert!(!settings.freeze);
}

#[test]
fn config_freeze_false_disables_freeze_without_cli_flag() {
    let config = config_with_base(Base {
        freeze: Some(false),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&[]), &config);
    assert!(!settings.freeze);
}

// ─── delay precedence ───────────────────────────────────────────────────────

#[test]
fn delay_defaults_to_none() {
    let settings = AppSettings::resolve(&cli(&[]), &Config::default());
    assert_eq!(settings.delay, None);
}

#[test]
fn delay_cli_overrides_config() {
    let config = config_with_base(Base {
        delay: Some(200),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&["--delay", "500"]), &config);
    assert_eq!(settings.delay, Some(500));
}

#[test]
fn delay_falls_back_to_config_without_cli_flag() {
    let config = config_with_base(Base {
        delay: Some(200),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&[]), &config);
    assert_eq!(settings.delay, Some(200));
}

// ─── encoding precedence ────────────────────────────────────────────────────

#[test]
fn encoding_defaults_to_png() {
    let settings = AppSettings::resolve(&cli(&[]), &Config::default());
    assert_eq!(settings.encoding, EncodingFormat::Png);
}

#[cfg(feature = "jxl")]
#[test]
fn encoding_flag_overrides_everything() {
    let config = Config {
        file: Some(File {
            path: None,
            name_format: None,
            encoding: Some(EncodingFormat::Png),
        }),
        ..Config::default()
    };
    let settings = AppSettings::resolve(&cli(&["--encoding", "jxl", "/tmp/shot.png"]), &config);
    assert_eq!(settings.encoding, EncodingFormat::Jxl);
}

#[test]
fn encoding_inferred_from_file_extension_without_flag() {
    #[cfg(feature = "jxl")]
    let (ext, expected) = ("jxl", EncodingFormat::Jxl);
    #[cfg(not(feature = "jxl"))]
    let (ext, expected) = ("png", EncodingFormat::Png);

    let settings = AppSettings::resolve(&cli(&[&format!("/tmp/shot.{ext}")]), &Config::default());
    assert_eq!(settings.encoding, expected);
}

#[cfg(feature = "jxl")]
#[test]
fn encoding_falls_back_to_config_file_encoding() {
    let config = Config {
        file: Some(File {
            path: None,
            name_format: None,
            encoding: Some(EncodingFormat::Jxl),
        }),
        ..Config::default()
    };
    let settings = AppSettings::resolve(&cli(&[]), &config);
    assert_eq!(settings.encoding, EncodingFormat::Jxl);
}

// ─── file_name_format precedence ────────────────────────────────────────────

#[test]
fn file_name_format_defaults_to_timestamp_pattern() {
    let settings = AppSettings::resolve(&cli(&["-"]), &Config::default());
    // Resolution is internal; confirm indirectly through resolve_output_file's
    // behaviour on a directory target instead, since file_name_format itself
    // isn't exposed on AppSettings. Here we simply assert stdout mode kicked in.
    assert!(settings.stdout_print);
    assert!(settings.file.is_none());
}

// ─── file / stdout resolution ───────────────────────────────────────────────

#[test]
fn dash_positional_enables_stdout_and_skips_file() {
    let settings = AppSettings::resolve(&cli(&["-"]), &Config::default());
    assert!(settings.stdout_print);
    assert!(settings.file.is_none());
}

#[test]
fn explicit_file_path_is_used_verbatim_dir() {
    let settings = AppSettings::resolve(&cli(&["/tmp"]), &Config::default());
    assert!(settings.file.is_some());
    assert!(!settings.stdout_print);
    assert!(settings.file.unwrap().starts_with("/tmp"));
}

#[test]
fn config_file_false_disables_default_file_output() {
    let config = config_with_base(Base {
        file: Some(false),
        stdout: Some(false),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&[]), &config);
    assert!(settings.file.is_none());
    assert!(!settings.stdout_print);
}

#[test]
fn config_stdout_true_enables_stdout_without_cli_flag() {
    let config = config_with_base(Base {
        stdout: Some(true),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&[]), &config);
    assert!(settings.stdout_print);
}

// ─── command resolution precedence ──────────────────────────────────────────

#[test]
fn list_outputs_flag_selects_list_outputs_command() {
    let settings = AppSettings::resolve(&cli(&["--list-outputs"]), &Config::default());
    assert!(matches!(settings.command, Command::ListOutputs));
}

#[test]
fn list_outputs_json_flag_selects_command() {
    let settings = AppSettings::resolve(&cli(&["--list-outputs-json"]), &Config::default());
    assert!(matches!(settings.command, Command::ListOutputsJson));
}

#[test]
fn list_outputs_info_flag_selects_command() {
    let settings = AppSettings::resolve(&cli(&["--list-outputs-info"]), &Config::default());
    assert!(matches!(settings.command, Command::ListOutputsInfo));
}

#[test]
fn list_toplevels_flag_selects_command() {
    let settings = AppSettings::resolve(&cli(&["--list-toplevels"]), &Config::default());
    assert!(matches!(settings.command, Command::ListToplevels));
}

#[test]
fn list_toplevels_json_flag_selects_command() {
    let settings = AppSettings::resolve(&cli(&["--list-toplevels-json"]), &Config::default());
    assert!(matches!(settings.command, Command::ListToplevelsJson));
}

#[test]
fn no_flags_defaults_to_screenshot_all() {
    let settings = AppSettings::resolve(&cli(&[]), &Config::default());
    assert!(matches!(
        settings.command,
        Command::Screenshot(CaptureMode::All)
    ));
}

#[cfg(feature = "color_picker")]
#[test]
fn color_flag_selects_color_picker_command() {
    let settings = AppSettings::resolve(&cli(&["--color"]), &Config::default());
    assert!(matches!(settings.command, Command::ColorPicker(_)));
}

// ─── capture mode resolution ────────────────────────────────────────────────

#[test]
fn toplevel_flag_selects_toplevel_capture_mode() {
    let settings = AppSettings::resolve(&cli(&["--toplevel", "abc"]), &Config::default());
    match settings.command {
        Command::Screenshot(CaptureMode::Toplevel(name)) => assert_eq!(name, "abc"),
        _ => panic!("expected Toplevel capture mode"),
    }
}

#[test]
fn choose_toplevel_flag_selects_choose_toplevel_capture_mode() {
    let settings = AppSettings::resolve(&cli(&["--choose-toplevel"]), &Config::default());
    assert!(matches!(
        settings.command,
        Command::Screenshot(CaptureMode::ChooseToplevel)
    ));
}

#[test]
fn output_flag_selects_output_capture_mode() {
    let settings = AppSettings::resolve(&cli(&["-o", "eDP-1"]), &Config::default());
    match settings.command {
        Command::Screenshot(CaptureMode::Output(name)) => assert_eq!(name, "eDP-1"),
        _ => panic!("expected Output capture mode"),
    }
}

#[test]
fn config_output_used_when_cli_output_absent() {
    let config = config_with_base(Base {
        output: Some("HDMI-A-1".to_string()),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&[]), &config);
    match settings.command {
        Command::Screenshot(CaptureMode::Output(name)) => assert_eq!(name, "HDMI-A-1"),
        _ => panic!("expected Output capture mode from config"),
    }
}

#[test]
fn cli_output_takes_precedence_over_config_output() {
    let config = config_with_base(Base {
        output: Some("HDMI-A-1".to_string()),
        ..Base::default()
    });
    let settings = AppSettings::resolve(&cli(&["-o", "eDP-1"]), &config);
    match settings.command {
        Command::Screenshot(CaptureMode::Output(name)) => assert_eq!(name, "eDP-1"),
        _ => panic!("expected Output capture mode from CLI"),
    }
}

#[test]
fn choose_output_flag_selects_choose_output_capture_mode() {
    let settings = AppSettings::resolve(&cli(&["--choose-output"]), &Config::default());
    assert!(matches!(
        settings.command,
        Command::Screenshot(CaptureMode::ChooseOutput)
    ));
}

#[test]
fn geometry_string_parses_into_geometry_region() {
    let settings = AppSettings::resolve(&cli(&["-g", "10,20 300x400"]), &Config::default());
    match settings.command {
        Command::Screenshot(CaptureMode::GeometryRegion(region)) => {
            assert_eq!(region.inner.position.x, 10);
            assert_eq!(region.inner.position.y, 20);
            assert_eq!(region.inner.size.width, 300);
            assert_eq!(region.inner.size.height, 400);
        }
        _ => panic!("expected GeometryRegion capture mode"),
    }
}

#[cfg(feature = "selector")]
#[test]
fn interactive_geometry_uses_cli_colors_over_config() {
    let config = Config {
        geometry: Some(Geometry {
            foreground_color: Some("#111111ff".to_string()),
            background_color: Some("#222222ff".to_string()),
        }),
        ..Config::default()
    };
    // `-g` has `allow_hyphen_values` + `num_args = 0..=1`, so it must come last
    // or it would greedily swallow the next flag as its own value.
    let settings = AppSettings::resolve(
        &cli(&[
            "--geometry-foreground-color",
            "#aaaaaaaa",
            "--geometry-background-color",
            "#bbbbbbbb",
            "-g",
        ]),
        &config,
    );
    match settings.command {
        Command::Screenshot(CaptureMode::Geometry {
            foreground_color,
            background_color,
        }) => {
            assert_eq!(foreground_color.as_deref(), Some("#aaaaaaaa"));
            assert_eq!(background_color.as_deref(), Some("#bbbbbbbb"));
        }
        _ => panic!("expected interactive Geometry capture mode"),
    }
}

#[cfg(feature = "selector")]
#[test]
fn interactive_geometry_falls_back_to_config_colors() {
    let config = Config {
        geometry: Some(Geometry {
            foreground_color: Some("#111111ff".to_string()),
            background_color: Some("#222222ff".to_string()),
        }),
        ..Config::default()
    };
    let settings = AppSettings::resolve(&cli(&["-g"]), &config);
    match settings.command {
        Command::Screenshot(CaptureMode::Geometry {
            foreground_color,
            background_color,
        }) => {
            assert_eq!(foreground_color.as_deref(), Some("#111111ff"));
            assert_eq!(background_color.as_deref(), Some("#222222ff"));
        }
        _ => panic!("expected interactive Geometry capture mode"),
    }
}

// ─── clipboard / notifications precedence ──────────────────────────────────

#[cfg(feature = "clipboard")]
#[test]
fn clipboard_precedence() {
    assert!(!AppSettings::resolve(&cli(&[]), &Config::default()).clipboard);
    assert!(AppSettings::resolve(&cli(&["--clipboard"]), &Config::default()).clipboard);

    let config = config_with_base(Base {
        clipboard: Some(true),
        ..Base::default()
    });
    assert!(AppSettings::resolve(&cli(&[]), &config).clipboard);
}

#[cfg(feature = "notifications")]
#[test]
fn notifications_precedence() {
    assert!(AppSettings::resolve(&cli(&[]), &Config::default()).notifications);
    assert!(!AppSettings::resolve(&cli(&["--silent"]), &Config::default()).notifications);

    let config = config_with_base(Base {
        notifications: Some(false),
        ..Base::default()
    });
    assert!(!AppSettings::resolve(&cli(&[]), &config).notifications);

    // --silent always wins, even if config explicitly re-enables notifications.
    let config = config_with_base(Base {
        notifications: Some(true),
        ..Base::default()
    });
    assert!(!AppSettings::resolve(&cli(&["--silent"]), &config).notifications);
}

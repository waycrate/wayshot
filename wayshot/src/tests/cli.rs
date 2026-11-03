use std::path::PathBuf;

use clap::Parser;

use crate::cli::Cli;

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    let mut full = vec!["wayshot"];
    full.extend_from_slice(args);
    Cli::try_parse_from(full)
}

/// Sanity check that the whole `clap` derive definition (conflicts, aliases,
/// value names, etc.) is internally consistent.
#[test]
fn cli_definition_is_valid() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}

#[test]
fn no_args_uses_defaults() {
    let cli = parse(&[]).expect("should parse with no args");
    assert!(cli.file.is_none());
    assert!(!cli.list_outputs);
    assert!(!cli.list_outputs_json);
    assert!(!cli.list_outputs_info);
    assert!(!cli.list_toplevels);
    assert!(!cli.list_toplevels_json);
    assert!(!cli.geometry);
    assert!(cli.geometry_string.is_none());
    assert!(cli.geometry_background_color.is_none());
    assert!(cli.geometry_foreground_color.is_none());
    assert!(cli.output.is_none());
    assert!(!cli.choose_output);
    assert!(cli.toplevel.is_none());
    assert!(!cli.choose_toplevel);
    assert!(!cli.cursor);
    assert!(!cli.no_freeze);
    assert!(cli.delay.is_none());
    assert!(cli.encoding.is_none());
    assert!(cli.file_name_format.is_none());
    assert!(cli.config.is_none());
}

#[test]
fn positional_file_is_captured() {
    let cli = parse(&["/tmp/shot.png"]).unwrap();
    assert_eq!(cli.file, Some(PathBuf::from("/tmp/shot.png")));
}

#[test]
fn dash_positional_is_captured_verbatim() {
    let cli = parse(&["-"]).unwrap();
    assert_eq!(cli.file, Some(PathBuf::from("-")));
}

#[test]
fn short_and_long_output_flag() {
    assert_eq!(
        parse(&["-o", "eDP-1"]).unwrap().output.as_deref(),
        Some("eDP-1")
    );
    assert_eq!(
        parse(&["--output", "eDP-1"]).unwrap().output.as_deref(),
        Some("eDP-1")
    );
}

#[test]
fn list_outputs_short_long_and_alias() {
    assert!(parse(&["-l"]).unwrap().list_outputs);
    assert!(parse(&["--list-outputs"]).unwrap().list_outputs);
}

#[test]
fn list_outputs_json_and_info_flags() {
    assert!(parse(&["--list-outputs-json"]).unwrap().list_outputs_json);
    assert!(parse(&["--list-outputs-info"]).unwrap().list_outputs_info);
}

#[test]
fn list_toplevels_and_alias() {
    assert!(parse(&["--list-toplevels"]).unwrap().list_toplevels);
    assert!(parse(&["--list-windows"]).unwrap().list_toplevels);
}

#[test]
fn list_toplevels_json_and_alias() {
    assert!(
        parse(&["--list-toplevels-json"])
            .unwrap()
            .list_toplevels_json
    );
    assert!(parse(&["--list-windows-json"]).unwrap().list_toplevels_json);
}

#[test]
fn toplevel_flag_and_alias() {
    assert_eq!(
        parse(&["--toplevel", "foo"]).unwrap().toplevel.as_deref(),
        Some("foo")
    );
    assert_eq!(
        parse(&["--window", "foo"]).unwrap().toplevel.as_deref(),
        Some("foo")
    );
}

#[test]
fn choose_toplevel_flag_and_alias() {
    assert!(parse(&["--choose-toplevel"]).unwrap().choose_toplevel);
    assert!(parse(&["--choose-window"]).unwrap().choose_toplevel);
}

#[test]
fn choose_output_flag_and_alias() {
    assert!(parse(&["--choose-output"]).unwrap().choose_output);
    assert!(parse(&["--choose-output"]).unwrap().choose_output);
}

#[test]
fn geometry_flag() {
    assert!(parse(&["-g"]).unwrap().geometry);
    assert!(parse(&["--geometry"]).unwrap().geometry);
}

#[test]
fn geometry_flag_does_not_take_a_value() {
    // Anything after `-g` is parsed on its own: flags stay flags and a bare
    // token is the output file.
    let cli = parse(&["-g", "-c"]).unwrap();
    assert!(cli.geometry);
    assert!(cli.cursor);
    let cli = parse(&["-g", "out.png"]).unwrap();
    assert!(cli.geometry);
    assert_eq!(cli.file, Some(PathBuf::from("out.png")));
    let cli = parse(&["-g", "-"]).unwrap();
    assert!(cli.geometry);
    assert_eq!(cli.file, Some(PathBuf::from("-")));
}

#[test]
fn geometry_string_with_value() {
    let cli = parse(&["-G", "100,100 200x200"]).unwrap();
    assert_eq!(cli.geometry_string.as_deref(), Some("100,100 200x200"));
    let cli = parse(&["--geometry-string", "0,0 10x10"]).unwrap();
    assert_eq!(cli.geometry_string.as_deref(), Some("0,0 10x10"));
}

#[test]
fn geometry_string_allows_negative_coordinates() {
    let cli = parse(&["-G", "-1920,-10 1920x1080", "-c", "out.png"]).unwrap();
    assert_eq!(cli.geometry_string.as_deref(), Some("-1920,-10 1920x1080"));
    assert!(cli.cursor);
    assert_eq!(cli.file, Some(PathBuf::from("out.png")));
}

#[test]
fn geometry_string_requires_a_value() {
    assert!(parse(&["-G"]).is_err());
    assert!(parse(&["--geometry-string"]).is_err());
}

#[test]
fn geometry_string_conflicts_with_geometry() {
    assert!(parse(&["-g", "-G", "0,0 10x10"]).is_err());
    assert!(parse(&["-G", "0,0 10x10", "-g"]).is_err());
}

#[test]
fn geometry_conflicts_with_output() {
    assert!(parse(&["-g", "-o", "eDP-1"]).is_err());
    assert!(parse(&["-o", "eDP-1", "-g"]).is_err());
    assert!(parse(&["-G", "0,0 10x10", "-o", "eDP-1"]).is_err());
}

#[test]
fn choose_output_conflicts_with_geometry_and_output() {
    assert!(parse(&["--choose-output", "-g"]).is_err());
    assert!(parse(&["--choose-output", "-G", "0,0 10x10"]).is_err());
    assert!(parse(&["--choose-output", "-o", "eDP-1"]).is_err());
}

#[test]
fn toplevel_conflicts_with_geometry_output_and_choose_flags() {
    assert!(parse(&["--toplevel", "x", "-g"]).is_err());
    assert!(parse(&["--toplevel", "x", "-G", "0,0 10x10"]).is_err());
    assert!(parse(&["--toplevel", "x", "-o", "eDP-1"]).is_err());
    assert!(parse(&["--toplevel", "x", "--choose-output"]).is_err());
    assert!(parse(&["--toplevel", "x", "--choose-toplevel"]).is_err());
}

#[test]
fn choose_toplevel_conflicts_with_toplevel_output_and_choose_output() {
    assert!(parse(&["--choose-toplevel", "--toplevel", "x"]).is_err());
    assert!(parse(&["--choose-toplevel", "-o", "eDP-1"]).is_err());
    assert!(parse(&["--choose-toplevel", "--choose-output"]).is_err());
}

#[test]
fn independent_capture_target_flags_parse_together_fine() {
    // Sanity: flags that are NOT in each other's conflict lists coexist.
    assert!(parse(&["--cursor", "-o", "eDP-1"]).is_ok());
}

#[test]
fn cursor_and_no_freeze_flags() {
    let cli = parse(&["--cursor", "--no-freeze"]).unwrap();
    assert!(cli.cursor);
    assert!(cli.no_freeze);
}

#[test]
fn delay_parses_as_u32() {
    assert_eq!(parse(&["--delay", "250"]).unwrap().delay, Some(250));
}

#[test]
fn delay_rejects_non_numeric() {
    assert!(parse(&["--delay", "notanumber"]).is_err());
}

#[test]
fn delay_rejects_negative() {
    assert!(parse(&["--delay", "-5"]).is_err());
}

#[test]
fn encoding_and_its_visible_aliases() {
    use crate::utils::EncodingFormat;
    assert_eq!(
        parse(&["--encoding", "png"]).unwrap().encoding,
        Some(EncodingFormat::Png)
    );
    assert_eq!(
        parse(&["--extension", "png"]).unwrap().encoding,
        Some(EncodingFormat::Png)
    );
    assert_eq!(
        parse(&["--format", "png"]).unwrap().encoding,
        Some(EncodingFormat::Png)
    );
    assert_eq!(
        parse(&["--file-format", "png"]).unwrap().encoding,
        Some(EncodingFormat::Png)
    );
}

#[test]
fn invalid_encoding_value_fails() {
    assert!(parse(&["--encoding", "bogus"]).is_err());
}

#[test]
fn file_name_format_flag() {
    assert_eq!(
        parse(&["--file-name-format", "shot-%s"])
            .unwrap()
            .file_name_format
            .as_deref(),
        Some("shot-%s")
    );
}

#[test]
fn config_flag_sets_path() {
    let cli = parse(&["--config", "/tmp/wayshot.toml"]).unwrap();
    assert_eq!(cli.config, Some(PathBuf::from("/tmp/wayshot.toml")));
}

#[test]
fn geometry_color_flags() {
    let cli = parse(&[
        "--geometry-foreground-color",
        "#ffffffff",
        "--geometry-background-color",
        "#00000050",
    ])
    .unwrap();
    assert_eq!(cli.geometry_foreground_color.as_deref(), Some("#ffffffff"));
    assert_eq!(cli.geometry_background_color.as_deref(), Some("#00000050"));
}

#[test]
fn unknown_flag_errors() {
    assert!(parse(&["--not-a-real-flag"]).is_err());
}

#[test]
fn help_and_version_are_recognized() {
    // clap treats --help/--version as "successful" errors (DisplayHelp/DisplayVersion).
    assert!(parse(&["--help"]).is_err());
    assert!(parse(&["--version"]).is_err());
}

#[cfg(feature = "completions")]
#[test]
fn completions_flag_parses_shell() {
    let cli = parse(&["--completions", "fish"]).unwrap();
    assert!(matches!(cli.completions, Some(crate::cli::Shell::Fish)));
}

#[cfg(feature = "completions")]
#[test]
fn completions_is_exclusive_with_other_flags() {
    assert!(parse(&["--completions", "fish", "--cursor"]).is_err());
}

#[cfg(feature = "completions")]
#[test]
fn completions_rejects_unknown_shell() {
    assert!(parse(&["--completions", "not-a-shell"]).is_err());
}

#[cfg(feature = "color_picker")]
#[test]
fn color_flag_default_missing_value_is_plain() {
    use crate::cli::ColorFormat;
    let cli = parse(&["--color"]).unwrap();
    assert_eq!(cli.color, Some(ColorFormat::Plain));
}

#[cfg(feature = "color_picker")]
#[test]
fn color_flag_with_explicit_format() {
    use crate::cli::ColorFormat;
    assert_eq!(
        parse(&["--color", "hex"]).unwrap().color,
        Some(ColorFormat::Hex)
    );
    assert_eq!(
        parse(&["--color", "hex-alpha"]).unwrap().color,
        Some(ColorFormat::HexAlpha)
    );
    assert_eq!(
        parse(&["--color", "hsl"]).unwrap().color,
        Some(ColorFormat::Hsl)
    );
}

#[cfg(feature = "color_picker")]
#[test]
fn color_conflicts_with_geometry_output_and_choose_output() {
    assert!(parse(&["--color", "-g"]).is_err());
    assert!(parse(&["--color", "-G", "0,0 10x10"]).is_err());
    assert!(parse(&["--color", "-o", "eDP-1"]).is_err());
    assert!(parse(&["--color", "--choose-output"]).is_err());
}

#[cfg(feature = "clipboard")]
#[test]
fn clipboard_flag() {
    assert!(parse(&["--clipboard"]).unwrap().clipboard);
    assert!(!parse(&[]).unwrap().clipboard);
}

#[cfg(feature = "notifications")]
#[test]
fn silent_flag_and_alias() {
    assert!(parse(&["--silent"]).unwrap().silent);
    assert!(parse(&["--no-notifications"]).unwrap().silent);
    assert!(!parse(&[]).unwrap().silent);
}

#[cfg(feature = "logger")]
#[test]
fn log_level_flag_parses() {
    let cli = parse(&["--log-level", "debug"]).unwrap();
    assert_eq!(cli.log_level, Some(tracing::Level::DEBUG));
}

#[cfg(feature = "logger")]
#[test]
fn invalid_log_level_fails() {
    assert!(parse(&["--log-level", "not-a-level"]).is_err());
}

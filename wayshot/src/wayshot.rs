use config::Config;
use std::{
    env,
    io::{self, BufWriter, Write},
    path::PathBuf,
};

use clap::Parser;
use eyre::Result;
use libwayshot::WayshotConnection;

mod cli;
#[cfg(feature = "clipboard")]
mod clipboard;
#[cfg(feature = "color_picker")]
mod color_picker;
mod config;
#[cfg(feature = "logger")]
mod logger;
#[cfg(feature = "notifications")]
mod notification;
mod screenshot;
mod utils;

use crate::utils::EncodingFormat;

// ─── Resolved settings ────────────────────────────────────────────────────────

/// Runtime settings derived by merging CLI flags with the config file.
///
/// Priority for every field: explicit CLI flag > config file value > built-in default.
/// For boolean flags a CLI `true` always wins; config provides the default.
struct AppSettings {
    /// Whether to render the cursor in the captured image.
    cursor: bool,
    /// Final encoding format, after resolving extension / flag / config precedence.
    encoding: EncodingFormat,
    /// Destination file path, or `None` to skip file output.
    file: Option<PathBuf>,
    /// Write image bytes to stdout.
    stdout_print: bool,
    /// JPEG-XL encoder settings (always present; used only when encoding is Jxl).
    jxl: config::Jxl,
    /// PNG encoder settings.
    png: config::Png,
    #[cfg(feature = "clipboard")]
    clipboard: bool,
    #[cfg(feature = "notifications")]
    notifications: bool,
}

impl AppSettings {
    fn resolve(cli: &cli::Cli, config: &Config) -> Self {
        let base = config.base.clone().unwrap_or_default();
        let file_config = config.file.clone().unwrap_or_default();
        let encoding_config = config.encoding.clone().unwrap_or_default();

        // ── Cursor ────────────────────────────────────────────────────────────
        // Either the --cursor flag or config `cursor = true` enables cursor capture.
        let cursor = cli.cursor || base.cursor.unwrap_or_default();

        // ── Encoding ──────────────────────────────────────────────────────────
        // Resolution order:
        //   1. --encoding flag
        //   2. format inferred from the FILE extension
        //   3. config `[file] encoding`
        //   4. built-in default (PNG)
        let input_encoding: Option<EncodingFormat> =
            cli.file.as_ref().and_then(|p| p.try_into().ok());
        let encoding = cli
            .encoding
            .or(input_encoding)
            .unwrap_or_else(|| file_config.encoding.unwrap_or_default());

        if let Some(ie) = input_encoding
            && ie != encoding
        {
            tracing::warn!(
                "Requested encoding '{encoding}' does not match \
                 the file extension '{ie}'. Using the requested encoding."
            );
        }

        // ── File name format ──────────────────────────────────────────────────
        // CLI --file-name-format overrides config; falls back to a timestamp pattern.
        let file_name_format = cli.file_name_format.clone().unwrap_or_else(|| {
            file_config
                .name_format
                .clone()
                .unwrap_or_else(|| "wayshot-%Y_%m_%d-%H_%M_%S".to_string())
        });

        // ── Stdout / file path ────────────────────────────────────────────────
        // stdout_print starts from config `stdout = true`.
        // resolve_output_file may also flip it to true when FILE is `-`.
        let mut stdout_print = base.stdout.unwrap_or_default();
        let file = resolve_output_file(
            cli.file.clone(),
            &base,
            &file_config,
            &file_name_format,
            encoding,
            &mut stdout_print,
        );

        AppSettings {
            cursor,
            encoding,
            file,
            stdout_print,
            jxl: encoding_config.jxl.unwrap_or_default(),
            png: encoding_config.png.unwrap_or_default(),
            #[cfg(feature = "clipboard")]
            clipboard: cli.clipboard || base.clipboard.unwrap_or_default(),
            #[cfg(feature = "notifications")]
            notifications: !cli.silent && base.notifications.unwrap_or(true),
        }
    }
}

/// Determine where (if anywhere) to write the image file.
///
/// Sets `stdout_print` to `true` when the caller passed `-` as the file path.
fn resolve_output_file(
    cli_file: Option<PathBuf>,
    base: &config::Base,
    file_config: &config::File,
    file_name_format: &str,
    encoding: EncodingFormat,
    stdout_print: &mut bool,
) -> Option<PathBuf> {
    if let Some(path) = cli_file {
        if path.to_string_lossy() == "-" {
            *stdout_print = true;
            return None;
        }
        return Some(utils::get_full_file_name(&path, file_name_format, encoding));
    }
    if base.file.unwrap_or_default() {
        let dir = file_config
            .path
            .clone()
            .unwrap_or_else(|| env::current_dir().unwrap_or_default());
        return Some(utils::get_full_file_name(&dir, file_name_format, encoding));
    }
    None
}

// ─── List commands ────────────────────────────────────────────────────────────

/// Handle `--list-*` flags. Returns `true` if a list command was handled.
fn handle_list_commands(
    cli: &cli::Cli,
    conn: &WayshotConnection,
    writer: &mut impl Write,
) -> Result<bool> {
    if cli.list_outputs {
        for output in conn.get_all_outputs() {
            writeln!(writer, "{}", output.name)?;
        }
        return Ok(true);
    }
    if cli.list_outputs_info {
        conn.print_displays_info();
        return Ok(true);
    }
    if cli.list_toplevels {
        for tl in conn.get_all_toplevels().iter().filter(|t| t.active) {
            writeln!(writer, "{}", tl.id_and_title())?;
        }
        return Ok(true);
    }
    Ok(false)
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let config_path = cli.config.clone().unwrap_or(Config::get_default_path());
    let config = Config::load(&config_path).unwrap_or_default();

    #[cfg(feature = "logger")]
    logger::setup(&cli, &config);

    let settings = AppSettings::resolve(&cli, &config);
    let conn = WayshotConnection::new()?;

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    // List commands print info and exit immediately without capturing.
    if handle_list_commands(&cli, &conn, &mut writer)? {
        writer.flush()?;
        return Ok(());
    }

    // Color picker is also a query mode — pick a pixel color and exit.
    #[cfg(feature = "color_picker")]
    if cli.color {
        return color_picker::pick(&conn);
    }

    let mode = screenshot::CaptureMode::from_cli(&cli);
    let result = screenshot::capture(&conn, &mode, settings.cursor);

    match result {
        Ok((image_buffer, shot_result)) => {
            let encoded = utils::encode_image(
                &image_buffer,
                settings.encoding,
                &settings.jxl,
                &settings.png,
            )
            .map_err(|e| eyre::eyre!("Failed to encode image: {e}"))?;

            if let Some(ref f) = settings.file {
                std::fs::write(f, &encoded)?;
            }

            if settings.stdout_print {
                writer.write_all(&encoded)?;
            }

            #[cfg(feature = "clipboard")]
            if settings.clipboard {
                clipboard::copy_to_clipboard(encoded)?;
            }

            #[cfg(feature = "notifications")]
            if settings.notifications {
                notification::send_success(&shot_result);
            }
            // Silence unused warning when the notifications feature is disabled.
            #[cfg(not(feature = "notifications"))]
            drop(shot_result);

            Ok(())
        }
        Err(e) => {
            #[cfg(feature = "notifications")]
            if settings.notifications {
                notification::send_failure(&e);
            }
            Err(e)
        }
    }
}

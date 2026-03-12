use std::{env, path::PathBuf};

use crate::cli::Cli;
use crate::config::{self, Config};
use crate::screenshot::CaptureMode;
use crate::utils::{self, EncodingFormat};

// ─── Command ──────────────────────────────────────────────────────────────────

/// The top-level operation to perform, fully resolved from CLI + config.
pub(crate) enum Command {
    /// Print the names of all connected outputs and exit.
    ListOutputs,
    /// Print detailed info about all connected outputs and exit.
    ListOutputsInfo,
    /// Print the id+title strings of all active toplevels and exit.
    ListToplevels,
    /// Pick a pixel color interactively and exit.
    #[cfg(feature = "color_picker")]
    ColorPicker,
    /// Capture a screenshot using the given mode.
    Screenshot(CaptureMode),
}

// ─── Resolved settings ────────────────────────────────────────────────────────

/// Runtime settings derived by merging CLI flags with the config file.
///
/// Priority for every field: explicit CLI flag > config file value > built-in default.
/// After `resolve()` returns, nothing in the call chain needs `cli` or `config`.
pub(crate) struct AppSettings {
    /// Top-level operation to execute.
    pub(crate) command: Command,
    /// Whether to render the cursor in the captured image.
    pub(crate) cursor: bool,
    /// When true, freeze the screen before region/point selection; when false, select on live display.
    pub(crate) freeze: bool,
    /// Final encoding format, after resolving extension / flag / config precedence.
    pub(crate) encoding: EncodingFormat,
    /// Destination file path, or `None` to skip file output.
    pub(crate) file: Option<PathBuf>,
    /// Write image bytes to stdout.
    pub(crate) stdout_print: bool,
    /// JPEG-XL encoder settings (always present; used only when encoding is Jxl).
    pub(crate) jxl: config::Jxl,
    /// PNG encoder settings.
    pub(crate) png: config::Png,
    #[cfg(feature = "clipboard")]
    pub(crate) clipboard: bool,
    #[cfg(feature = "notifications")]
    pub(crate) notifications: bool,
}

impl AppSettings {
    pub(crate) fn resolve(cli: &Cli, config: &Config) -> Self {
        let base = config.base.clone().unwrap_or_default();
        let file_config = config.file.clone().unwrap_or_default();
        let encoding_config = config.encoding.clone().unwrap_or_default();

        // ── Cursor ────────────────────────────────────────────────────────────
        // Either the --cursor flag or config `cursor = true` enables cursor capture.
        let cursor = cli.cursor || base.cursor.unwrap_or_default();

        // ── Freeze ─────────────────────────────────────────────────────────────
        // Freeze screen before selection; false when CLI --no-freeze or config freeze = false.
        let freeze = !cli.no_freeze && base.freeze.unwrap_or(true);

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
        let file = Self::resolve_output_file(
            cli.file.clone(),
            &base,
            &file_config,
            &file_name_format,
            encoding,
            &mut stdout_print,
        );

        // ── Command ───────────────────────────────────────────────────────────
        // Query commands are checked first; screenshot mode is the default.
        let command = 'cmd: {
            if cli.list_outputs {
                break 'cmd Command::ListOutputs;
            }
            if cli.list_outputs_info {
                break 'cmd Command::ListOutputsInfo;
            }
            if cli.list_toplevels {
                break 'cmd Command::ListToplevels;
            }
            #[cfg(feature = "color_picker")]
            if cli.color {
                break 'cmd Command::ColorPicker;
            }
            let output = cli.output.clone().or_else(|| base.output.clone());
            Command::Screenshot(Self::resolve_capture_mode(cli, output))
        };

        AppSettings {
            command,
            cursor,
            freeze,
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

    fn resolve_capture_mode(cli: &Cli, output: Option<String>) -> CaptureMode {
        #[cfg(feature = "selector")]
        if cli.geometry {
            return CaptureMode::Geometry;
        }
        if let Some(ref name) = cli.toplevel {
            CaptureMode::Toplevel(name.clone())
        } else if cli.choose_toplevel {
            CaptureMode::ChooseToplevel
        } else if let Some(name) = output {
            CaptureMode::Output(name)
        } else if cli.choose_output {
            CaptureMode::ChooseOutput
        } else {
            CaptureMode::All
        }
    }

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
}

use std::path::PathBuf;

use clap::{
    Parser,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};
#[cfg(feature = "logger")]
use tracing::Level;

use crate::utils::EncodingFormat;

fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Parser)]
#[command(version, about, styles=get_styles())]
pub struct Cli {
    // ─── Positional argument ──────────────────────────────────────────────────
    /// Custom screenshot file path. Accepts:
    ///   1. A directory — default naming scheme is applied.
    ///   2. A file path — encoding is inferred from the extension.
    ///   3. `-` — write raw image bytes to stdout (no file is saved).
    #[arg(value_name = "FILE", verbatim_doc_comment)]
    pub file: Option<PathBuf>,

    // ─── Query commands (print info then exit; no screenshot is taken) ────────
    /// List all connected outputs by name.
    #[arg(short, long, alias = "list-outputs")]
    pub list_outputs: bool,

    /// List all connected outputs with name, description, size, and position.
    #[arg(long)]
    pub list_outputs_info: bool,

    /// List all active toplevel windows.
    #[arg(long, alias = "list-windows")]
    pub list_toplevels: bool,

    /// Click a point on screen and print its color values.
    #[cfg(feature = "color_picker")]
    #[arg(long, conflicts_with_all = ["geometry", "output", "choose_output"])]
    pub color: bool,

    // ─── Capture target (what to capture) ────────────────────────────────────
    /// Interactively select a screen region to capture.
    #[arg(short, long)]
    pub geometry: bool,

    /// Capture a specific output/display by name.
    #[arg(short, long, conflicts_with = "geometry")]
    pub output: Option<String>,

    /// Interactively choose an output/display to capture.
    #[arg(long, alias = "choose-output", conflicts_with_all = ["geometry", "output"])]
    pub choose_output: bool,

    /// Capture a specific toplevel window by its "app_id title" string.
    #[arg(long, alias = "window", conflicts_with_all = ["geometry", "output", "choose_output", "choose_toplevel"])]
    pub toplevel: Option<String>,

    /// Interactively choose a toplevel window to capture.
    #[arg(long, alias = "choose-window", conflicts_with_all = ["geometry", "output", "choose_output", "toplevel"])]
    pub choose_toplevel: bool,

    // ─── Capture options (how to capture) ────────────────────────────────────
    /// Include the cursor in the screenshot.
    #[arg(short, long)]
    pub cursor: bool,

    /// Do not freeze the screen when selecting a region (geometry) or a point (color picker).
    /// Selection happens on the live display; the capture is taken after selection.
    #[arg(long)]
    pub no_freeze: bool,

    // ─── Output options (where/how to save the image) ─────────────────────────
    /// Image encoding format. Defaults to the FILE extension, then to `png`.
    #[arg(
        long,
        verbatim_doc_comment,
        visible_aliases = ["extension", "format", "file-format"],
        value_name = "FILE_EXTENSION"
    )]
    pub encoding: Option<EncodingFormat>,

    /// `strftime`-style format for the output file name.
    /// Defaults to `wayshot-%Y_%m_%d-%H_%M_%S`.
    #[arg(long, verbatim_doc_comment)]
    pub file_name_format: Option<String>,

    /// Copy the screenshot to the Wayland clipboard.
    /// Wayshot stays in the background offering the image until it is overwritten.
    #[cfg(feature = "clipboard")]
    #[arg(long, verbatim_doc_comment)]
    pub clipboard: bool,

    // ─── Notification options ─────────────────────────────────────────────────
    /// Silents notification after screenshot
    #[cfg(feature = "notifications")]
    #[arg(long, alias = "no-notifications")]
    pub silent: bool,

    // ─── Global options ───────────────────────────────────────────────────────
    /// Path to the config file. Defaults to:
    ///   1. `$XDG_CONFIG_HOME/wayshot/config.toml`
    ///   2. `$HOME/wayshot/config.toml`
    ///   3. Built-in defaults when no file is found.
    #[arg(long, verbatim_doc_comment)]
    pub config: Option<PathBuf>,

    /// Log level written to stderr.
    #[cfg(feature = "logger")]
    #[arg(long)]
    pub log_level: Option<Level>,
}

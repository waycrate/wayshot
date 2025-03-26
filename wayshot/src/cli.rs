use std::path::PathBuf;

use clap::{
    Parser, arg,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};
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
    /// Custom screenshot file path can be of the following types:
    ///     1. Directory (Default naming scheme is used for the screenshot file).
    ///     2. Path (Encoding is automatically inferred from the extension).
    ///     3. `-` (Indicates writing to terminal [stdout]).
    #[arg(value_name = "FILE", verbatim_doc_comment)]
    pub file: Option<PathBuf>,

    /// Copy image to clipboard. Can be used simultaneously with [FILE].
    /// Wayshot persists in the background offering the image till the clipboard is overwritten.
    #[arg(long, verbatim_doc_comment)]
    pub clipboard: bool,

    /// Log level to be used for printing to stderr
    #[arg(long, verbatim_doc_comment)]
    pub log_level: Option<Level>,

    /// Region aware screenshotting
    #[arg(short, long)]
    pub geometry: bool,

    /// Enable cursor in screenshots
    #[arg(short, long)]
    pub cursor: bool,

    /// Set image encoder, by default uses the file extension from the FILE
    /// positional argument. Otherwise defaults to png.
    #[arg(long, verbatim_doc_comment, visible_aliases = ["extension", "format", "file-format"], value_name = "FILE_EXTENSION")]
    pub encoding: Option<EncodingFormat>,

    /// List all valid outputs
    #[arg(short, long, alias = "list-outputs")]
    pub list_outputs: bool,

    /// Choose a particular output/display to screenshot
    #[arg(short, long, conflicts_with = "geometry")]
    pub output: Option<String>,

    /// Present a fuzzy selector for output/display selection
    #[arg(long, alias = "choose-output", conflicts_with_all = ["geometry", "output"])]
    pub choose_output: bool,

    /// Output file name's formatting.
    /// Defaults to config value (`wayshot-%Y_%m_%d-%H_%M_%S`)
    #[arg(long, verbatim_doc_comment)]
    pub file_name_format: Option<String>,

    /// Path to your config file.
    /// Defaults to:
    ///     1. `$XDG_CONFIG_HOME/wayshot/config.toml`
    ///     2. `$HOME/wayshot/config.toml` -- if `$XDG_CONFIG_HOME` variable doesn't exist
    ///     3. `None` -- if the config isn't found, the `Config::default()` will be used
    #[arg(long, verbatim_doc_comment)]
    pub config: Option<PathBuf>,
}

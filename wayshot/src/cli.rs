use std::path::PathBuf;

use clap::{
    Parser, arg,
    builder::{
        Styles, TypedValueParser,
        styling::{AnsiColor, Effects},
    },
};
use eyre::WrapErr;

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
    #[arg(long, default_value = "info", value_parser = clap::builder::PossibleValuesParser::new(["trace", "debug", "info", "warn", "error"]).map(|s| -> tracing::Level{ s.parse().wrap_err_with(|| format!("Failed to parse log level: {}", s)).unwrap()}))]
    pub log_level: tracing::Level,

    /// Arguments to call slurp with for selecting a region
    #[arg(short, long, value_name = "SLURP_ARGS")]
    pub slurp: Option<Option<String>>,

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
    #[arg(short, long, conflicts_with = "slurp")]
    pub output: Option<String>,

    /// Present a fuzzy selector for output/display selection
    #[arg(long, alias = "choose-output", conflicts_with_all = ["slurp", "output"])]
    pub choose_output: bool,
}

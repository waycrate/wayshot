use std::path::PathBuf;

use clap::arg;

use clap::Parser;
use eyre::WrapErr;

use crate::utils::EncodingFormat;
use clap::builder::TypedValueParser;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    /// Custom output path can be of the following types:
    ///     1. Directory (Default naming scheme is used for the image output).
    ///     2. Path (Encoding is automatically inferred from the extension).
    ///     3. `-` (Indicates writing to terminal [stdout]).
    #[arg(value_name = "OUTPUT", verbatim_doc_comment)]
    pub file: Option<PathBuf>,

    /// Copy image to clipboard. Can be used simultaneously with [OUTPUT] or stdout.
    /// Wayshot persists in the background offering the image till the clipboard is overwritten.
    #[arg(long, verbatim_doc_comment)]
    pub clipboard: bool,

    /// Log level to be used for printing to stderr
    #[arg(long, default_value = "info", value_parser = clap::builder::PossibleValuesParser::new(["trace", "debug", "info", "warn", "error"]).map(|s| -> tracing::Level{ s.parse().wrap_err_with(|| format!("Failed to parse log level: {}", s)).unwrap()}))]
    pub log_level: tracing::Level,

    /// Arguments to call slurp with for selecting a region
    #[arg(short, long, value_name = "SLURP_ARGS")]
    pub slurp: Option<String>,

    /// Enable cursor in screenshots
    #[arg(short, long)]
    pub cursor: bool,

    /// Set image encoder, by default uses the file extension from the OUTPUT
    /// positional argument. Otherwise defaults to png.
    #[arg(long, verbatim_doc_comment, visible_aliases = ["extension", "format", "output-format"], value_name = "FILE_EXTENSION")]
    pub encoding: Option<EncodingFormat>,

    /// List all valid outputs
    #[arg(short, long, alias = "listoutputs")]
    pub list_outputs: bool,

    /// Choose a particular output/display to screenshot
    #[arg(short, long, conflicts_with = "slurp")]
    pub output: Option<String>,

    /// Present a fuzzy selector for output/display selection
    #[arg(long, alias = "chooseoutput", conflicts_with_all = ["slurp", "output"])]
    pub choose_output: bool,

    /// Path to your config file
    /// defaults to:
    ///     1. `$XDG_CONFIG_HOME/wayshot/config.toml`
    ///     2. `$HOME/wayshot/config.toml` -- if `$XDG_CONFIG_HOME` variable doesn't exist
    ///     3. None -- if the config isn't found, the `Config::default()` will be used
    #[arg(long)]
    pub config: Option<PathBuf>,
}

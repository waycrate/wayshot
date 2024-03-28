use crate::utils::EncodingFormat;
use clap::Parser;
use std::path::PathBuf;
use tracing::Level;

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
    /// Defaults to config value (`true`)
    #[arg(long, verbatim_doc_comment)]
    pub clipboard: Option<bool>,

    /// Log level to be used for printing to stderr
    /// Defaults to config value (`info`)
    #[arg(long, verbatim_doc_comment)]
    pub log_level: Option<Level>,

    /// Arguments to call slurp with for selecting a region
    #[arg(short, long, value_name = "SLURP_ARGS")]
    pub slurp: Option<String>,

    /// Enable cursor in screenshots.
    /// Defaults to config value (`false`)
    #[arg(short, long, verbatim_doc_comment)]
    pub cursor: Option<bool>,

    /// Set image encoder, by default uses the file extension from the OUTPUT
    /// positional argument. Otherwise defaults to config value (`png`).
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

    /// Path to your config file.
    /// Defaults to:
    ///     1. `$XDG_CONFIG_HOME/wayshot/config.toml`
    ///     2. `$HOME/wayshot/config.toml` -- if `$XDG_CONFIG_HOME` variable doesn't exist
    ///     3. `None` -- if the config isn't found, the `Config::default()` will be used
    #[arg(long, verbatim_doc_comment)]
    pub config: Option<PathBuf>,

    /// Output filename's formatting.
    /// Defaults to config value (`wayshot-%Y_%m_%d-%H_%M_%S`)
    #[arg(long, verbatim_doc_comment)]
    pub filename_format: Option<String>,
}

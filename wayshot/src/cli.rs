use clap::arg;

use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    /// Enable debug mode
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub debug: bool,

    /// Arguments to call slurp with for selecting a region
    #[arg(short, long, value_name = "SLURP_ARGS")]
    pub slurp: Option<String>,

    /// Mention a custom file path
    #[arg(short, long, conflicts_with = "stdout", value_name = "FILE_PATH")]
    pub file: Option<String>,

    /// Output the image data to standard out
    #[arg(long, conflicts_with = "file", action = clap::ArgAction::SetTrue)]
    pub stdout: bool,

    /// Enable cursor in screenshots
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub cursor: bool,

    /// Set image encoder (Png is default)
    #[arg(short, long, value_name = "FILE_EXTENSION")]
    pub extension: Option<String>,

    /// List all valid outputs
    #[arg(short, long, alias = "listoutputs", action = clap::ArgAction::SetTrue)]
    pub list_outputs: bool,

    /// Choose a particular display to screenshot
    #[arg(short, long, conflicts_with = "slurp")]
    pub output: Option<String>,

    /// Present a fuzzy selector for outputs
    #[arg(long, alias = "chooseoutput", conflicts_with_all = ["slurp", "output"], action = clap::ArgAction::SetTrue)]
    pub choose_output: bool,
}

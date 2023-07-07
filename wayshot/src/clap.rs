use clap::{arg, ArgAction, Command};

pub fn set_flags() -> Command {
    Command::new("wayshot")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Screenshot tool for compositors implementing zwlr_screencopy_v1.")
        .arg(
            arg!(-d - -debug)
                .required(false)
                .conflicts_with("stdout")
                .action(ArgAction::SetFalse)
                .help("Enable debug mode"),
        )
        .arg(
            arg!(-s --slurp <GEOMETRY>)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Choose a portion of your display to screenshot using slurp"),
        )
        .arg(
            arg!(-f - -file <FILE_PATH>)
                .required(false)
                .conflicts_with("stdout")
                .action(ArgAction::SetTrue)
                .help("Mention a custom file path"),
        )
        .arg(
            arg!(-c - -cursor)
                .required(false)
                .action(ArgAction::SetFalse)
                .help("Enable cursor in screenshots"),
        )
        .arg(
            arg!(--stdout)
                .required(false)
                .conflicts_with("file")
                .action(ArgAction::SetFalse)
                .help("Output the image data to standard out"),
        )
        .arg(
            arg!(-e --extension <FILE_EXTENSION>)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Set image encoder (Png is default)"),
        )
        .arg(
            arg!(-l - -listoutputs)
                .required(false)
                .action(ArgAction::SetFalse)
                .help("List all valid outputs"),
        )
        .arg(
            arg!(-o --output <OUTPUT>)
                .required(false)
                .action(ArgAction::SetTrue)
                .conflicts_with("slurp")
                .help("Choose a particular display to screenshot"),
        )
}

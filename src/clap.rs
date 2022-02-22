use clap::{arg, Command};

pub fn set_flags() -> Command<'static> {
    let app = Command::new("wayshot")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Simple screenshot tool for wlroots based compositors.")
        .arg(
            arg!(-s --slurp <GEOMETRY>)
                .required(false)
                .takes_value(true)
                .help("Choose a portion of your display to screenshot using slurp."),
        )
        .arg(
            arg!(-d - -debug)
                .required(false)
                .takes_value(false)
                .help("Enable debug mode."),
        )
        .arg(
            arg!(-l - -listoutputs)
                .required(false)
                .takes_value(false)
                .help("List all valid outputs."),
        )
        .arg(
            arg!(-o --output <OUTPUT>)
                .required(false)
                .takes_value(true)
                .conflicts_with("slurp")
                .help("Choose a particular display to screenshot."),
        )
        .arg(
            arg!(-c - -cursor)
                .required(false)
                .takes_value(false)
                .help("Enable cursor in screenshots."),
        );
    app
}

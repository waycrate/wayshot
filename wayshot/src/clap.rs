#![deny(warnings)]
#![warn(unused_extern_crates)]
// Enable some groups of clippy lints.
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
// Specific lints to enforce.
#![warn(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![deny(clippy::disallowed_types)]
#![deny(clippy::manual_let_else)]
#![allow(clippy::unreachable)]

use clap::{arg, ArgAction, Command};

pub fn set_flags() -> Command {
    Command::new("wayshot")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Screenshot tool for compositors implementing zwlr_screencopy_v1.")
        .arg(
            arg!(-d - -debug)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Enable debug mode"),
        )
        .arg(
            arg!(-s --slurp <GEOMETRY>)
                .required(false)
                .action(ArgAction::Set)
                .help("Choose a portion of your display to screenshot using slurp"),
        )
        .arg(
            arg!(-f - -file <FILE_PATH>)
                .required(false)
                .conflicts_with("stdout")
                .action(ArgAction::Set)
                .help("Mention a custom file path"),
        )
        .arg(
            arg!(-c - -cursor)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Enable cursor in screenshots"),
        )
        .arg(
            arg!(--stdout)
                .required(false)
                .conflicts_with("file")
                .action(ArgAction::SetTrue)
                .help("Output the image data to standard out"),
        )
        .arg(
            arg!(-e --extension <FILE_EXTENSION>)
                .required(false)
                .action(ArgAction::Set)
                .help("Set image encoder (Png is default)"),
        )
        .arg(
            arg!(-l - -listoutputs)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("List all valid outputs"),
        )
        .arg(
            arg!(-o --output <OUTPUT>)
                .required(false)
                .action(ArgAction::Set)
                .conflicts_with("slurp")
                .help("Choose a particular display to screenshot"),
        )
        .arg(
            arg!(--chooseoutput)
                .required(false)
                .action(ArgAction::SetTrue)
                .conflicts_with("slurp")
                .conflicts_with("output")
                .help("Present a fuzzy selector for outputs"),
        )
}

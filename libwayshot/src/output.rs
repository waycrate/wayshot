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

use wayland_client::protocol::{wl_output, wl_output::WlOutput};

/// Represents an accessible wayland output.
///
/// Do not instantiate, instead use [`crate::WayshotConnection::get_all_outputs`].
#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub wl_output: WlOutput,
    pub name: String,
    pub description: String,
    pub transform: wl_output::Transform,
    pub dimensions: OutputPositioning,
    pub mode: WlOutputMode,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct WlOutputMode {
    pub width: i32,
    pub height: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct OutputPositioning {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

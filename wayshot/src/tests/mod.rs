//! Private test module. All unit tests for the wayshot binary live here to keep
//! main source files focused. Tests must not require a live Wayland compositor,
//! D-Bus session, or interactive TTY, since they run headless in CI.

mod cli;
#[cfg(feature = "clipboard")]
mod clipboard;
#[cfg(feature = "color_picker")]
mod color_picker;
mod config;
mod listing;
#[cfg(feature = "notifications")]
mod notification;
mod settings;
mod utils;

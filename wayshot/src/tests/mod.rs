//! Private test module. All unit tests for the wayshot binary live here to keep
//! main source files focused. Most tests must not require a live Wayland
//! compositor, D-Bus session, or interactive TTY, since they run headless in
//! CI. A few (`screenshot.rs`) do use a live compositor - CI starts a
//! headless one (see `.github/workflows/test-coverage.yml`); those skip
//! themselves at runtime when `WAYLAND_DISPLAY` is unset so this module stays
//! green without one.

mod cli;
#[cfg(feature = "clipboard")]
mod clipboard;
#[cfg(feature = "color_picker")]
mod color_picker;
mod config;
mod listing;
#[cfg(feature = "logger")]
mod logger;
#[cfg(feature = "notifications")]
mod notification;
mod screenshot;
mod settings;
mod utils;

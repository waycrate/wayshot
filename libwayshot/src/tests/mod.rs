//! Private test module. All unit tests for libwayshot live here to keep main source files focused.

mod convert;
mod error;
mod image_util;
mod live_capture;
mod live_connection;
#[cfg(feature = "screencast")]
mod live_screencast;
mod screencopy;

#[cfg(all(test, unix))]
mod output;

#[cfg(all(test, unix))]
mod region;

#[cfg(all(test, unix))]
mod dispatch;

#[cfg(all(test, unix))]
mod lib;

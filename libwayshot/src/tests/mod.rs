//! Private test module. All unit tests for libwayshot live here to keep main source files focused.

mod convert_tests;
mod error_tests;
mod image_util_tests;
mod screencopy_tests;

#[cfg(all(test, unix))]
mod output_tests;

#[cfg(all(test, unix))]
mod region_tests;

#[cfg(all(test, unix))]
mod dispatch_tests;

#[cfg(all(test, unix))]
mod lib_tests;

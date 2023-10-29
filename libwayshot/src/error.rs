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

use std::{io, result};

use thiserror::Error;
use wayland_client::{globals::GlobalError, ConnectError, DispatchError};

pub type Result<T, E = Error> = result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("no outputs supplied")]
    NoOutputs,
    #[error("image buffer is not big enough")]
    BufferTooSmall,
    #[error("image color type not supported")]
    InvalidColor,
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("dispatch error: {0}")]
    Dispatch(#[from] DispatchError),
    #[error("global error: {0}")]
    Global(#[from] GlobalError),
    #[error("connect error: {0}")]
    Connect(#[from] ConnectError),
    #[error("framecopy failed")]
    FramecopyFailed,
    #[error("No supported buffer format")]
    NoSupportedBufferFormat,
    #[error("Cannot find required wayland protocol")]
    ProtocolNotFound(String),
}

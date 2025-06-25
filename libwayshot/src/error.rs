use std::{io, result};

use drm::buffer::UnrecognizedFourcc;
use thiserror::Error;
use wayland_client::{
    ConnectError, DispatchError,
    globals::{BindError, GlobalError},
};

pub type Result<T, E = WayshotError> = result::Result<T, E>;

/// Error type for wayshot.
#[derive(Error, Debug)]
pub enum WayshotError {
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
    #[error("bind error: {0}")]
    Bind(#[from] BindError),
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
    #[error("error occurred in freeze callback")]
    FreezeCallbackError(String),
    #[error(
        "dmabuf configuration not initialized. Did you not use Wayshot::from_connection_with_dmabuf()?"
    )]
    NoDMAStateError,
    #[error("dmabuf color format provided by compositor is invalid")]
    UnrecognizedColorCode(#[from] UnrecognizedFourcc),
    #[error("dmabuf device has been destroyed")]
    EGLError(#[from] khronos_egl::Error),
    #[error("No EGLImageTargetTexture2DOES function located, this extension may not be supported")]
    EGLImageToTexProcNotFoundError,
    #[error("Not Support format")]
    NotSupportFormat,
    #[error("Capture Failed")]
    CaptureFailed(String),
}

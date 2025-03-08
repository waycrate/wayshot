use std::{io, result};

use drm::buffer::UnrecognizedFourcc;
use thiserror::Error;
use wayland_client::{
    globals::{BindError, GlobalError},
    ConnectError, DispatchError,
};

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
    FreezeCallbackError,
    #[error("dmabuf configuration not initialized. Did you not use Wayshot::from_connection_with_dmabuf()?")]
    NoDMAStateError,
    #[error("dmabuf color format provided by compositor is invalid")]
    UnrecognizedColorCode(#[from] UnrecognizedFourcc),
    #[error("dmabuf device has been destroyed")]
    EGLError(#[from] khronos_egl::Error),
    #[error("No EGLImageTargetTexture2DOES function located, this extension may not be supported")]
    EGLImageToTexProcNotFoundError,
}

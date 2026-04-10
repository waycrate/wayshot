use std::{io, result};

use drm::buffer::UnrecognizedFourcc;
use gbm::InvalidFdError;
#[cfg(feature = "egl")]
use r_egl_wayland::r_egl as egl;
use thiserror::Error;
use wayland_client::{
    ConnectError, DispatchError, WEnum,
    globals::{BindError, GlobalError},
};
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason;

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
    #[error("framecopy failed with reason {0:?}")]
    FramecopyFailedWithReason(WEnum<FailureReason>),
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
    #[cfg(feature = "egl")]
    #[error("EGL error: {0}")]
    EGLError(#[from] egl::Error),
    #[cfg(feature = "egl")]
    #[error("No EGLImageTargetTexture2DOES function located, this extension may not be supported")]
    EGLImageToTexProcNotFoundError,
    #[error("Capture failed: {0}")]
    CaptureFailed(String),
    #[error("Unsupported for some reason: {0}")]
    Unsupported(String),
    #[error("Fd does not exist")]
    InvalidFd(#[from] InvalidFdError),
}

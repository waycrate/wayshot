use r_egl_wayland::r_egl::Error as GLError;
use std::result;
use thiserror::Error;
use wayland_client::{ConnectError, globals::GlobalError};
pub type Result<T, E = WaylandEGLStateError> = result::Result<T, E>;

#[derive(Error, Debug)]
pub enum WaylandEGLStateError {
    #[error("Shader compilation failed")]
    GLShaderCompileFailed,

    #[error("Failed to create gl program")]
    GLCreateProgramFailed,

    #[error("Failed to link gl program")]
    GLLinkProgramFailed,

    #[error("Global Error")]
    GLobalError(#[from] GlobalError),

    #[error("Connect error")]
    ConnectError(#[from] ConnectError),

    #[error("Gl Surface error")]
    GLSurfaceError(#[from] wayland_egl::Error),

    #[error("Gl cannot initlize")]
    GLInitError(#[from] GLError),
}

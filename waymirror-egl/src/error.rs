use std::result;
use thiserror::Error;

pub type Result<T, E = WaylandEGLStateError> = result::Result<T, E>;

#[derive(Error, Debug)]
pub enum WaylandEGLStateError {
    #[error("xdg_wm_base global missing")]
    XdgWmBaseMissing,

    #[error("wl_compositor global missing")]
    WlCompositorMissing,

    #[error("Shader compilation failed")]
    GLShaderCompileFailed,

    #[error("Failed to create gl program")]
    GLCreateProgramFailed,

    #[error("Failed to link gl program")]
    GLLinkProgramFailed,
}

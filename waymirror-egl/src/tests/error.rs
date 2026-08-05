use wayland_client::ConnectError;

use crate::error::WaylandEGLStateError;

#[test]
fn shader_compile_failed_message() {
    assert_eq!(
        WaylandEGLStateError::GLShaderCompileFailed.to_string(),
        "Shader compilation failed"
    );
}

#[test]
fn create_program_failed_message() {
    assert_eq!(
        WaylandEGLStateError::GLCreateProgramFailed.to_string(),
        "Failed to create gl program"
    );
}

#[test]
fn link_program_failed_message() {
    assert_eq!(
        WaylandEGLStateError::GLLinkProgramFailed.to_string(),
        "Failed to link gl program"
    );
}

#[test]
fn connect_error_converts_via_from() {
    let err: WaylandEGLStateError = ConnectError::NoCompositor.into();
    assert!(matches!(err, WaylandEGLStateError::ConnectError(_)));
    assert_eq!(err.to_string(), "Connect error");
}

#[test]
fn wayland_egl_error_converts_via_from() {
    let err: WaylandEGLStateError = wayland_egl::Error::InvalidSize.into();
    assert!(matches!(err, WaylandEGLStateError::GLSurfaceError(_)));
    assert_eq!(err.to_string(), "Gl Surface error");
}

#[test]
fn egl_error_converts_via_from() {
    let err: WaylandEGLStateError = r_egl_wayland::r_egl::Error::NotInitialized.into();
    assert!(matches!(err, WaylandEGLStateError::GLInitError(_)));
    assert_eq!(err.to_string(), "Gl cannot initlize");
}

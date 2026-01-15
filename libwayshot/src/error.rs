use std::{io, result};

use drm::buffer::UnrecognizedFourcc;
use gbm::InvalidFdError;
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
    #[error("dmabuf device has been destroyed")]
    EGLError(#[from] egl::Error),
    #[error("No EGLImageTargetTexture2DOES function located, this extension may not be supported")]
    EGLImageToTexProcNotFoundError,
    #[error("Capture failed: {0}")]
    CaptureFailed(String),
    #[error("Unsupported for some reason: {0}")]
    Unsupported(String),
    #[error("Fd does not exist")]
    InvalidFd(#[from] InvalidFdError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use drm::buffer::UnrecognizedFourcc;
    use wayland_client::{
        ConnectError, DispatchError,
        backend::{InvalidId, ObjectId, WaylandError, protocol::ProtocolError},
        globals::{BindError, GlobalError},
    };

    #[test]
    fn test_display_no_outputs() {
        let err = Error::NoOutputs;
        assert_eq!(err.to_string(), "no outputs supplied");
    }

    #[test]
    fn test_display_buffer_too_small() {
        let err = Error::BufferTooSmall;
        assert_eq!(err.to_string(), "image buffer is not big enough");
    }

    #[test]
    fn test_display_invalid_color() {
        let err = Error::InvalidColor;
        assert_eq!(err.to_string(), "image color type not supported");
    }

    #[test]
    fn test_from_io_error() {
        use std::io;
        let io_error = io::Error::other("test error");
        let wayshot_error: Error = io_error.into();

        match wayshot_error {
            Error::Io(_) => {}
            _ => panic!("Expected Error::Io(...)"),
        }
    }

    #[test]
    fn test_from_dispatch_error_bad_message() {
        let dispatch_error = DispatchError::BadMessage {
            sender_id: ObjectId::null(),
            interface: "test_interface",
            opcode: 2,
        };

        let wayshot_error: Error = dispatch_error.into();

        match wayshot_error {
            Error::Dispatch(DispatchError::BadMessage { .. }) => {}
            _ => panic!("Expected Error::Dispatch(DispatchError::BadMessage)"),
        }
    }

    #[test]
    fn test_from_dispatch_error_backend() {
        let protocol_error = ProtocolError {
            code: 1,
            object_id: 10,
            object_interface: "wl_compositor".to_string(),
            message: "Protocol error".to_string(),
        };
        let wayland_error = WaylandError::Protocol(protocol_error);
        let dispatch_error = DispatchError::Backend(wayland_error);

        let wayshot_error: Error = dispatch_error.into();

        match wayshot_error {
            Error::Dispatch(DispatchError::Backend(WaylandError::Protocol(_))) => {}
            _ => panic!("Expected Error::Dispatch(DispatchError::Backend(...))"),
        }
    }

    #[test]
    fn test_from_bind_error_uv() {
        let bind_error = BindError::UnsupportedVersion;
        let wayshot_error: Error = bind_error.into();

        match wayshot_error {
            Error::Bind(BindError::UnsupportedVersion) => {}
            _ => panic!("Expected Error::Bind(BindError::UnsupportedVersion)"),
        }
    }

    #[test]
    fn test_from_bind_error_np() {
        let bind_error = BindError::NotPresent;
        let wayshot_error: Error = bind_error.into();

        match wayshot_error {
            Error::Bind(BindError::NotPresent) => {}
            _ => panic!("Expected Error::Bind(BindError::NotPresent)"),
        }
    }

    #[test]
    fn test_from_global_backend_protocol() {
        let protocol_error = ProtocolError {
            code: 1,
            object_id: 10,
            object_interface: "wl_compositor".to_string(),
            message: "Protocol error".to_string(),
        };

        let wayland_error = WaylandError::Protocol(protocol_error);
        let global_error = GlobalError::Backend(wayland_error);

        let wayshot_error: Error = global_error.into();

        match wayshot_error {
            Error::Global(GlobalError::Backend(WaylandError::Protocol(_))) => {}
            _ => panic!("Expected Error::Global(GlobalError::Backend(...))"),
        }
    }

    #[test]
    fn test_from_global_invalid_id() {
        let invalid_struct = InvalidId;
        let global_error = GlobalError::InvalidId(invalid_struct);

        let wayshot_error: Error = global_error.into();

        match wayshot_error {
            Error::Global(GlobalError::InvalidId(_)) => {}
            _ => panic!("Expected Error::Global(GlobalError::InvalidId(...))"),
        }
    }

    #[test]
    fn test_from_connect_error_nwl() {
        let connect_error = ConnectError::NoWaylandLib;
        let wayshot_error: Error = connect_error.into();

        match wayshot_error {
            Error::Connect(ConnectError::NoWaylandLib) => {}
            _ => panic!("Expected Error::Connect(ConnectError::NoWaylandLib)"),
        }
    }

    #[test]
    fn test_from_connect_error_ncp() {
        let connect_error = ConnectError::NoCompositor;
        let wayshot_error: Error = connect_error.into();

        match wayshot_error {
            Error::Connect(ConnectError::NoCompositor) => {}
            _ => panic!("Expected Error::Connect(ConnectError::NoCompositor)"),
        }
    }

    #[test]
    fn test_from_connect_error_ifd() {
        let connect_error = ConnectError::InvalidFd;
        let wayshot_error: Error = connect_error.into();

        match wayshot_error {
            Error::Connect(ConnectError::InvalidFd) => {}
            _ => panic!("Expected Error::Connect(ConnectError::InvalidFd)"),
        }
    }

    #[test]
    fn test_display_framecopy_failed() {
        let err = Error::FramecopyFailed;
        assert_eq!(err.to_string(), "framecopy failed");
    }

    #[test]
    fn test_display_no_supported_buffer_format() {
        let err = Error::NoSupportedBufferFormat;
        assert_eq!(err.to_string(), "No supported buffer format");
    }

    #[test]
    fn test_display_protocol_not_found() {
        let err = Error::ProtocolNotFound("wl_compositor".to_string());
        assert_eq!(err.to_string(), "Cannot find required wayland protocol");
    }

    #[test]
    fn test_display_freeze_callback_error() {
        let err = Error::FreezeCallbackError("some callback info".to_string());
        assert_eq!(err.to_string(), "error occurred in freeze callback");
    }

    #[test]
    fn test_display_no_dma_state_error() {
        let err = Error::NoDMAStateError;
        let expected_msg = "dmabuf configuration not initialized. Did you not use Wayshot::from_connection_with_dmabuf()?";
        assert_eq!(err.to_string(), expected_msg);
    }

    #[test]
    fn test_from_unrecognised_fourcc() {
        let fourcc_error = UnrecognizedFourcc(42);
        let wayshot_error: Error = fourcc_error.into();

        match wayshot_error {
            Error::UnrecognizedColorCode(UnrecognizedFourcc(42)) => {}
            _ => panic!("Expected Error::UnrecognizedColorCode(UnrecognizedFourcc(42))"),
        }
    }

    #[test]
    fn test_from_egl_error() {
        let egl_error = egl::Error::ContextLost;
        let wayshot_error: Error = egl_error.into();

        match wayshot_error {
            Error::EGLError(egl::Error::ContextLost) => {}
            _ => panic!("Expected Error::EGLError(khronos_egl::Error::ContextLost)"),
        }
    }
}

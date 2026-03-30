//! Private test module. All unit tests for libwayshot live here to keep main source files focused.

mod error_tests {
    use crate::error::Error;
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
        let io_error = std::io::Error::other("test error");
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

    #[cfg(feature = "egl")]
    #[test]
    fn test_from_egl_error() {
        use r_egl_wayland::r_egl as egl;
        let egl_error = egl::Error::ContextLost;
        let wayshot_error: Error = egl_error.into();
        match wayshot_error {
            Error::EGLError(egl::Error::ContextLost) => {}
            _ => panic!("Expected Error::EGLError(khronos_egl::Error::ContextLost)"),
        }
    }

    #[test]
    fn test_display_framecopy_failed_with_reason() {
        use wayland_client::WEnum;
        use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason;
        let reason = WEnum::<FailureReason>::Unknown(1);
        let err = Error::FramecopyFailedWithReason(reason);
        assert!(err.to_string().contains("framecopy failed with reason"));
    }

    #[test]
    fn test_display_capture_failed() {
        let err = Error::CaptureFailed("test capture error".to_string());
        assert_eq!(err.to_string(), "Capture failed: test capture error");
    }

    #[test]
    fn test_display_unsupported() {
        let err = Error::Unsupported("reason".to_string());
        assert_eq!(err.to_string(), "Unsupported for some reason: reason");
    }

    #[cfg(feature = "egl")]
    #[test]
    fn test_display_egl_image_to_tex_proc_not_found() {
        let err = Error::EGLImageToTexProcNotFoundError;
        assert!(err.to_string().contains("EGLImageTargetTexture2DOES"));
    }
}

#[cfg(test)]
mod convert_tests {
    use crate::convert::{Convert, create_converter};
    use image::ColorType;
    use wayland_client::protocol::wl_shm;

    #[test]
    fn create_converter_returns_none_for_unknown_format() {
        // Argb2101010 is not in the supported list (we support Abgr2101010, Xbgr2101010)
        let unsupported = wl_shm::Format::Argb2101010;
        assert!(create_converter(unsupported).is_none());
    }

    #[test]
    fn create_converter_xbgr8888_returns_some() {
        assert!(create_converter(wl_shm::Format::Xbgr8888).is_some());
        assert!(create_converter(wl_shm::Format::Abgr8888).is_some());
    }

    #[test]
    fn create_converter_xrgb8888_returns_some() {
        assert!(create_converter(wl_shm::Format::Xrgb8888).is_some());
        assert!(create_converter(wl_shm::Format::Argb8888).is_some());
    }

    #[test]
    fn create_converter_bgr10_returns_some() {
        assert!(create_converter(wl_shm::Format::Xbgr2101010).is_some());
        assert!(create_converter(wl_shm::Format::Abgr2101010).is_some());
    }

    #[test]
    fn create_converter_bgr888_returns_some() {
        assert!(create_converter(wl_shm::Format::Bgr888).is_some());
    }

    #[test]
    fn convert_none_produces_rgba8() {
        let converter: Box<dyn Convert> = create_converter(wl_shm::Format::Xbgr8888).unwrap();
        let mut data = vec![0x11, 0x22, 0x33, 0x44];
        let out = converter.convert_inplace(&mut data);
        assert_eq!(out, ColorType::Rgba8);
        assert_eq!(data, vec![0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn convert_rgb8_swaps_r_and_b() {
        let converter = create_converter(wl_shm::Format::Xrgb8888).unwrap();
        let mut data = vec![0x11, 0x22, 0x33, 0x44];
        let out = converter.convert_inplace(&mut data);
        assert_eq!(out, ColorType::Rgba8);
        assert_eq!(data[0], 0x33);
        assert_eq!(data[1], 0x22);
        assert_eq!(data[2], 0x11);
        assert_eq!(data[3], 0x44);
    }

    #[test]
    fn convert_rgb8_multiple_pixels() {
        let converter = create_converter(wl_shm::Format::Argb8888).unwrap();
        let mut data = vec![0x11, 0x22, 0x33, 0x44, 0xaa, 0xbb, 0xcc, 0xdd];
        converter.convert_inplace(&mut data);
        assert_eq!(data[0], 0x33);
        assert_eq!(data[2], 0x11);
        assert_eq!(data[4], 0xcc);
        assert_eq!(data[6], 0xaa);
    }

    #[test]
    fn convert_bgr10_produces_rgba8() {
        let converter = create_converter(wl_shm::Format::Abgr2101010).unwrap();
        let mut data = vec![0x00, 0x00, 0x00, 0xFF];
        let out = converter.convert_inplace(&mut data);
        assert_eq!(out, ColorType::Rgba8);
        assert_eq!(data[3], 255);
    }

    #[test]
    fn convert_bgr888_produces_rgb8() {
        let converter = create_converter(wl_shm::Format::Bgr888).unwrap();
        let mut data = vec![0x01, 0x02, 0x03];
        let out = converter.convert_inplace(&mut data);
        assert_eq!(out, ColorType::Rgb8);
    }
}

#[cfg(test)]
mod screencopy_tests {
    use crate::region::Size;
    use crate::screencopy::FrameFormat;
    use wayland_client::protocol::wl_shm;

    #[test]
    fn frame_format_byte_size() {
        let format = FrameFormat {
            format: wl_shm::Format::Argb8888,
            size: Size {
                width: 100,
                height: 200,
            },
            stride: 400,
        };
        assert_eq!(format.byte_size(), 400 * 200);
    }

    #[test]
    fn frame_format_byte_size_small() {
        let format = FrameFormat {
            format: wl_shm::Format::Xrgb8888,
            size: Size {
                width: 2,
                height: 2,
            },
            stride: 8,
        };
        assert_eq!(format.byte_size(), 16);
    }
}

#[cfg(test)]
mod image_util_tests {
    use crate::image_util::rotate_image_buffer;
    use crate::region::Size;
    use image::{DynamicImage, ImageBuffer, RgbaImage};
    use wayland_client::protocol::wl_output::Transform;

    fn make_image(w: u32, h: u32) -> DynamicImage {
        let buf: RgbaImage =
            ImageBuffer::from_raw(w, h, (0..w * h * 4).map(|i| i as u8).collect()).unwrap();
        DynamicImage::ImageRgba8(buf)
    }

    #[test]
    fn rotate_image_buffer_normal_unchanged() {
        let image = make_image(10, 20);
        let logical_size = Size {
            width: 10,
            height: 20,
        };
        let out = rotate_image_buffer(image, Transform::Normal, logical_size, 1.0);
        assert_eq!(out.width(), 10);
        assert_eq!(out.height(), 20);
    }

    #[test]
    fn rotate_image_buffer_90_swaps_dimensions() {
        let image = make_image(10, 20);
        let logical_size = Size {
            width: 10,
            height: 20,
        };
        let out = rotate_image_buffer(image, Transform::_90, logical_size, 2.0);
        assert_eq!(out.width(), 20);
        assert_eq!(out.height(), 10);
    }

    #[test]
    fn rotate_image_buffer_180_same_dimensions() {
        let image = make_image(8, 6);
        let logical_size = Size {
            width: 8,
            height: 6,
        };
        let out = rotate_image_buffer(image, Transform::_180, logical_size, 1.0);
        assert_eq!(out.width(), 8);
        assert_eq!(out.height(), 6);
    }

    #[test]
    fn rotate_image_buffer_270_swaps_dimensions() {
        let image = make_image(12, 14);
        let logical_size = Size {
            width: 12,
            height: 14,
        };
        let out = rotate_image_buffer(image, Transform::_270, logical_size, 1.0);
        assert_eq!(out.width(), 14);
        assert_eq!(out.height(), 12);
    }

    #[test]
    fn rotate_image_buffer_flipped_same_dimensions() {
        let image = make_image(5, 5);
        let logical_size = Size {
            width: 5,
            height: 5,
        };
        let out = rotate_image_buffer(image, Transform::Flipped, logical_size, 1.0);
        assert_eq!(out.width(), 5);
        assert_eq!(out.height(), 5);
    }
}

#[cfg(all(test, unix))]
mod output_tests {
    use crate::output::OutputInfo;
    use crate::region::{LogicalRegion, Position, Region, Size};
    use std::mem;
    use std::os::unix::net::UnixStream;
    use wayland_backend::client::Backend;
    use wayland_client::Proxy;
    use wayland_client::protocol::wl_output::WlOutput;

    fn make_output_info(
        name: &str,
        description: &str,
        physical_size: Size,
        logical_region: LogicalRegion,
    ) -> OutputInfo {
        OutputInfo {
            wl_output: dummy_wl_output(),
            name: name.to_string(),
            description: description.to_string(),
            transform: wayland_client::protocol::wl_output::Transform::Normal,
            physical_size,
            logical_region,
        }
    }

    fn dummy_wl_output() -> WlOutput {
        let (client, server) = UnixStream::pair().expect("unix stream");
        Box::leak(Box::new(server));
        let backend = Backend::connect(client).expect("backend");
        let weak = backend.downgrade();
        Box::leak(Box::new(backend));
        WlOutput::inert(weak)
    }

    #[test]
    fn display_formats_name_and_description() {
        let output_info = make_output_info(
            "HDMI-A-1",
            "Main Display",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        assert_eq!(output_info.to_string(), "HDMI-A-1 (Main Display)");
        mem::forget(output_info);
    }

    #[test]
    fn display_formats_empty_name_and_description() {
        let output_info = make_output_info(
            "",
            "",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        assert_eq!(output_info.to_string(), " ()");
        mem::forget(output_info);
    }

    #[test]
    fn scale_returns_ratio_between_physical_and_logical_heights() {
        let output_info = make_output_info(
            "DP-1",
            "Secondary Display",
            Size {
                width: 3840,
                height: 2160,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        assert_eq!(output_info.scale(), 2.0);
        mem::forget(output_info);
    }

    #[test]
    fn scale_returns_correct_ratio_for_various_sizes() {
        let o1 = make_output_info(
            "eDP-1",
            "Laptop Screen",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        assert_eq!(o1.scale(), 1.0);
        mem::forget(o1);

        let o15 = make_output_info(
            "DP-2",
            "HiDPI Display",
            Size {
                width: 3840,
                height: 2160,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 2560,
                        height: 1440,
                    },
                },
            },
        );
        assert_eq!(o15.scale(), 1.5);
        mem::forget(o15);
    }

    #[test]
    fn debug_format() {
        let output_info = make_output_info(
            "HDMI-1",
            "Debug Display",
            Size {
                width: 800,
                height: 600,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 800,
                        height: 600,
                    },
                },
            },
        );
        let debug_str = format!("{:?}", output_info);
        assert!(debug_str.contains("OutputInfo"));
        assert!(debug_str.contains("HDMI-1"));
        assert!(debug_str.contains("Debug Display"));
        assert!(debug_str.contains("800"));
        assert!(debug_str.contains("600"));
        mem::forget(output_info);
    }

    #[test]
    fn clone_and_partial_eq() {
        let o1 = make_output_info(
            "HDMI-1",
            "Clone Display",
            Size {
                width: 1024,
                height: 768,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1024,
                        height: 768,
                    },
                },
            },
        );
        let o2 = o1.clone();
        assert_eq!(o1, o2);
        assert_eq!(o1.name, o2.name);
        assert_eq!(o1.description, o2.description);
        assert_eq!(o1.physical_size, o2.physical_size);
        assert_eq!(o1.logical_region, o2.logical_region);
        mem::forget(o1);
        mem::forget(o2);
    }

    #[test]
    fn physical_size_returns_physical_size() {
        let output_info = make_output_info(
            "DP-1",
            "Display",
            Size {
                width: 3840,
                height: 2160,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 100, y: 50 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        let phys = output_info.physical_size();
        assert_eq!(phys.width, 3840);
        assert_eq!(phys.height, 2160);
        mem::forget(output_info);
    }

    #[test]
    fn logical_size_returns_logical_size() {
        let output_info = make_output_info(
            "eDP-1",
            "Display",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 960,
                        height: 540,
                    },
                },
            },
        );
        let log_size = output_info.logical_size();
        assert_eq!(log_size.width, 960);
        assert_eq!(log_size.height, 540);
        mem::forget(output_info);
    }

    #[test]
    fn logical_position_returns_position() {
        let output_info = make_output_info(
            "HDMI-1",
            "Display",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: -100, y: 200 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        let pos = output_info.logical_position();
        assert_eq!(pos.x, -100);
        assert_eq!(pos.y, 200);
        mem::forget(output_info);
    }

    #[test]
    fn wayshot_target_from_output_info() {
        use crate::WayshotTarget;
        let output_info = make_output_info(
            "HDMI-1",
            "Display",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        let target = WayshotTarget::from(output_info);
        match &target {
            crate::WayshotTarget::Screen(wl_output) => {
                let _ = wl_output.version();
            }
            crate::WayshotTarget::Toplevel(_) => panic!("Expected Screen variant"),
        }
        mem::forget(target);
    }

    #[test]
    fn output_info_as_ref_returns_wl_output() {
        let output_info = make_output_info(
            "HDMI-1",
            "Display",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        let wl_ref = output_info.as_ref();
        assert!(std::ptr::eq(wl_ref, &output_info.wl_output));
        mem::forget(output_info);
    }
}

#[cfg(all(test, unix))]
mod region_tests {
    use crate::error::Error;
    use crate::output::OutputInfo;
    use crate::region::{EmbeddedRegion, LogicalRegion, Position, Region, Size};
    use std::mem;
    use std::os::unix::net::UnixStream;
    use wayland_backend::client::Backend;
    use wayland_client::Proxy;
    use wayland_client::protocol::wl_output::WlOutput;

    fn make_output(name: &str, position: Position, size: Size) -> OutputInfo {
        OutputInfo {
            wl_output: dummy_wl_output(),
            name: name.to_string(),
            description: format!("{name} description"),
            transform: wayland_client::protocol::wl_output::Transform::Normal,
            physical_size: size,
            logical_region: LogicalRegion {
                inner: Region { position, size },
            },
        }
    }

    fn dummy_wl_output() -> WlOutput {
        let (client, server) = UnixStream::pair().expect("unix stream");
        Box::leak(Box::new(server));
        let backend = Backend::connect(client).expect("backend");
        let weak = backend.downgrade();
        Box::leak(Box::new(backend));
        WlOutput::inert(weak)
    }

    #[test]
    fn embedded_region_new_clamps_to_relative_bounds() {
        let viewport = LogicalRegion {
            inner: Region {
                position: Position { x: 5, y: -5 },
                size: Size {
                    width: 20,
                    height: 20,
                },
            },
        };
        let relative_to = LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 15,
                    height: 10,
                },
            },
        };
        let embedded = EmbeddedRegion::new(viewport, relative_to).expect("should be clamped");
        assert_eq!(
            embedded.inner,
            Region {
                position: Position { x: 5, y: 0 },
                size: Size {
                    width: 10,
                    height: 10
                }
            }
        );
    }

    #[test]
    fn embedded_region_new_returns_none_when_outside() {
        let viewport = LogicalRegion {
            inner: Region {
                position: Position { x: 20, y: 20 },
                size: Size {
                    width: 5,
                    height: 5,
                },
            },
        };
        let relative_to = LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 10,
                    height: 10,
                },
            },
        };
        assert!(EmbeddedRegion::new(viewport, relative_to).is_none());
    }

    #[test]
    fn embedded_region_logical_restores_absolute_coordinates() {
        let relative_to = LogicalRegion {
            inner: Region {
                position: Position { x: 10, y: 15 },
                size: Size {
                    width: 100,
                    height: 100,
                },
            },
        };
        let embedded = EmbeddedRegion {
            relative_to,
            inner: Region {
                position: Position { x: 5, y: 5 },
                size: Size {
                    width: 20,
                    height: 30,
                },
            },
        };
        let logical = embedded.logical();
        assert_eq!(
            logical,
            LogicalRegion {
                inner: Region {
                    position: Position { x: 15, y: 20 },
                    size: Size {
                        width: 20,
                        height: 30
                    }
                }
            }
        );
    }

    #[test]
    fn display_formatters_match_expected_layout() {
        let position = Position { x: -5, y: 10 };
        let size = Size {
            width: 42,
            height: 24,
        };
        let region = Region { position, size };
        let logical = LogicalRegion { inner: region };
        let embedded = EmbeddedRegion {
            relative_to: logical,
            inner: region,
        };
        assert_eq!(position.to_string(), "(-5, 10)");
        assert_eq!(size.to_string(), "(42x24)");
        assert_eq!(region.to_string(), "(-5, 10) (42x24)");
        assert_eq!(logical.to_string(), "(-5, 10) (42x24)");
        assert_eq!(
            embedded.to_string(),
            "(-5, 10) (42x24) relative to (-5, 10) (42x24)"
        );
    }

    #[test]
    fn logical_region_from_output_copies_inner_region() {
        let output = make_output(
            "primary",
            Position { x: 100, y: 50 },
            Size {
                width: 1920,
                height: 1080,
            },
        );
        let logical = LogicalRegion::from(&output);
        assert_eq!(logical.inner.position.x, 100);
        assert_eq!(logical.inner.position.y, 50);
        assert_eq!(logical.inner.size.width, 1920);
        assert_eq!(logical.inner.size.height, 1080);
        mem::forget(output);
    }

    #[test]
    fn logical_region_try_from_outputs_spans_all_outputs() {
        let mut outputs = vec![
            make_output(
                "A",
                Position { x: 0, y: 0 },
                Size {
                    width: 1920,
                    height: 1080,
                },
            ),
            make_output(
                "B",
                Position { x: 1920, y: -100 },
                Size {
                    width: 1280,
                    height: 1024,
                },
            ),
        ];
        let logical = LogicalRegion::try_from(outputs.as_slice()).expect("valid slice");
        assert_eq!(logical.inner.position.x, 0);
        assert_eq!(logical.inner.position.y, -100);
        assert_eq!(logical.inner.size.width, 1920 + 1280);
        assert_eq!(logical.inner.size.height, 1180);
        for output in outputs.drain(..) {
            mem::forget(output);
        }
    }

    #[test]
    fn logical_region_try_from_empty_slice_errors() {
        let empty: [OutputInfo; 0] = [];
        let err = LogicalRegion::try_from(empty.as_slice()).unwrap_err();
        match err {
            Error::NoOutputs => {}
            _ => panic!("expected Error::NoOutputs"),
        }
    }

    #[test]
    fn position_size_region_logical_region_default() {
        let pos = Position::default();
        assert_eq!(pos.x, 0);
        assert_eq!(pos.y, 0);

        let size = Size::default();
        assert_eq!(size.width, 0);
        assert_eq!(size.height, 0);

        let region = Region::default();
        assert_eq!(region.position, pos);
        assert_eq!(region.size, size);

        let logical = LogicalRegion::default();
        assert_eq!(logical.inner, region);
    }

    #[test]
    fn embedded_region_exact_fit() {
        let viewport = LogicalRegion {
            inner: Region {
                position: Position { x: 10, y: 20 },
                size: Size {
                    width: 100,
                    height: 50,
                },
            },
        };
        let relative_to = LogicalRegion {
            inner: Region {
                position: Position { x: 10, y: 20 },
                size: Size {
                    width: 100,
                    height: 50,
                },
            },
        };
        let embedded = EmbeddedRegion::new(viewport, relative_to).expect("exact fit");
        assert_eq!(embedded.inner.position.x, 0);
        assert_eq!(embedded.inner.position.y, 0);
        assert_eq!(embedded.inner.size.width, 100);
        assert_eq!(embedded.inner.size.height, 50);
    }

    #[test]
    fn embedded_region_fully_inside() {
        let viewport = LogicalRegion {
            inner: Region {
                position: Position { x: 5, y: 5 },
                size: Size {
                    width: 10,
                    height: 10,
                },
            },
        };
        let relative_to = LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 100,
                    height: 100,
                },
            },
        };
        let embedded = EmbeddedRegion::new(viewport, relative_to).expect("fully inside");
        assert_eq!(embedded.inner.position.x, 5);
        assert_eq!(embedded.inner.position.y, 5);
        assert_eq!(embedded.inner.size.width, 10);
        assert_eq!(embedded.inner.size.height, 10);
    }

    #[test]
    fn logical_region_from_output_ref() {
        let output = make_output(
            "primary",
            Position { x: 42, y: 43 },
            Size {
                width: 1920,
                height: 1080,
            },
        );
        let logical = LogicalRegion::from(&output);
        assert_eq!(logical.inner.position.x, 42);
        assert_eq!(logical.inner.position.y, 43);
        assert_eq!(logical.inner.size.width, 1920);
        assert_eq!(logical.inner.size.height, 1080);
        mem::forget(output);
    }
}

#[cfg(all(test, unix))]
mod dispatch_tests {
    use crate::dispatch::{CaptureFrameState, Card};

    #[test]
    fn card_open_nonexistent_path_errors() {
        let err = Card::open("/nonexistent/dri/renderD999").err().unwrap();
        assert!(err.kind() == std::io::ErrorKind::NotFound);
    }

    #[test]
    fn capture_frame_state_new_with_gbm() {
        let state = CaptureFrameState::new(true);
        assert!(state.formats.is_empty());
        assert!(state.dmabuf_formats.is_empty());
    }

    #[test]
    fn capture_frame_state_new_without_gbm() {
        let state = CaptureFrameState::new(false);
        assert!(state.formats.is_empty());
        assert!(state.dmabuf_formats.is_empty());
    }
}

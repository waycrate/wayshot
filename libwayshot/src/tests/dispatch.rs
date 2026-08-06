use crate::dispatch::{CaptureFrameState, FrameState, LayerShellState, OutputCaptureState};
use crate::output::OutputInfo;
use crate::region::{LogicalRegion, Position, Size, TopLevel};
use crate::screencopy::{DMAFrameFormat, FrameFormat};
use std::os::unix::net::UnixStream;
use wayland_backend::client::Backend;
use wayland_client::protocol::{
    wl_output::{self, WlOutput},
    wl_registry::{self, WlRegistry},
    wl_shm,
};
use wayland_client::{Connection, Dispatch, Proxy, WEnum};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1, FailureReason},
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{self, ZxdgOutputV1};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, ZwlrLayerSurfaceV1,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{
    self, ZwlrScreencopyFrameV1,
};

fn dummy_conn() -> Connection {
    let (client, server) = UnixStream::pair().expect("unix stream");
    Box::leak(Box::new(server));
    let backend = Backend::connect(client).expect("backend");
    Connection::from_backend(backend)
}

fn inert<T: Proxy>(conn: &Connection) -> T {
    T::inert(conn.backend().downgrade())
}

fn default_output_info(wl_output: WlOutput) -> OutputInfo {
    OutputInfo {
        wl_output,
        name: String::new(),
        description: String::new(),
        transform: wl_output::Transform::Normal,
        physical_size: Size::default(),
        logical_region: LogicalRegion::default(),
    }
}

#[cfg(feature = "dmabuf")]
#[test]
fn card_open_nonexistent_path_errors() {
    use crate::dispatch::Card;
    let err = Card::open("/nonexistent/dri/renderD999").err().unwrap();
    assert!(err.kind() == std::io::ErrorKind::NotFound);
}

#[cfg(feature = "dmabuf")]
#[test]
fn card_as_fd_returns_valid_fd() {
    use crate::dispatch::Card;
    use std::os::fd::{AsFd, AsRawFd};

    // /dev/null is always present and read/write-able, so this stays CI-safe
    // without needing a real DRM device.
    let card = Card::open("/dev/null").expect("open /dev/null");
    assert!(card.as_fd().as_raw_fd() >= 0);
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

#[test]
fn capture_frame_state_initial_state_is_none() {
    let state = CaptureFrameState::new(false);
    assert!(state.state.is_none());
    assert!(state.toplevels.is_empty());
}

#[test]
fn frame_state_variants_and_equality() {
    assert_eq!(FrameState::Failed, FrameState::Failed);
    assert_eq!(FrameState::Finished, FrameState::Finished);
    assert_ne!(FrameState::Failed, FrameState::Finished);

    let reason = WEnum::<FailureReason>::Unknown(42);
    let s1 = FrameState::FailedWithReason(reason);
    let s2 = FrameState::FailedWithReason(WEnum::<FailureReason>::Unknown(42));
    assert_eq!(s1, s2);
    assert_ne!(s1, FrameState::Failed);
}

// --- OutputCaptureState + WlRegistry ---

#[test]
fn registry_global_wl_output_v4_registers_output() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let registry: WlRegistry = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: Vec::new(),
    };
    <OutputCaptureState as Dispatch<WlRegistry, ()>>::event(
        &mut state,
        &registry,
        wl_registry::Event::Global {
            name: 1,
            interface: "wl_output".to_string(),
            version: 4,
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.outputs.len(), 1);
    std::mem::forget(state);
}

#[test]
fn registry_global_wl_output_below_v4_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let registry: WlRegistry = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: Vec::new(),
    };
    <OutputCaptureState as Dispatch<WlRegistry, ()>>::event(
        &mut state,
        &registry,
        wl_registry::Event::Global {
            name: 1,
            interface: "wl_output".to_string(),
            version: 3,
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.outputs.is_empty());
}

#[test]
fn registry_global_other_interface_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let registry: WlRegistry = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: Vec::new(),
    };
    <OutputCaptureState as Dispatch<WlRegistry, ()>>::event(
        &mut state,
        &registry,
        wl_registry::Event::Global {
            name: 1,
            interface: "wl_compositor".to_string(),
            version: 4,
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.outputs.is_empty());
}

#[test]
fn registry_global_remove_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let registry: WlRegistry = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: Vec::new(),
    };
    <OutputCaptureState as Dispatch<WlRegistry, ()>>::event(
        &mut state,
        &registry,
        wl_registry::Event::GlobalRemove { name: 1 },
        &(),
        &conn,
        &qh,
    );
    assert!(state.outputs.is_empty());
}

// --- OutputCaptureState + WlOutput ---

#[test]
fn wl_output_event_for_unregistered_output_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: Vec::new(),
    };
    <OutputCaptureState as Dispatch<WlOutput, ()>>::event(
        &mut state,
        &wl_output,
        wl_output::Event::Name {
            name: "eDP-1".to_string(),
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.outputs.is_empty());
    std::mem::forget(wl_output);
}

#[test]
fn wl_output_name_event_sets_name() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output.clone())],
    };
    <OutputCaptureState as Dispatch<WlOutput, ()>>::event(
        &mut state,
        &wl_output,
        wl_output::Event::Name {
            name: "eDP-1".to_string(),
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.outputs[0].name, "eDP-1");
    std::mem::forget(state);
}

#[test]
fn wl_output_description_event_sets_description() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output.clone())],
    };
    <OutputCaptureState as Dispatch<WlOutput, ()>>::event(
        &mut state,
        &wl_output,
        wl_output::Event::Description {
            description: "Laptop screen".to_string(),
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.outputs[0].description, "Laptop screen");
    std::mem::forget(state);
}

#[test]
fn wl_output_mode_event_sets_physical_size() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output.clone())],
    };
    <OutputCaptureState as Dispatch<WlOutput, ()>>::event(
        &mut state,
        &wl_output,
        wl_output::Event::Mode {
            flags: WEnum::Value(wl_output::Mode::Current),
            width: 1920,
            height: 1080,
            refresh: 60000,
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.outputs[0].physical_size.width, 1920);
    assert_eq!(state.outputs[0].physical_size.height, 1080);
    std::mem::forget(state);
}

#[test]
fn wl_output_geometry_event_sets_transform() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output.clone())],
    };
    <OutputCaptureState as Dispatch<WlOutput, ()>>::event(
        &mut state,
        &wl_output,
        wl_output::Event::Geometry {
            x: 0,
            y: 0,
            physical_width: 300,
            physical_height: 200,
            subpixel: WEnum::Value(wl_output::Subpixel::Unknown),
            make: "Make".to_string(),
            model: "Model".to_string(),
            transform: WEnum::Value(wl_output::Transform::_90),
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.outputs[0].transform, wl_output::Transform::_90);
    std::mem::forget(state);
}

#[test]
fn wl_output_geometry_unknown_transform_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output.clone())],
    };
    <OutputCaptureState as Dispatch<WlOutput, ()>>::event(
        &mut state,
        &wl_output,
        wl_output::Event::Geometry {
            x: 0,
            y: 0,
            physical_width: 300,
            physical_height: 200,
            subpixel: WEnum::Value(wl_output::Subpixel::Unknown),
            make: "Make".to_string(),
            model: "Model".to_string(),
            transform: WEnum::Unknown(9999),
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.outputs[0].transform, wl_output::Transform::Normal);
    std::mem::forget(state);
}

#[test]
fn wl_output_scale_and_done_events_are_noop() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output.clone())],
    };
    <OutputCaptureState as Dispatch<WlOutput, ()>>::event(
        &mut state,
        &wl_output,
        wl_output::Event::Scale { factor: 2 },
        &(),
        &conn,
        &qh,
    );
    <OutputCaptureState as Dispatch<WlOutput, ()>>::event(
        &mut state,
        &wl_output,
        wl_output::Event::Done,
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.outputs.len(), 1);
    std::mem::forget(state);
}

// --- OutputCaptureState + ZxdgOutputV1 ---

#[test]
fn zxdg_output_event_for_unregistered_index_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let xdg_output: ZxdgOutputV1 = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: Vec::new(),
    };
    <OutputCaptureState as Dispatch<ZxdgOutputV1, usize>>::event(
        &mut state,
        &xdg_output,
        zxdg_output_v1::Event::LogicalPosition { x: 1, y: 2 },
        &0,
        &conn,
        &qh,
    );
    assert!(state.outputs.is_empty());
}

#[test]
fn zxdg_output_logical_position_sets_position() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let xdg_output: ZxdgOutputV1 = inert(&conn);
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output)],
    };
    <OutputCaptureState as Dispatch<ZxdgOutputV1, usize>>::event(
        &mut state,
        &xdg_output,
        zxdg_output_v1::Event::LogicalPosition { x: 10, y: -5 },
        &0,
        &conn,
        &qh,
    );
    assert_eq!(
        state.outputs[0].logical_region.inner.position,
        Position { x: 10, y: -5 }
    );
    std::mem::forget(state);
}

#[test]
fn zxdg_output_logical_size_sets_size() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let xdg_output: ZxdgOutputV1 = inert(&conn);
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output)],
    };
    <OutputCaptureState as Dispatch<ZxdgOutputV1, usize>>::event(
        &mut state,
        &xdg_output,
        zxdg_output_v1::Event::LogicalSize {
            width: 1920,
            height: 1080,
        },
        &0,
        &conn,
        &qh,
    );
    assert_eq!(
        state.outputs[0].logical_region.inner.size,
        Size {
            width: 1920,
            height: 1080
        }
    );
    std::mem::forget(state);
}

#[test]
fn zxdg_output_done_name_description_are_noop() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<OutputCaptureState>().handle();
    let xdg_output: ZxdgOutputV1 = inert(&conn);
    let wl_output: WlOutput = inert(&conn);
    let mut state = OutputCaptureState {
        outputs: vec![default_output_info(wl_output)],
    };
    for event in [
        zxdg_output_v1::Event::Done,
        zxdg_output_v1::Event::Name {
            name: "eDP-1".to_string(),
        },
        zxdg_output_v1::Event::Description {
            description: "desc".to_string(),
        },
    ] {
        <OutputCaptureState as Dispatch<ZxdgOutputV1, usize>>::event(
            &mut state,
            &xdg_output,
            event,
            &0,
            &conn,
            &qh,
        );
    }
    assert_eq!(state.outputs.len(), 1);
    std::mem::forget(state);
}

// --- CaptureFrameState + ExtImageCopyCaptureFrameV1 ---

#[test]
fn ext_frame_ready_sets_finished() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ExtImageCopyCaptureFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureFrameV1, ()>>::event(
        &mut state,
        &frame,
        ext_image_copy_capture_frame_v1::Event::Ready,
        &(),
        &conn,
        &qh,
    );
    assert!(state.buffer_done.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(state.state, Some(FrameState::Finished));
}

#[test]
fn ext_frame_failed_sets_failed_with_reason() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ExtImageCopyCaptureFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureFrameV1, ()>>::event(
        &mut state,
        &frame,
        ext_image_copy_capture_frame_v1::Event::Failed {
            reason: WEnum::Value(FailureReason::Stopped),
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.buffer_done.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(
        state.state,
        Some(FrameState::FailedWithReason(WEnum::Value(
            FailureReason::Stopped
        )))
    );
}

#[test]
fn ext_frame_transform_sets_transform() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ExtImageCopyCaptureFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureFrameV1, ()>>::event(
        &mut state,
        &frame,
        ext_image_copy_capture_frame_v1::Event::Transform {
            transform: WEnum::Value(wl_output::Transform::Flipped180),
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.transform, Some(wl_output::Transform::Flipped180));
}

#[test]
fn ext_frame_transform_unknown_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ExtImageCopyCaptureFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureFrameV1, ()>>::event(
        &mut state,
        &frame,
        ext_image_copy_capture_frame_v1::Event::Transform {
            transform: WEnum::Unknown(9999),
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.transform, None);
}

#[test]
fn ext_frame_damage_is_noop() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ExtImageCopyCaptureFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureFrameV1, ()>>::event(
        &mut state,
        &frame,
        ext_image_copy_capture_frame_v1::Event::Damage {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.state.is_none());
}

// --- CaptureFrameState + ExtImageCopyCaptureSessionV1 ---

#[test]
fn session_buffer_size_updates_formats_and_dmabuf_formats() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    state.formats.push(FrameFormat {
        format: wl_shm::Format::Argb8888,
        size: Size::default(),
        stride: 0,
    });
    state.dmabuf_formats.push(DMAFrameFormat {
        format: 0,
        size: Size::default(),
    });
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::BufferSize {
            width: 1920,
            height: 1080,
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(
        state.formats[0].size,
        Size {
            width: 1920,
            height: 1080
        }
    );
    assert_eq!(state.formats[0].stride, 4 * 1920);
    assert_eq!(
        state.dmabuf_formats[0].size,
        Size {
            width: 1920,
            height: 1080
        }
    );
}

#[test]
fn session_shm_format_pushes_format() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::ShmFormat {
            format: WEnum::Value(wl_shm::Format::Xbgr8888),
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.formats.len(), 1);
    assert_eq!(state.formats[0].format, wl_shm::Format::Xbgr8888);
}

#[test]
fn session_shm_format_unknown_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::ShmFormat {
            format: WEnum::Unknown(9999),
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.formats.is_empty());
}

#[test]
fn session_dmabuf_format_pushes_dma_format() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::DmabufFormat {
            format: 0x34325241,
            modifiers: vec![],
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.dmabuf_formats.len(), 1);
    assert_eq!(state.dmabuf_formats[0].format, 0x34325241);
}

#[test]
fn session_done_sets_session_done() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::Done,
        &(),
        &conn,
        &qh,
    );
    assert!(state.session_done);
    assert!(state.state.is_none());
}

#[test]
fn session_stopped_sets_failed() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::Stopped,
        &(),
        &conn,
        &qh,
    );
    assert!(state.session_done);
    assert_eq!(state.state, Some(FrameState::Failed));
}

#[test]
fn session_dmabuf_device_without_find_gbm_is_noop() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    // find_gbm=false, so the dmabuf feature path (if any) short-circuits immediately.
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::DmabufDevice { device: vec![0; 8] },
        &(),
        &conn,
        &qh,
    );
    #[cfg(feature = "dmabuf")]
    assert!(state.gbm.is_none());
}

#[cfg(feature = "dmabuf")]
#[test]
fn session_dmabuf_device_with_invalid_length_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    let mut state = CaptureFrameState::new(true);
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::DmabufDevice {
            device: vec![1, 2, 3],
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.gbm.is_none());
}

#[cfg(feature = "dmabuf")]
#[test]
fn session_dmabuf_device_with_unknown_drm_node_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let session: ExtImageCopyCaptureSessionV1 = inert(&conn);
    let mut state = CaptureFrameState::new(true);
    // A device id that will not resolve to a real DRM node on any CI machine.
    <CaptureFrameState as Dispatch<ExtImageCopyCaptureSessionV1, ()>>::event(
        &mut state,
        &session,
        ext_image_copy_capture_session_v1::Event::DmabufDevice {
            device: u64::MAX.to_le_bytes().to_vec(),
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.gbm.is_none());
}

// --- CaptureFrameState + ExtForeignToplevelListV1 ---

#[test]
fn toplevel_list_toplevel_event_pushes_toplevel() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let list: ExtForeignToplevelListV1 = inert(&conn);
    let handle: ExtForeignToplevelHandleV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtForeignToplevelListV1, ()>>::event(
        &mut state,
        &list,
        ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel: handle },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.toplevels.len(), 1);
    std::mem::forget(state);
}

#[test]
fn toplevel_list_finished_is_noop() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let list: ExtForeignToplevelListV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtForeignToplevelListV1, ()>>::event(
        &mut state,
        &list,
        ext_foreign_toplevel_list_v1::Event::Finished,
        &(),
        &conn,
        &qh,
    );
    assert!(state.toplevels.is_empty());
}

// --- CaptureFrameState + ExtForeignToplevelHandleV1 ---

#[test]
fn toplevel_handle_event_for_unregistered_toplevel_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let handle: ExtForeignToplevelHandleV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ExtForeignToplevelHandleV1, ()>>::event(
        &mut state,
        &handle,
        ext_foreign_toplevel_handle_v1::Event::Title {
            title: "Ignored".to_string(),
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.toplevels.is_empty());
    std::mem::forget(handle);
}

#[test]
fn toplevel_handle_events_update_matching_toplevel() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let handle: ExtForeignToplevelHandleV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    state.toplevels.push(TopLevel::new(handle.clone()));

    <CaptureFrameState as Dispatch<ExtForeignToplevelHandleV1, ()>>::event(
        &mut state,
        &handle,
        ext_foreign_toplevel_handle_v1::Event::Title {
            title: "My App".to_string(),
        },
        &(),
        &conn,
        &qh,
    );
    <CaptureFrameState as Dispatch<ExtForeignToplevelHandleV1, ()>>::event(
        &mut state,
        &handle,
        ext_foreign_toplevel_handle_v1::Event::AppId {
            app_id: "com.example.app".to_string(),
        },
        &(),
        &conn,
        &qh,
    );
    <CaptureFrameState as Dispatch<ExtForeignToplevelHandleV1, ()>>::event(
        &mut state,
        &handle,
        ext_foreign_toplevel_handle_v1::Event::Identifier {
            identifier: "id-1".to_string(),
        },
        &(),
        &conn,
        &qh,
    );

    assert_eq!(state.toplevels[0].title, "My App");
    assert_eq!(state.toplevels[0].app_id, "com.example.app");
    assert_eq!(state.toplevels[0].identifier, "id-1");
    assert!(state.toplevels[0].active);

    <CaptureFrameState as Dispatch<ExtForeignToplevelHandleV1, ()>>::event(
        &mut state,
        &handle,
        ext_foreign_toplevel_handle_v1::Event::Closed,
        &(),
        &conn,
        &qh,
    );
    assert!(!state.toplevels[0].active);
    std::mem::forget(state);
}

#[test]
fn toplevel_handle_done_event_is_noop() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let handle: ExtForeignToplevelHandleV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    state.toplevels.push(TopLevel::new(handle.clone()));

    <CaptureFrameState as Dispatch<ExtForeignToplevelHandleV1, ()>>::event(
        &mut state,
        &handle,
        ext_foreign_toplevel_handle_v1::Event::Done,
        &(),
        &conn,
        &qh,
    );
    assert!(state.toplevels[0].active);
    assert!(state.toplevels[0].title.is_empty());
    std::mem::forget(state);
}

// --- CaptureFrameState + ZwlrScreencopyFrameV1 ---

#[test]
fn screencopy_frame_buffer_event_pushes_format() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ZwlrScreencopyFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ZwlrScreencopyFrameV1, ()>>::event(
        &mut state,
        &frame,
        zwlr_screencopy_frame_v1::Event::Buffer {
            format: WEnum::Value(wl_shm::Format::Argb8888),
            width: 100,
            height: 200,
            stride: 400,
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.formats.len(), 1);
    assert_eq!(state.formats[0].format, wl_shm::Format::Argb8888);
}

#[test]
fn screencopy_frame_buffer_unknown_format_is_ignored() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ZwlrScreencopyFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ZwlrScreencopyFrameV1, ()>>::event(
        &mut state,
        &frame,
        zwlr_screencopy_frame_v1::Event::Buffer {
            format: WEnum::Unknown(9999),
            width: 100,
            height: 200,
            stride: 400,
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.formats.is_empty());
}

#[test]
fn screencopy_frame_ready_sets_finished() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ZwlrScreencopyFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ZwlrScreencopyFrameV1, ()>>::event(
        &mut state,
        &frame,
        zwlr_screencopy_frame_v1::Event::Ready {
            tv_sec_hi: 0,
            tv_sec_lo: 0,
            tv_nsec: 0,
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.state, Some(FrameState::Finished));
}

#[test]
fn screencopy_frame_failed_sets_failed() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ZwlrScreencopyFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ZwlrScreencopyFrameV1, ()>>::event(
        &mut state,
        &frame,
        zwlr_screencopy_frame_v1::Event::Failed,
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.state, Some(FrameState::Failed));
}

#[test]
fn screencopy_frame_damage_is_noop() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ZwlrScreencopyFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ZwlrScreencopyFrameV1, ()>>::event(
        &mut state,
        &frame,
        zwlr_screencopy_frame_v1::Event::Damage {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        },
        &(),
        &conn,
        &qh,
    );
    assert!(state.state.is_none());
}

#[test]
fn screencopy_frame_linux_dmabuf_pushes_dma_format() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ZwlrScreencopyFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ZwlrScreencopyFrameV1, ()>>::event(
        &mut state,
        &frame,
        zwlr_screencopy_frame_v1::Event::LinuxDmabuf {
            format: 0x34325241,
            width: 1920,
            height: 1080,
        },
        &(),
        &conn,
        &qh,
    );
    assert_eq!(state.dmabuf_formats.len(), 1);
    assert_eq!(state.dmabuf_formats[0].format, 0x34325241);
}

#[test]
fn screencopy_frame_buffer_done_sets_flag() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let frame: ZwlrScreencopyFrameV1 = inert(&conn);
    let mut state = CaptureFrameState::new(false);
    <CaptureFrameState as Dispatch<ZwlrScreencopyFrameV1, ()>>::event(
        &mut state,
        &frame,
        zwlr_screencopy_frame_v1::Event::BufferDone,
        &(),
        &conn,
        &qh,
    );
    assert!(state.buffer_done.load(std::sync::atomic::Ordering::SeqCst));
}

// --- LayerShellState + ZwlrLayerSurfaceV1 ---

#[test]
fn layer_surface_configure_inserts_output_and_acks() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<LayerShellState>().handle();
    let surface: ZwlrLayerSurfaceV1 = inert(&conn);
    let wl_output: WlOutput = inert(&conn);
    let mut state = LayerShellState {
        configured_outputs: Default::default(),
    };
    <LayerShellState as Dispatch<ZwlrLayerSurfaceV1, WlOutput>>::event(
        &mut state,
        &surface,
        zwlr_layer_surface_v1::Event::Configure {
            serial: 1,
            width: 100,
            height: 100,
        },
        &wl_output,
        &conn,
        &qh,
    );
    assert!(state.configured_outputs.contains(&wl_output));
    std::mem::forget(state);
}

#[test]
fn layer_surface_closed_is_noop() {
    let conn = dummy_conn();
    let qh = conn.new_event_queue::<LayerShellState>().handle();
    let surface: ZwlrLayerSurfaceV1 = inert(&conn);
    let wl_output: WlOutput = inert(&conn);
    let mut state = LayerShellState {
        configured_outputs: Default::default(),
    };
    <LayerShellState as Dispatch<ZwlrLayerSurfaceV1, WlOutput>>::event(
        &mut state,
        &surface,
        zwlr_layer_surface_v1::Event::Closed,
        &wl_output,
        &conn,
        &qh,
    );
    assert!(state.configured_outputs.is_empty());
    std::mem::forget(wl_output);
}

// --- ZwpLinuxDmabufV1 / ZwpLinuxBufferParamsV1 no-ops (dmabuf feature) ---

#[cfg(feature = "dmabuf")]
#[test]
fn zwp_linux_dmabuf_and_buffer_params_events_are_noop() {
    use wayland_protocols::wp::linux_dmabuf::zv1::client::{
        zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
        zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1},
    };

    let conn = dummy_conn();
    let qh = conn.new_event_queue::<CaptureFrameState>().handle();
    let mut state = CaptureFrameState::new(false);

    let dmabuf: ZwpLinuxDmabufV1 = inert(&conn);
    <CaptureFrameState as Dispatch<ZwpLinuxDmabufV1, ()>>::event(
        &mut state,
        &dmabuf,
        zwp_linux_dmabuf_v1::Event::Format { format: 0 },
        &(),
        &conn,
        &qh,
    );

    let params: ZwpLinuxBufferParamsV1 = inert(&conn);
    <CaptureFrameState as Dispatch<ZwpLinuxBufferParamsV1, ()>>::event(
        &mut state,
        &params,
        zwp_linux_buffer_params_v1::Event::Created {
            buffer: inert(&conn),
        },
        &(),
        &conn,
        &qh,
    );
}

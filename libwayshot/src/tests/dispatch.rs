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

#[test]
fn capture_frame_state_initial_state_is_none() {
    let state = CaptureFrameState::new(false);
    assert!(state.state.is_none());
    assert!(state.toplevels.is_empty());
}

#[test]
fn frame_state_variants_and_equality() {
    use crate::dispatch::FrameState;
    use wayland_client::WEnum;
    use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason;

    assert_eq!(FrameState::Failed, FrameState::Failed);
    assert_eq!(FrameState::Finished, FrameState::Finished);
    assert_ne!(FrameState::Failed, FrameState::Finished);

    let reason = WEnum::<FailureReason>::Unknown(42);
    let s1 = FrameState::FailedWithReason(reason);
    let s2 = FrameState::FailedWithReason(WEnum::<FailureReason>::Unknown(42));
    assert_eq!(s1, s2);
    assert_ne!(s1, FrameState::Failed);
}

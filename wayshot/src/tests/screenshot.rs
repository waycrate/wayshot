//! Tests that connect to a real Wayland compositor to exercise `capture()`'s
//! non-interactive dispatch paths. The interactive branches (`ChooseOutput`,
//! `ChooseToplevel`, `Geometry` via libwaysip) need real pointer/keyboard
//! input or a TTY and aren't attempted here - see the module doc in
//! `libwayshot/src/tests/live_connection.rs` for the general rationale on
//! why these skip themselves without a compositor.

use libwayshot::WayshotConnection;
use libwayshot::region::LogicalRegion;

use crate::screenshot::{CaptureMode, ShotResult, capture};

fn skip_without_compositor() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("skipping: no WAYLAND_DISPLAY set (requires a live compositor)");
        return true;
    }
    false
}

#[test]
fn capture_all_outputs_succeeds() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let (image, result) = capture(&conn, &CaptureMode::All, false, true)
        .expect("capturing all outputs should succeed");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
    assert!(matches!(result, ShotResult::All));
}

#[test]
fn capture_output_by_real_name_succeeds() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let name = conn.get_all_outputs()[0].name.clone();
    let (image, result) = capture(&conn, &CaptureMode::Output(name.clone()), false, true)
        .expect("capturing a real output by name should succeed");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
    match result {
        ShotResult::Output { name: got } => assert_eq!(got, name),
        _ => panic!("expected ShotResult::Output"),
    }
}

#[test]
fn capture_output_by_unknown_name_errors_cleanly() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let result = capture(
        &conn,
        &CaptureMode::Output("definitely-not-a-real-output".to_string()),
        false,
        true,
    );
    assert!(result.is_err());
}

#[test]
fn capture_toplevel_errors_when_none_exist() {
    if skip_without_compositor() {
        return;
    }
    // A freshly started headless compositor has no windows open, so any
    // identifier should fail to match - this exercises the "not found"
    // error path in capture_toplevel_by_identifier without needing an
    // actual client window.
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let result = capture(
        &conn,
        &CaptureMode::Toplevel("no-such-toplevel".to_string()),
        false,
        true,
    );
    assert!(result.is_err());
}

#[test]
fn capture_geometry_region_succeeds_for_the_full_output() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let region = LogicalRegion::from(&conn.get_all_outputs()[0]);
    let (image, result) = capture(&conn, &CaptureMode::GeometryRegion(region), false, true)
        .expect("capturing an explicit region should succeed");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
    assert!(matches!(result, ShotResult::Area));
}

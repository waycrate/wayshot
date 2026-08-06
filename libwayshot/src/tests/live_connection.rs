//! Tests that connect to a *real* Wayland compositor over the actual
//! protocol, as opposed to the "inert object" tests elsewhere in this module
//! that fake a single dead proxy without a server behind it.
//!
//! `WayshotConnection::new()` does a live registry round-trip
//! (`registry_queue_init`), so there's no way to exercise it without an
//! actual compositor process answering on the other end of the socket. CI
//! starts one (wlroots' headless backend, see
//! `.github/workflows/test-coverage.yml`) and points `WAYLAND_DISPLAY` at
//! it before running tests. Locally, or in any other CI job, there's no
//! compositor, so these skip themselves at runtime instead of failing -
//! `cargo test` must stay green on a plain developer machine.
//!
//! Capture-specific tests (screenshot pipeline, frame formats, etc.) live in
//! `live_capture.rs`; this file covers connection lifecycle and output/
//! toplevel bookkeeping.

use crate::WayshotConnection;

pub(super) fn skip_without_compositor() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("skipping: no WAYLAND_DISPLAY set (requires a live compositor)");
        return true;
    }
    false
}

#[test]
fn connects_and_lists_at_least_one_output() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let outputs = conn.get_all_outputs();
    assert!(
        !outputs.is_empty(),
        "expected at least one output (WLR_HEADLESS_OUTPUTS should create one)"
    );
}

#[test]
fn output_info_fields_are_populated() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let output = &conn.get_all_outputs()[0];
    assert!(!output.name.is_empty());
    assert!(output.physical_size.width > 0);
    assert!(output.physical_size.height > 0);
    assert!(output.logical_region.inner.size.width > 0);
    assert!(output.logical_region.inner.size.height > 0);
}

#[test]
fn refresh_outputs_keeps_the_same_output_present() {
    if skip_without_compositor() {
        return;
    }
    let mut conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let name_before = conn.get_all_outputs()[0].name.clone();
    conn.refresh_outputs()
        .expect("refreshing outputs against a live compositor should succeed");
    let names_after: Vec<&str> = conn
        .get_all_outputs()
        .iter()
        .map(|o| o.name.as_str())
        .collect();
    assert!(names_after.contains(&name_before.as_str()));
}

#[test]
fn toplevel_list_does_not_error_even_when_empty() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    // A freshly started headless compositor has no toplevels open; this just
    // confirms the query round-trip itself doesn't error.
    let _ = conn.get_all_toplevels();
}

#[test]
fn refresh_toplevels_does_not_error() {
    if skip_without_compositor() {
        return;
    }
    let mut conn = WayshotConnection::new().expect("should connect to the CI compositor");
    conn.refresh_toplevels()
        .expect("refreshing toplevels against a live compositor should succeed");
}

#[test]
fn protocol_support_flags_do_not_panic() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    // Just confirm these are readable without panicking; the actual value
    // depends on which capture protocol the compositor advertises.
    let _ = conn.toplevel_capture_support();
    let _ = conn.image_copy_support();
}

#[test]
fn try_clone_produces_an_independent_working_connection() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let cloned = conn
        .try_clone()
        .expect("cloning a live connection should succeed");
    assert!(!cloned.get_all_outputs().is_empty());
}

#[test]
fn screenshot_all_produces_a_non_empty_image() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let image = conn
        .screenshot_all(false)
        .expect("should capture the virtual output via shm screencopy");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
}

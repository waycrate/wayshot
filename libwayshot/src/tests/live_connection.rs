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

use crate::WayshotConnection;

fn skip_without_compositor() -> bool {
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

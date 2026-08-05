//! Integration test that exercises the compiled `waymirror-egl` binary
//! directly. `WaylandEGLState::new()` calls `Connection::connect_to_env()`
//! before touching any EGL/GPU state, so this doesn't need a GPU or even a
//! compositor - it just checks the binary fails cleanly (via its `Result`
//! return, not a panic) instead of when there's nothing to connect to.
//! `WAYLAND_DISPLAY`/`WAYLAND_SOCKET` are stripped explicitly so this
//! behaves the same on a machine that does have a compositor running.

use std::process::Command;

#[test]
fn without_a_compositor_it_fails_cleanly_instead_of_panicking() {
    let output = Command::new(env!("CARGO_BIN_EXE_waymirror-egl"))
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .output()
        .expect("failed to run waymirror-egl binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
    assert!(!stderr.contains("panicked at"));
}

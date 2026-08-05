//! Integration tests that exercise the compiled `wayshot` binary directly.
//!
//! These cover `main()`'s early-exit paths that never touch Wayland
//! (`--completions`, `--help`, `--version`), the clap usage-error exit path,
//! and the "no compositor available" error path — verifying the binary fails
//! cleanly instead of panicking when `WayshotConnection::new()` can't
//! connect. None of this requires a running compositor, so it's safe in
//! headless CI. `WAYLAND_DISPLAY`/`WAYLAND_SOCKET` are stripped explicitly so
//! the tests behave the same on a machine that does have a compositor
//! running.

use std::process::Command;

fn wayshot_cmd() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_wayshot"));
    cmd.env_remove("WAYLAND_DISPLAY");
    cmd.env_remove("WAYLAND_SOCKET");
    cmd
}

#[test]
fn help_flag_exits_successfully_without_a_compositor() {
    let output = wayshot_cmd()
        .arg("--help")
        .output()
        .expect("failed to run wayshot binary");
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn version_flag_prints_version_and_exits_successfully() {
    let output = wayshot_cmd()
        .arg("--version")
        .output()
        .expect("failed to run wayshot binary");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("wayshot"));
}

#[cfg(feature = "completions")]
#[test]
fn completions_flag_prints_a_script_without_touching_wayland() {
    let output = wayshot_cmd()
        .args(["--completions", "bash"])
        .output()
        .expect("failed to run wayshot binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wayshot"));
}

#[test]
fn conflicting_flags_exit_with_a_usage_error_before_touching_wayland() {
    let output = wayshot_cmd()
        .args(["--choose-output", "--toplevel", "x"])
        .output()
        .expect("failed to run wayshot binary");
    assert!(!output.status.success());
    assert!(!output.stderr.is_empty());
}

#[test]
fn without_a_compositor_it_fails_cleanly_instead_of_panicking() {
    let output = wayshot_cmd()
        .arg("--list-outputs")
        .output()
        .expect("failed to run wayshot binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
    assert!(!stderr.contains("panicked at"));
}

// ─── Tests below need a real compositor (CI starts a headless wlroots one; see
// .github/workflows/test-coverage.yml). They read WAYLAND_DISPLAY as-is
// instead of stripping it, and skip themselves at runtime when it's unset so
// this file stays green on a plain developer machine or any other CI job. ───

fn compositor_available() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("skipping: no WAYLAND_DISPLAY set (requires a live compositor)");
        return false;
    }
    true
}

#[test]
fn list_outputs_prints_the_virtual_output_name() {
    if !compositor_available() {
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_wayshot"))
        .arg("--list-outputs")
        .output()
        .expect("failed to run wayshot binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
}

#[test]
fn list_outputs_json_produces_valid_json() {
    if !compositor_available() {
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_wayshot"))
        .arg("--list-outputs-json")
        .output()
        .expect("failed to run wayshot binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_array());
    assert!(!parsed.as_array().unwrap().is_empty());
}

#[cfg(feature = "notifications")]
#[test]
fn capturing_to_stdout_produces_a_valid_png() {
    if !compositor_available() {
        return;
    }
    // --encoding is explicit so this doesn't depend on whatever encoding a
    // user's own config.toml might default to. --silent skips the
    // notification path (D-Bus session bus availability in CI is a separate
    // concern from the capture/encode/stdout pipeline this test targets).
    let output = Command::new(env!("CARGO_BIN_EXE_wayshot"))
        .args(["--encoding", "png", "--silent", "-"])
        .output()
        .expect("failed to run wayshot binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // PNG magic bytes.
    assert!(
        output
            .stdout
            .starts_with(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'])
    );
}

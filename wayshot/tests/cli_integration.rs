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

#[cfg(feature = "notifications")]
#[test]
fn capturing_to_a_file_writes_a_valid_png_and_the_notification_fork_does_not_hang() {
    if !compositor_available() {
        return;
    }
    // Deliberately NOT --silent: this is the tool's primary real-world path
    // (save to a file, notify on success), which forks a background process
    // to serve the "open folder" notification action. The parent is
    // documented to return immediately regardless of whether the fork or the
    // D-Bus notification itself succeeds, so this also confirms that fork
    // doesn't block wayshot's own exit even without a notification daemon
    // present in CI.
    //
    // --config points at a path that doesn't exist so this doesn't depend on
    // (and isn't silently skipped by) a real user config.toml that disables
    // notifications - discovered by testing on a machine with exactly such a
    // config, which made this test pass without ever actually exercising the
    // notification/fork path it's meant to cover.
    let path = std::env::temp_dir().join(format!(
        "wayshot-integration-test-{}-notify-fork.png",
        std::process::id()
    ));
    let bogus_config = std::env::temp_dir().join(format!(
        "wayshot-integration-test-{}-nonexistent-config.toml",
        std::process::id()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_wayshot"))
        .args(["--encoding", "png", "--config"])
        .arg(&bogus_config)
        .arg(&path)
        .output()
        .expect("failed to run wayshot binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(&path).expect("output file should have been written");
    let _ = std::fs::remove_file(&path);
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']));
}

#[cfg(feature = "clipboard")]
#[test]
fn clipboard_flag_sets_the_real_wayland_clipboard() {
    if !compositor_available() {
        return;
    }
    if Command::new("wl-paste").arg("--version").output().is_err() {
        eprintln!("skipping: wl-paste (wl-clipboard) not installed");
        return;
    }

    // Write to a real file rather than stdout ("-"), AND avoid
    // `Command::output()` / any piped stdout+stderr entirely. The clipboard
    // fork deliberately persists in the background indefinitely (to keep
    // serving the clipboard until it's overwritten) and never closes the
    // stdio fds it inherited at fork time. If either pipe were captured here
    // (as `Command::output()` sets up for both stdout AND stderr by
    // default), that lingering background process would hold the pipe's
    // write end open forever and our read would hang waiting for an EOF that
    // never comes - this is a real, reproducible bug in the tool
    // (`wayshot --clipboard - | cat` hangs the same way from a plain shell,
    // and redirecting only stdout to a file while leaving stderr piped still
    // hangs), not just a test-harness quirk, and is worth fixing in
    // clipboard.rs separately.
    let path = std::env::temp_dir().join(format!(
        "wayshot-integration-test-{}-clipboard.png",
        std::process::id()
    ));
    let status = Command::new(env!("CARGO_BIN_EXE_wayshot"))
        .args(["--clipboard", "--silent", "--encoding", "png"])
        .arg(&path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to run wayshot binary");
    assert!(status.success());
    let bytes = std::fs::read(&path).expect("output file should have been written");
    let _ = std::fs::remove_file(&path);

    // The background process wayshot forked to serve the clipboard claims
    // the wl_data_source *after* the parent (and thus wayshot's own exit)
    // already returned - a genuine async handoff, not just test-harness
    // slack. Reading via wl-paste immediately can catch it mid-handshake (or
    // read a stale previous clipboard owner), so poll briefly instead of
    // asserting on the very first read.
    let mut pasted_stdout = Vec::new();
    let mut matched = false;
    for _ in 0..20 {
        let pasted = Command::new("wl-paste")
            .output()
            .expect("failed to run wl-paste");
        assert!(pasted.status.success());
        pasted_stdout = pasted.stdout;
        if pasted_stdout == bytes {
            matched = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    assert!(
        matched,
        "clipboard content should eventually match the captured image \
         (got {} bytes, expected {} bytes)",
        pasted_stdout.len(),
        bytes.len()
    );
}

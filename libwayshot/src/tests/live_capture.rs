//! Live-compositor tests for the capture pipeline (frame formats, screenshot
//! entry points, region handling). See `live_connection.rs` for why these
//! need a real compositor and how they skip themselves without one.

use crate::WayshotConnection;
use crate::WayshotTarget;
use crate::region::LogicalRegion;

use super::live_connection::skip_without_compositor;

#[test]
fn get_available_frame_formats_returns_at_least_one_format() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let output = conn.get_all_outputs()[0].clone();
    let target = WayshotTarget::Screen(output.wl_output);
    let formats = conn
        .get_available_frame_formats(&target)
        .expect("compositor should report at least one supported shm format");
    assert!(!formats.is_empty());
}

#[test]
fn capture_frame_copies_returns_one_entry_per_output() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let outputs = conn.get_all_outputs().to_vec();
    let requests: Vec<_> = outputs.iter().map(|o| (o.clone(), None)).collect();
    let copies = conn
        .capture_frame_copies(&requests, false)
        .expect("capturing every output should succeed");
    assert_eq!(copies.len(), outputs.len());
}

#[test]
fn screenshot_with_explicit_region_matches_output_bounds() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let output = &conn.get_all_outputs()[0];
    let region = LogicalRegion::from(output);
    let image = conn
        .screenshot(region, false)
        .expect("capturing the output's own logical region should succeed");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
}

#[test]
fn screenshot_single_output_produces_an_image() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let output = conn.get_all_outputs()[0].clone();
    let image = conn
        .screenshot_single_output(&output, false)
        .expect("capturing a single output should succeed");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
}

#[test]
fn screenshot_outputs_errors_on_empty_slice() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let result = conn.screenshot_outputs(&[], false);
    assert!(matches!(result, Err(crate::Error::NoOutputs)));
}

#[test]
fn screenshot_freeze_runs_the_callback_and_returns_its_region() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let expected_region = LogicalRegion::from(&conn.get_all_outputs()[0]);
    // A programmatic callback (no real interactive selection) that just hands
    // back a fixed region - this still exercises the layer-shell overlay
    // creation/teardown path, just not any actual input handling.
    let image = conn
        .screenshot_freeze(move |_conn| Ok(expected_region), false)
        .expect("freeze capture with a programmatic callback should succeed");
    assert!(image.width() > 0);
    assert!(image.height() > 0);
}

#[cfg(feature = "dmabuf")]
#[test]
fn dmabuf_capture_errors_cleanly_without_a_dmabuf_capable_connection() {
    if skip_without_compositor() {
        return;
    }
    // `WayshotConnection::new()` never sets `dmabuf_state` - only
    // `from_connection_with_dmabuf()` does (see `dmabuf_state: None` in both
    // `from_connection` and `try_clone`, vs. `Some(..)` only in
    // `from_connection_with_dmabuf`). `capture_target_frame_dmabuf` checks
    // that field before touching any GPU/DRM device, so this is
    // deterministic regardless of the host's actual dmabuf/GPU capability -
    // it holds the same way on a machine with full GPU support, because it's
    // driven entirely by which constructor was called, not by what the
    // environment can do.
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let output = conn.get_all_outputs()[0].clone();
    let target = WayshotTarget::Screen(output.wl_output);
    let result = conn.capture_target_frame_dmabuf(&target, false, None);
    assert!(matches!(result, Err(crate::Error::NoDMAStateError)));
}

//! Live-compositor tests for the shm-backed screencast path
//! (`create_screencast_with_shm` + `WayshotScreenCast::screencast()`).
//!
//! Despite the doc comment on `WayshotScreenCast::screencast()` mentioning
//! PipeWire, libwayshot itself never touches PipeWire - that comment
//! describes what a *caller* (e.g. the `waymirror` example) does with the
//! resulting buffer. Everything in this file is pure Wayland protocol
//! (image-copy-capture / wlr-screencopy backed shm buffers), so unlike the
//! DMA-BUF/EGL screencast variants, it needs no GPU and works fine against
//! the CI headless compositor. See `live_connection.rs` for why these tests
//! skip themselves without a live compositor.

use std::ffi::CString;
use std::fs::File;

use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, memfd_create};

use crate::WayshotConnection;
use crate::WayshotTarget;

use super::live_connection::skip_without_compositor;

/// A `wl_shm_pool` fd has to be an anonymous, sealed memory file - wlroots
/// rejects a plain disk-backed file with "Couldn't mmap from fd" (a
/// protocol-level error, not a Rust type error, since a regular `File` also
/// implements `AsFd` and compiles fine). This mirrors what the crate's own
/// private `create_shm_fd()` helper does internally.
fn shm_backed_file(size: u64) -> File {
    let fd = memfd_create(
        CString::new("libwayshot-test").unwrap().as_c_str(),
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
    )
    .expect("memfd_create should succeed");
    let _ = fcntl_add_seals(&fd, SealFlags::SHRINK | SealFlags::SEAL);
    let file = File::from(fd);
    file.set_len(size).expect("set memfd length");
    file
}

#[test]
fn create_screencast_with_shm_captures_a_frame() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let output = conn.get_all_outputs()[0].clone();
    let target = WayshotTarget::Screen(output.wl_output.clone());

    // Query formats ourselves first so we know how big to size the shm file;
    // create_screencast_with_shm needs the fd already sized correctly.
    let formats = conn
        .get_available_frame_formats(&target)
        .expect("compositor should report at least one supported shm format");
    let format = formats.first().expect("at least one format");

    let file = shm_backed_file(format.byte_size());

    let mut screencast = conn
        .create_screencast_with_shm(target, false, format.format, &file)
        .expect("should create a shm-backed screencast session");
    assert!(
        screencast.dmabuf_bo().is_none(),
        "shm sessions shouldn't have a dmabuf-backed buffer object"
    );

    screencast
        .screencast()
        .expect("capturing a frame into the shm buffer should succeed");

    assert!(screencast.current_size().width > 0);
    assert!(screencast.current_size().height > 0);
}

#[test]
fn screencast_can_capture_more_than_one_frame_on_the_same_session() {
    if skip_without_compositor() {
        return;
    }
    let conn = WayshotConnection::new().expect("should connect to the CI compositor");
    let output = conn.get_all_outputs()[0].clone();
    let target = WayshotTarget::Screen(output.wl_output.clone());

    let formats = conn
        .get_available_frame_formats(&target)
        .expect("compositor should report at least one supported shm format");
    let format = formats.first().expect("at least one format");

    let file = shm_backed_file(format.byte_size());

    let mut screencast = conn
        .create_screencast_with_shm(target, false, format.format, &file)
        .expect("should create a shm-backed screencast session");

    screencast
        .screencast()
        .expect("first capture should succeed");
    screencast
        .screencast()
        .expect("second capture on the same session should also succeed");
}

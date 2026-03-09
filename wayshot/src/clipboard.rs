//! Wayland clipboard integration.

use eyre::Result;
use rustix::runtime::{self, Fork};
use wl_clipboard_rs::copy::{MimeType, Options, Source};

/// Copy `data` to the Wayland clipboard.
///
/// The process is forked so that the image stays available until something
/// else is copied. The parent returns immediately; the child blocks inside
/// `opts.copy()` until the clipboard is overwritten, then exits normally.
/// If the fork fails the current process serves the clipboard without
/// persistence (i.e. the image is only available until wayshot exits).
pub fn copy_to_clipboard(data: Vec<u8>) -> Result<()> {
    let mut opts = Options::new();
    match unsafe { runtime::kernel_fork() } {
        Ok(Fork::ParentOf(_)) => {
            // Parent exits this function so the rest of main can continue.
        }
        Ok(Fork::Child(_)) => {
            opts.foreground(true);
            opts.copy(Source::Bytes(data.into()), MimeType::Autodetect)?;
        }
        Err(e) => {
            tracing::warn!(
                "Fork failed ({e}): image will only be available on the clipboard \
                 until wayshot exits. Use a clipboard manager to persist it."
            );
            opts.copy(Source::Bytes(data.into()), MimeType::Autodetect)?;
        }
    }
    Ok(())
}

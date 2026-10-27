//! Wayland clipboard integration.

use eyre::Result;
use std::os::fd::AsRawFd;
use wl_clipboard_rs::copy::{MimeType, Options, Source};

use crate::utils::EncodingFormat;

/// Determine the correct MIME type for the given encoding format.
///
/// wl-clipboard-rs's MimeType::Autodetect only works reliably for JPEG and PNG.
/// For other formats (WebP, AVIF, JXL, etc.), we must explicitly specify the MIME type.
pub(crate) fn mime_type_for_encoding(encoding: EncodingFormat) -> MimeType {
    let mime_str = match encoding {
        #[cfg(feature = "jpeg")]
        EncodingFormat::Jpg => "image/jpeg",
        EncodingFormat::Png => "image/png",
        #[cfg(feature = "pnm")]
        EncodingFormat::Ppm => "image/x-portable-pixmap",
        #[cfg(feature = "qoi")]
        EncodingFormat::Qoi => "image/qoi",
        #[cfg(feature = "webp")]
        EncodingFormat::Webp => "image/webp",
        #[cfg(feature = "avif")]
        EncodingFormat::Avif => "image/avif",
        #[cfg(feature = "jxl")]
        EncodingFormat::Jxl => "image/jxl",
    };
    MimeType::Specific(mime_str.to_string())
}

/// Copy `data` to the Wayland clipboard.
///
/// The process is forked so that the image stays available until something
/// else is copied. The parent returns immediately; the child blocks inside
/// `opts.copy()` until the clipboard is overwritten, then exits normally.
/// If the fork fails the current process serves the clipboard without
/// persistence (i.e. the image is only available until wayshot exits).
///
/// The `encoding` parameter is used to explicitly set the MIME type, avoiding
/// reliance on wl-clipboard-rs's autodetection which only works for JPEG/PNG.
pub fn copy_to_clipboard(data: Vec<u8>, encoding: EncodingFormat) -> Result<()> {
    let mime = mime_type_for_encoding(encoding);
    let mut opts = Options::new();
    match unsafe { libc::fork() } {
        -1 => {
            let e = std::io::Error::last_os_error();
            tracing::warn!(
                "Fork failed ({e}): image will only be available on the clipboard \
                 until wayshot exits. Use a clipboard manager to persist it."
            );
            opts.copy(Source::Bytes(data.into()), mime)?;
        }
        0 => {
            if let Ok(devnull) = std::fs::File::open("/dev/null") {
                let fd = devnull.as_raw_fd();
                unsafe {
                    libc::dup2(fd, libc::STDIN_FILENO);
                    libc::dup2(fd, libc::STDOUT_FILENO);
                    libc::dup2(fd, libc::STDERR_FILENO);
                }
            }
            opts.foreground(true);
            let result = opts.copy(Source::Bytes(data.into()), mime);
            std::process::exit(if result.is_ok() { 0 } else { 1 });
        }
        _ => {
            // Parent exits this function so the rest of main can continue.
        }
    }
    Ok(())
}

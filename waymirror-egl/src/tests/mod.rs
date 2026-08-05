//! Private test module. Most of this crate needs a real GPU/EGL context and
//! a Wayland connection with `xdg_shell`/window-surface support, neither of
//! which is available in headless CI (the compositor CI starts for
//! `libwayshot`/`wayshot` is `WLR_RENDERER=pixman` specifically to avoid
//! needing a GPU at all). What's covered here is what's left: pure error
//! formatting/conversion, and the one piece of branching logic that could be
//! pulled out of a `Dispatch` handler without needing a live context.

mod dispatch;
mod error;

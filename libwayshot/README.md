<p align=center>
  <img src="https://git.sr.ht/~shinyzenith/wayshot/blob/main/docs/assets/wayshot.png" alt=wayshot width=60%>
  <p align=center>A native, blazing-fast 🚀🚀🚀 screenshot crate for wlroots based compositors such as sway and river.</p>

  <p align="center">
  <a href="./LICENSE.md"><img src="https://img.shields.io/github/license/waycrate/wayshot?style=flat-square&logo=appveyor"></a>
  <img src="https://img.shields.io/badge/cargo-v0.1.0-green?style=flat-square&logo=appveyor">
  </p>
</p>

# `libwayshot`

`libwayshot` is a convenient wrapper over the wlroots screenshot protocol that provides a simple API to take screenshots with.

# Example usage

```rust
use libwayshot::WayshotConnection;

let wayshot_connection = WayshotConnection::new()?;
let image_buffer = wayshot_connection.screenshot_all()?;
```

# Optional features

All features are enabled by default. To opt out selectively:

```toml
libwayshot = { version = "...", default-features = false, features = ["png", "egl"] }
```

| Feature | What it enables | Extra dependencies |
|---------|----------------|--------------------|
| `png`   | PNG encoding/decoding | — (via image) |
| `jpeg`  | JPEG encoding/decoding | — (via image) |
| `qoi`   | QOI encoding/decoding | — (via image) |
| `webp`  | WebP encoding/decoding | — (via image) |
| `avif`  | AVIF encoding/decoding | rav1e (via image) |
| `egl`   | EGL/OpenGL GPU capture backend (DMA-BUF → EGLImage → GL texture) | gl, r-egl-wayland |

## EGL capture backend

The `egl` feature enables zero-copy GPU screen capture via the EGL/OpenGL path.
When enabled, the following APIs are available:

- `WayshotConnection::capture_target_frame_eglimage` — capture to an `EGLImage`
- `WayshotConnection::capture_target_frame_eglimage_on_display` — capture to an `EGLImage` on a given `EGLDisplay`
- `WayshotConnection::bind_target_frame_to_gl_texture` — capture and bind directly to the current GL texture
- `WayshotConnection::create_screencast_with_egl` — set up a screencast session with EGL display binding
- `EGLImageGuard` — RAII wrapper that owns and destroys the `EGLImage` on drop

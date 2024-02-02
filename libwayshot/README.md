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

<p align=center>
  <img src="https://git.sr.ht/~shinyzenith/wayshot/blob/main/docs/assets/wayshot.png" alt=wayshot width=60%>
  <p align=center>A native, blazing-fast 🚀🚀🚀 screenshot tool for wlroots based compositors such as sway and river written in Rust.</p>

  <p align="center">
  <a href="./LICENSE.md"><img src="https://img.shields.io/github/license/waycrate/wayshot?style=flat-square&logo=appveyor"></a>
  <img src="https://img.shields.io/badge/cargo-v1.4.5-green?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/issues/waycrate/wayshot?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/forks/waycrate/wayshot?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/stars/waycrate/wayshot?style=flat-square&logo=appveyor">
  <br>
  <img src="https://repology.org/badge/vertical-allrepos/wayshot.svg">
  </p>
</p>

# Portal integration

[xdg-desktop-portal-luminous](https://github.com/waycrate/xdg-desktop-portal-luminous) is a xdg-desktop-portal backend for wlroots based compositors, providing screenshot and screencast capabilities.


# Some usage examples:

NOTE: Read `man 7 wayshot` for more examples.

NOTE: Read `man 5 wayshot` for [config file](./config.toml) information.

NOTE: Read `man wayshot` for flag information.

Screenshot and crop region:

```bash
wayshot -g
```

Fullscreen:

```bash
wayshot
```

Screenshot and copy to clipboard:

```bash
wayshot --clipboard
```

Pick color with information:

```bash
wayshot --color
```

Capture a specific output:

```bash
wayshot --list-outputs   # see available output names
wayshot -o eDP-1
```

Interactively choose a window to capture:

```bash
wayshot --choose-toplevel
```

Pick a hex color code, using ImageMagick:

```bash
wayshot -g - | convert - -format '%[pixel:p{0,0}]' txt:-|grep -E "#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})" -o
```

# Optional features

All features are enabled in the default build. To reduce binary size or compile-time dependencies,
features can be selectively disabled:

```bash
cargo build --no-default-features --features clipboard,logger,notifications
```

| Feature        | What it adds                                          | Extra dependency      |
|----------------|-------------------------------------------------------|-----------------------|
| `avif`         | AVIF encoding (`--encoding avif`)                     | rav1e (via image)     |
| `clipboard`    | `--clipboard` flag, copy to Wayland clipboard         | wl-clipboard-rs       |
| `color_picker` | `--color` flag, freeze screen and pick a pixel color  | —                     |
| `jxl`          | JPEG-XL encoding (`--encoding jxl`)                   | libjxl / jpegxl-rs    |
| `logger`       | `--log-level` flag, tracing output to stderr          | tracing-subscriber    |
| `notifications`| Desktop notifications after each capture              | notify-rust           |
| `selector`     | `--geometry` flag, interactive region selection       | libwaysip             |

# Installation

## AUR:

`wayshot-git` & `wayshot-bin` have been packaged.

## Compile time dependencies:

-   scdoc (If present, man-pages will be generated.)
-   rustup
-   make
-   pkg-config
-   libjxl _(optional — only needed when the `jxl` feature is enabled)_

## Compiling:

-   `git clone https://github.com/waycrate/wayshot && cd wayshot`
-   `make setup`
-   `make`
-   `sudo make install`

# Support:

1. https://matrix.to/#/#waycrate-tools:matrix.org
2. https://discord.gg/KKZRDYrRYW

# Smithay Developers:

Massive thanks to smithay developer <a href="https://github.com/cmeissl">Cmeissl</a> and <a href="https://github.com/vberger">Victor Berger</a>. Without them this project won't be possible as my wayland knowledge is limited.

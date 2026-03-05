<p align=center>
  <img src="https://git.sr.ht/~shinyzenith/wayshot/blob/main/docs/assets/wayshot.png" alt=wayshot width=60%>
  <p align=center>A native, blazing-fast 🚀🚀🚀 screenshot tool for wlroots based compositors such as sway and river written in Rust.</p>

  <p align="center">
  <a href="./LICENSE.md"><img src="https://img.shields.io/github/license/waycrate/wayshot?style=flat-square&logo=appveyor"></a>
  <img src="https://img.shields.io/badge/cargo-v1.3.0-green?style=flat-square&logo=appveyor">
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

Screenshot and Crop Region:

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

Pick color with information

```bash
wayshot --color
```

Pick a hex color code:

```bash
wayshot --color hex
```

Pick a color in HSL format (useful for CSS):

```bash
wayshot --color hsl
```

Pick a color and copy the hex code directly to clipboard:

```bash
wayshot --color hex | wl-copy
```

# Installation

## AUR:

`wayshot-git` & `wayshot-bin` have been packaged.

## Compile time dependencies:

-   scdoc (If present, man-pages will be generated.)
-   rustup
-   make
-   pkg-config
-   libjxl

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

<p align=center>
  <img src="./docs/assets/wayshot.png" alt=wayshot width=60%>
  <p align=center>A native screenshot tool for Wayland (X11 support coming soon) written in Rust.</p>
  
  <p align="center">
  <a href="./LICENSE.md"><img src="https://img.shields.io/github/license/waycrate/wayshot?style=flat-square&logo=appveyor"></a>
  <img src="https://img.shields.io/badge/cargo-v1.0.0-green?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/issues/waycrate/wayshot?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/forks/waycrate/wayshot?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/stars/waycrate/wayshot?style=flat-square&logo=appveyor">
  </p>
</p>

# Usage:

**Note: The project is a WIP.**

Region Selection:

```bash
wayshot -s "$(slurp -f '%x %y %w %h')" > /tmp/image.png
```

Fullscreen:

```bash
wayshot > /tmp/image.png
```

# Installation
## AUR:
`wayshot-git` `wayshot-musl-git` have been packaged. `wayshot-bin` & `wayshot-musl-bin` will be released soon.

## Compile time dependencies:
-   rustup
-   make

## Compiling:
-   `git clone https://github.com/waycrate/wayshot && cd wayshot`
-   `make setup`
-   `make clean`
    -   `make`
    -   `make glibc`
-   `sudo make install`

# Support server:

https://discord.gg/KKZRDYrRYW

# Contributors:

<a href="https://github.com/waycrate/wayshot/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=waycrate/wayshot" />
</a>

# Cmeissl: 
Massive thanks to smithay contributor <a href="https://github.com/cmeissl">Cmeissl</a>. Without them this project won't be possible as my wayland knowledge is very very limited.

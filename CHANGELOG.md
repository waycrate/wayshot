# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.6] - 2026-03-09

### libwayshot 0.7.3

#### Added

- **`size`** and **`position`** fields on **`OutputInfo`** ([`b5c5896`](https://github.com/waycrate/wayshot/commit/b5c58965a42ba3ac2ac918874b079d27553f248b), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Changed

- screencast / capture API cleanup ([`9722235`](https://github.com/waycrate/wayshot/commit/972223507577ef7f40d49bf035e9238f5eca7cd4), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Fixed

- **ext-image-copy** frame state can advertise **multiple** SHM formats; handle the full set instead of assuming one ([#307](https://github.com/waycrate/wayshot/pull/307), [@Decodetalkers](https://github.com/Decodetalkers)).
- replace `unwrap()` calls with proper **`Result`** handling ([#294](https://github.com/waycrate/wayshot/pull/294), [@0xsamalt](https://github.com/0xsamalt)).
- avoid cloning the image buffer where a shared view is enough ([#290](https://github.com/waycrate/wayshot/pull/290), [@Nuhiat-Arefin](https://github.com/Nuhiat-Arefin)).

### wayshot

#### Added

- **`--silent`** (quiet non-error output) ([#280](https://github.com/waycrate/wayshot/pull/280), [@Suryansh-Dey](https://github.com/Suryansh-Dey)).

[1.4.6]: https://github.com/waycrate/wayshot/compare/1.4.5...v1.4.6

## [1.4.5] - 2026-01-20

### libwayshot 0.7.2

#### Fixed

- `reexport` now exposes **`ExtForeignToplevelHandleV1`** (the handle type), not **`ExtForeignToplevelListV1`** ([`2259223`](https://github.com/waycrate/wayshot/commit/2259223eb224bf6c98f04e05e10b53e4d5cd2f2f), [@Decodetalkers](https://github.com/Decodetalkers)).

[1.4.5]: https://github.com/waycrate/wayshot/compare/1.4.4...v1.4.5

## [1.4.4] - 2026-01-20

### libwayshot 0.7.1

#### Fixed

- correct **re-export** of `ExtForeignToplevelHandle` at the crate root ([`2709211`](https://github.com/waycrate/wayshot/commit/27092119725aea89c9124f79211dc39cefad49cb), [@Decodetalkers](https://github.com/Decodetalkers)).

[1.4.4]: https://github.com/waycrate/wayshot/compare/1.4.3...v1.4.4

## [1.4.3] - 2026-01-20

### libwayshot 0.7.0

#### Added

- **Color capture** (sampling) API ([#258](https://github.com/waycrate/wayshot/pull/258), [@Decodetalkers](https://github.com/Decodetalkers)).
- **Screencast EGL** path, **r-egl-wayland** integration, static EGL (no `egl::Instance`), DMabuf `AsRef` usage, and **waymirror-egl** rework ([#270](https://github.com/waycrate/wayshot/pull/270), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Changed

- **Error** types and handling ([#252](https://github.com/waycrate/wayshot/pull/252), [@dhruvDev23](https://github.com/dhruvDev23)).
- **libdrm**-oriented buffer logic where applicable ([#270](https://github.com/waycrate/wayshot/pull/270), [@Decodetalkers](https://github.com/Decodetalkers)).
- Rotation: do not swap width/height when the image buffer is already correctly oriented ([#256](https://github.com/waycrate/wayshot/pull/256), [@AndreasBackx](https://github.com/AndreasBackx)).

#### Fixed

- Avoid panic when **ext-foreign-toplevel-list** is unavailable; degrade gracefully ([#266](https://github.com/waycrate/wayshot/pull/266), [@Decodetalkers](https://github.com/Decodetalkers)).
- EGL capture: roundtrip errors, **FD leaks**, and incorrect color in some paths ([#270](https://github.com/waycrate/wayshot/pull/270), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Breaking Changes

- Many capture / screenshot entry points now take arguments in a fixed order: **target → options → cursor → optional region** ([#270](https://github.com/waycrate/wayshot/pull/270), [@Decodetalkers](https://github.com/Decodetalkers)).
  - **Migration:** update call sites to the new parameter order; consult current `libwayshot` method signatures.
- **`WayshotTarget`** / foreign toplevel handles must be chosen from targets libwayshot already knows about (not an arbitrary foreign id alone) ([#270](https://github.com/waycrate/wayshot/pull/270), [@Decodetalkers](https://github.com/Decodetalkers)).
  - **Migration:** enumerate toplevels via libwayshot, then select the matching target.
- **`try_init_buf`** no longer returns the initialized buffer from the same return channel; use `()` / `Error` only ([#270](https://github.com/waycrate/wayshot/pull/270), [@Decodetalkers](https://github.com/Decodetalkers)).
  - **Migration:** follow the updated init + capture flow in the docs / examples.

### wayshot

#### Added

- **JPEG XL**-specific options in the config file ([#255](https://github.com/waycrate/wayshot/pull/255), [@Gigas002](https://github.com/Gigas002)).
- **PNG**-specific encoder options in the config file ([#268](https://github.com/waycrate/wayshot/pull/268), [@Gigas002](https://github.com/Gigas002)).

[1.4.3]: https://github.com/waycrate/wayshot/compare/1.4.2...v1.4.3

## [1.4.2] - 2025-12-18

### libwayshot 0.6.0

#### Added

- **ext-image-copy** path enabled for capture ([#245](https://github.com/waycrate/wayshot/pull/245), [@xonx4l](https://github.com/xonx4l)).
- DMabuf screen logic for the screencast path (refresh / GBM discovery, libwaysip bump, docs) ([#248](https://github.com/waycrate/wayshot/pull/248), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Changed

- return **GBM** errors instead of panicking ([#247](https://github.com/waycrate/wayshot/pull/247), [@dhruvDev23](https://github.com/dhruvDev23); first contribution).

#### Fixed

- region / screenshot capture mis-sized under **ext-image-copy** (also fixes cross-output area selection in the CLI) ([`e9ba46b`](https://github.com/waycrate/wayshot/commit/e9ba46be0fca25bf784e34bb9c2321b75ae6b55d), [@Decodetalkers](https://github.com/Decodetalkers)).

[1.4.2]: https://github.com/waycrate/wayshot/compare/1.4.1...v1.4.2

## [1.4.1] - 2025-12-15

### libwayshot 0.5.0

#### Added

- public `screencast` module and helpers, split from the screenshot-oriented surface ([`279896a`](https://github.com/waycrate/wayshot/commit/279896a060adaa385a8c6b59a38864468c2a18d8), [@Decodetalkers](https://github.com/Decodetalkers)).
- size API on output / screencast capture ([`c4dc48e`](https://github.com/waycrate/wayshot/commit/c4dc48e2de9592de1dee3fe8f7eee36673dd73c7), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Changed

- screencast capture path refined for OBS-style integration ([`d481448`](https://github.com/waycrate/wayshot/commit/d481448cece7f4ebffb1858a52d8d901bd69cf69), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Fixed

- output capture sometimes missed the negotiated frame format ([`c4dc48e`](https://github.com/waycrate/wayshot/commit/c4dc48e2de9592de1dee3fe8f7eee36673dd73c7), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Breaking Changes

- `Size` is now generic (`Size<T = u32>`) ([`279896a`](https://github.com/waycrate/wayshot/commit/279896a060adaa385a8c6b59a38864468c2a18d8), [@Decodetalkers](https://github.com/Decodetalkers)).
  - **Migration:** specify `Size<u32>` (or another `T`) where type inference no longer resolves, and update patterns that assumed a non-generic `Size`.

[1.4.1]: https://github.com/waycrate/wayshot/compare/1.4.0...v1.4.1

## [1.4.0] - 2025-12-13

### libwayshot 0.4.0

#### Added

- BGR888 buffer format support ([#82](https://github.com/waycrate/wayshot/pull/82), [@vivienm](https://github.com/vivienm)).
- `FrameGuard`, `ScreenCapturer`, and `LogicalRegion` / `EmbeddedRegion` ([#90](https://github.com/waycrate/wayshot/pull/90), [@Shinyzenith](https://github.com/Shinyzenith), [@AndreasBackx](https://github.com/AndreasBackx)).
- Freeze capture pipeline ([#90](https://github.com/waycrate/wayshot/pull/90), [@Shinyzenith](https://github.com/Shinyzenith)).
- Scaling / HiDPI handling in the capture path ([#90](https://github.com/waycrate/wayshot/pull/90), [@Shinyzenith](https://github.com/Shinyzenith), [@AndreasBackx](https://github.com/AndreasBackx)).
- zwlr-screencopy DMabuf capture and EGL helpers (`waymirror-egl` demo) ([#122](https://github.com/waycrate/wayshot/pull/122), [#90](https://github.com/waycrate/wayshot/pull/90), [@CheerfulPianissimo](https://github.com/CheerfulPianissimo)).
- public `capture_output_frame_get_state_shm` for negotiated SHM format ([#151](https://github.com/waycrate/wayshot/pull/151), [@Decodetalkers](https://github.com/Decodetalkers)).
- `capture_output_frame_shm_fd_with_format` and `get_available_frame_formats` for portal-style integration ([#207](https://github.com/waycrate/wayshot/pull/207), [@nicolo-mn](https://github.com/nicolo-mn)).
- ext-foreign-toplevel and related protocols; richer `OutputInfo` (scale, xdg output metadata) ([#206](https://github.com/waycrate/wayshot/pull/206), [@Rishik-Y](https://github.com/Rishik-Y)).
- JPEG-XL via `jpegxl-rs` ([#193](https://github.com/waycrate/wayshot/pull/193), [@Gigas002](https://github.com/Gigas002)).
- unit tests for `error`, `output`, `region` ([#228](https://github.com/waycrate/wayshot/pull/228), [@xonx4l](https://github.com/xonx4l)).

#### Changed

- fewer allocations, slice-based APIs ([#99](https://github.com/waycrate/wayshot/pull/99), [@murlakatamenka](https://github.com/murlakatamenka)).
- clipboard / process helpers use `rustix` instead of `nix` ([#120](https://github.com/waycrate/wayshot/pull/120), [#90](https://github.com/waycrate/wayshot/pull/90), [@Shinyzenith](https://github.com/Shinyzenith), [@AndreasBackx](https://github.com/AndreasBackx)).
- callback APIs use generics instead of caller-supplied `Box` ([#150](https://github.com/waycrate/wayshot/pull/150), [@Decodetalkers](https://github.com/Decodetalkers)).
- optional `image` codec features line up with crate feature flags ([#179](https://github.com/waycrate/wayshot/pull/179), [@Decodetalkers](https://github.com/Decodetalkers)).
- DMabuf is only wired for zwlr-screencopy; ext-image-copy does not use DMabuf in this release ([`cd1da42`](https://github.com/waycrate/wayshot/commit/cd1da425aa9ce8861a0edf3f1727719f998e386a), [`630e8bad`](https://github.com/waycrate/wayshot/commit/630e8bad47d10d97154737da91c3d102fa9148cc), [@Decodetalkers](https://github.com/Decodetalkers)).

#### Fixed

- ext-image-copy buffer placement with screencopy ([`c48fac4`](https://github.com/waycrate/wayshot/commit/c48fac4572d60bd8665c84babd6fcf7f5443306b), [@Decodetalkers](https://github.com/Decodetalkers)).
- layer-shell overlay on **niri** (roundtrip / anchor) ([`28331dcc`](https://github.com/waycrate/wayshot/commit/28331dcc3886f8b9e6bc09f9951fe59744c16acf), [@uncomfyhalomacro](https://github.com/uncomfyhalomacro)).

#### Breaking Changes

- legacy `screenshot` APIs deprecated or restricted where **ext-image-copy** differs from **zwlr-screencopy**; use **`screenshot_freeze`** instead ([`c0065d3`](https://github.com/waycrate/wayshot/commit/c0065d37547bc18729d052a6e89628c34c3d3097), [`21cfa0f`](https://github.com/waycrate/wayshot/commit/21cfa0f36c7c3b7ce6ba4998f0e9ae6808d93ec0), [@Decodetalkers](https://github.com/Decodetalkers)). Some paths (e.g. color picker) keep the old API on purpose ([`73df58e`](https://github.com/waycrate/wayshot/commit/73df58ef030c97e89d2b77f0a3788ef26ef00b83)).
  - **Migration:** move affected call sites to **`screenshot_freeze`**; re-read deprecation on the symbols you use.
- `FrameGuard` carries side / transform metadata ([`1af8f788`](https://github.com/waycrate/wayshot/commit/1af8f78840c3f20b74e0225a747505f8a4eaf73f), [@Decodetalkers](https://github.com/Decodetalkers)).
  - **Migration:** update construction and pattern matches to include the new fields.

### wayshot

#### Added

- `clap` derive CLI: optional `[OUTPUT]`, `--log-level`, `--encoding` and kebab-case options with aliases ([#90](https://github.com/waycrate/wayshot/pull/90), [@AndreasBackx](https://github.com/AndreasBackx)).
- freeze mode before region / output selection ([#90](https://github.com/waycrate/wayshot/pull/90), [@Shinyzenith](https://github.com/Shinyzenith)).
- `--clipboard` with optional background daemon ([#91](https://github.com/waycrate/wayshot/pull/91), [@Shinyzenith](https://github.com/Shinyzenith); also [#90](https://github.com/waycrate/wayshot/pull/90)).
- WebP, AVIF, JPEG-XL ([#98](https://github.com/waycrate/wayshot/pull/98), [#141](https://github.com/waycrate/wayshot/pull/141), [#193](https://github.com/waycrate/wayshot/pull/193); [@Gigas002](https://github.com/Gigas002) in [#141](https://github.com/waycrate/wayshot/pull/141) and [#193](https://github.com/waycrate/wayshot/pull/193)).
- default filename patterns and `--timestamp` ([#93](https://github.com/waycrate/wayshot/pull/93), [@Shinyzenith](https://github.com/Shinyzenith)).
- directory as output path resolves filename inside it ([#96](https://github.com/waycrate/wayshot/pull/96), [@Shinyzenith](https://github.com/Shinyzenith)).
- TOML config and `--config` ([#154](https://github.com/waycrate/wayshot/pull/154), [@Gigas002](https://github.com/Gigas002)).
- `--file-name-format` with `~` and env expansion ([#145](https://github.com/waycrate/wayshot/pull/145), [@Gigas002](https://github.com/Gigas002)).

#### Changed

- libwaysip 0.4 and API follow-ups ([#152](https://github.com/waycrate/wayshot/pull/152), [`7d32d38`](https://github.com/waycrate/wayshot/commit/7d32d389833f2ab6d4fe91bdaa6ad4dca9ee90d5), [`8f97ffe`](https://github.com/waycrate/wayshot/commit/8f97ffe00f1520f14b681852171412f7c7f4481f), [@uncomfyhalomacro](https://github.com/uncomfyhalomacro)).

#### Breaking Changes

- colored output, help text, and option renames (“friendly CLI”) ([#134](https://github.com/waycrate/wayshot/pull/134), [@Gigas002](https://github.com/Gigas002)).
  - **Migration:** update scripts and wrappers that parse `--help`, grep for old long option names, or assume plain stderr; use the current kebab-case flags and check `wayshot --help` for aliases where the old spellings still work.
- region selection uses **libwaysip** in-process instead of spawning **slurp** ([#152](https://github.com/waycrate/wayshot/pull/152), [`586ee25`](https://github.com/waycrate/wayshot/commit/586ee258f824a6fc71d796272ffa9635236cc226), [@Decodetalkers](https://github.com/Decodetalkers)).
  - **Migration:** drop expectations that `slurp` is run externally or that `-s` forwards arguments to slurp; adjust automation that wrapped or mocked slurp; rely on wayshot’s built-in selector (and libwaysip behavior) only.

#### Fixed

- `-` / stdout no longer creates an empty file in cwd ([`a5575b6`](https://github.com/waycrate/wayshot/commit/a5575b6c9c3f12426309ac0309ce6f012cf4d729), [@uncomfyhalomacro](https://github.com/uncomfyhalomacro)).
- embedded region selection on adjacent outputs ([#199](https://github.com/waycrate/wayshot/pull/199), [@Pestdoktor](https://github.com/Pestdoktor)).

[1.4.0]: https://github.com/waycrate/wayshot/compare/1.3.1...v1.4.0

# Wayshot

## [1.4.5] 2026-01-20

### Hot fix: reexport ExtForeignToplevelHandle and fix ci

[1.4.5]: https://github.com/waycrate/wayshot/compare/1.4.5...v1.4.4

## [1.4.4] 2026-01-20

### Hot fix: reexport ExtForeignToplevelHandle

[1.4.4]: https://github.com/waycrate/wayshot/compare/1.4.4...v1.4.3

## [1.4.3] 2026-01-20

### Internal improvements
- Reusable region, size, and position data structures (PR #78)
- CLI design and argument typing improvements
- Freeze functionality additions
- Removal of explicit panics and exits
- Internal refactoring for screen capture components


### libwayshot 0.7.0

### There is Breaking changes in libwayshot, but it is just some tidy up for the api, mainly the order of params
- (libwayshot): use the r-egl lib which maintained by waycrate instead of khornos-egl
- (libwayshot): remove the Instance uses of egl lib. we only use the static feature
- (libwayshot): make tidy up of the api
- (libwayshot): Fix the problem that libwayshot panic when there is no support for ext-foreign-toplevel-list
- (waymirror-egl): tidy up the project, and add it to the workspace. Now it is usable, though still has bugs about keyboard

[1.4.3]: https://github.com/waycrate/wayshot/compare/1.4.2...v1.4.3

## [1.4.2] - 2025-12-18

### What's Changed
- (wayshot): Fix the problem that area selector cannot cross screens.
- (libwayshot): Handle gbm errors instead of panicking by @dhruvDev23 in #247
- (libwayshot): Enable ext-image-copy by @xonx4l in #245
- (libwayshot): Add dmabuf screen logic for screencast part by @Decodetalkers in #248

### New Contributors
- @dhruvDev23 made their first contribution in #247

[1.4.2]: https://github.com/waycrate/wayshot/compare/1.4.1...v1.4.2

## [1.4.1] - 2025-12-15

* add api for the screencast
* fix that outputcapture may not get the frameformat sometime

[1.4.1]: https://github.com/waycrate/wayshot/compare/1.4.0...v1.4.1

## [1.4.0] - 2025-12-13

### libwayshot 0.4.0

* dmabuf support by @sooraj-satheesh
* Support JPEG-XL by @Gias002
* Add unit test by @xonx4l
* Support ext_\*Protocol by @Rishik-Y
* Use libwaysip instead of slurp by @Decodetalkers
* update the libwaysip by @uncomfyhalomacro

### Breaking changes
* ext-image-copy is supported
* We have dmabuf support now, though still needs some adjustment
* screencopy is deprecated now for it has bugs on ext-image-copy. If it is fixed, it will comeback. Now please use screenshot_freeze instead

[1.4.0]: https://github.com/waycrate/wayshot/compare/1.3.1...v1.4.0

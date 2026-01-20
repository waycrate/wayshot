# Wayshot

## [1.4.3] (libwayshot 0.7.0)

### There is Breaking changes in libwayshot, but it is just some tidy up for the api, mainly the order of params
- (libwayshot): use the r-egl lib which maintained by waycrate instead of khornos-egl
- (libwayshot): remove the Instance uses of egl lib. we only use the static feature
- (libwayshot): make tidy up of the api
- (libwayshot): Fix the problem that libwayshot panic when there is no support for ext-foreign-toplevel-list
- (waymirror-egl): tidy up the project, and add it to the workspace. Now it is usable, though still has bugs about keyboard

## [1.4.2] (libwayshot 0.6.0)

### What's Changed
- (wayshot): Fix the problem that area selector cannot cross screens.
- (libwayshot): Handle gbm errors instead of panicking by @dhruvDev23 in #247
- (libwayshot): Enable ext-image-copy by @xonx4l in #245
- (libwayshot): Add dmabuf screen logic for screencast part by @Decodetalkers in #248

### New Contributors
- @dhruvDev23 made their first contribution in #247


## 1.4.1 (libwayshot 0.5.0)

* add api for the screencast
* fix that outputcapture may not get the frameformat sometime

## 1.4.0 (libwayshot 0.4.0)

* dmabuf support by @sooraj-satheesh
* Support JPEG-XL by @Gias002
* Add unit test by @xonx4l
* Support ext_\*Protocol by @Rishik-Y
* Use libwaysip instead of slurp by @Decodetalkers
* update the libwaysip by @uncomfyhalomacro

## Breaking changes
* ext-image-copy is supported
* We have dmabuf support now, though still needs some adjustment
* screencopy is deprecated now for it has bugs on ext-image-copy. If it is fixed, it will comeback. Now please use screenshot_freeze instead

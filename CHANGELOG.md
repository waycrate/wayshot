# Wayshot

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

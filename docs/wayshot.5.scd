wayshot(5) "github.com/waycrate/wayshot" "Config file manual"

# NAME

Wayshot - Screenshot tool for compositors implementing zwlr_screencopy_v1 such as sway and river

# SYNTAX

Wayshot's configuration file uses the TOML format. The format's specification
can be found at _https://toml.io/en/v1.0.0_

# LOCATION

Wayshot doesn't create the config file for you, but it looks for one in the
following locations:

. Path, provided via CLI options: _wayshot --config config.toml_
. _$XDG_CONFIG_HOME/wayshot/config.toml_
. _$HOME/.config/wayshot/config.toml_

# BASE

This section documents the *[base]* table of the configuration file

*output* = _"<string>"_ | _"None"_

	Name of the output to take screenshot

	CLI option takes precedence: _wayshot --output "HDMI-A-1"_

	Default: _"None"_ (capture all outputs)

*cursor* = _true_ | _false_

	Controls if screenshot file should contain cursor or not

	CLI option takes precedence: _wayshot --cursor_

	Default: _false_

*clipboard* = _true_ | _false_

	Controls if screenshot file should be copied into clipboard.
	Wayshot persists in the background offering the image until the clipboard is overwritten

	CLI option takes precedence: _wayshot --clipboard_

	Default: _true_

*file* = _true_ | _false_

	Controls if screenshot file should be written on disk.
	CLI _FILE_ takes precedence over it.
	If it's not used, then _[file]_ section's _path_ and *name_format* are used as path to save file on disk

	Examples:
		- _file_ is _true_, CLI without args: _wayshot_ -> screenshot file saved in *path/name_format.encoding*
		- _file_ is _true_, CLI with _FILE_ argument: _wayshot foo.png_ -> screenshot file saved in current directory as *foo.png*
		- _file_ is _true_, CLI with _FILE_ argument as _-_ (stdout): _wayshot "-"_ -> screenshot file saved in *path/name_format.encoding*
		- _file_ is _false_, CLI without args: _wayshot_ -> screenshot file is not saved anywhere

	Default: _true_

*stdout* = _true_ | _false_

	Controls if screenshot file should be sent to stdout

	CLI's _FILE_ takes precedence: _wayshot "-"_

	Default: _false_

*log_level* = _"info"_ | _"trace"_ | _"debug"_ | _"warn"_ | _"error"_

	Log level. Possible values:
		- _"info"_
		- _"trace"_
		- _"debug"_
		- _"warn"_
		- _"error"_

	Default: _"info"_

# FILE

This section documents the *[file]* table of the configuration file

*path* = _"<string>"_ | _"None"_

	Directory to save screenshot file.
	Takes effect only if _[base]_ category's _file_ is _true_ and _FILE_ is not provided in CLI

	Refer to _[base]_ category's _file_ for examples

	Default: _"None"_ ($HOME/Pictures/Screenshots, fallbacks to current working directory)

*name_format* = _"<string>"_ | _"None"_

	Formatting of screenshot file name to save on disk.
	Refer to chrono formatting rules: https://docs.rs/chrono/latest/chrono/format/strftime/index.html

	CLI option takes precedence: *wayshot --file-name-format "wayshot-%Y_%m_%d-%H_%M_%S"*

	Default: _"None"_ (fallbacks to *"wayshot-%Y_%m_%d-%H_%M_%S"*)

*encoding* = _"png"_ | _"jpg"_ | _"ppm"_ | _"qui"_ | _"webp"_ | _"avif"_ | _"jxl"_

	Screenshot file encoding

	Examples:
		- _encoding_ is _avif_, CLI: _wayshot foo_ -> screenshot file is _foo.avif_
		- _encoding_ is _avif_, CLI: _wayshot --encoding png foo_ -> screenshot file is _foo.png_
		- _encoding_ is _avif_, CLI: _wayshot --encoding png foo.jpg_ -> screenshot file is _foo.jpg_
		- _encoding_ is _avif_, CLI: _wayshot --encoding png foo.jpg --encoding png_ -> screenshot file is _foo.png_

	Default: _"png"_

## ENCODING

	Encoding-specific configuration options.

## PNG

*[encoding.png] compression* = _"default"_ | _"fast"_ | _"best"_ | _"none"_ | _"uncompressed"_ | _0-9_

	PNG compression level. Controls the trade-off between file size and encoding speed.
	For detailed documentation, see: https://docs.rs/image/latest/image/codecs/png/enum.CompressionType.html

	- _"default"_: Balanced compression and speed
	- _"fast"_: Faster encoding, larger files (minimal compression)
	- _"best"_: Slower encoding, smaller files (high compression level)
	- _"none"_ or _"uncompressed"_: No compression whatsoever
	- _0-9_: Detailed numeric compression level (0=no compression, 9=maximum compression)

	Default: _"default"_

*[encoding.png] filter* = _"none"_ | _"sub"_ | _"up"_ | _"avg"_ | _"paeth"_ | _"adaptive"_

	PNG filter type. Filters process image data to improve compression.
	For detailed documentation, see: https://docs.rs/image/latest/image/codecs/png/enum.FilterType.html

	- _"none"_: No filtering (best for low bit depth grayscale or low color count)
	- _"sub"_: Filters based on previous pixel in the same scanline
	- _"up"_: Filters based on the scanline above
	- _"avg"_: Filters based on average of left and right neighbor pixels
	- _"paeth"_: Algorithm using left, upper left, and above pixels
	- _"adaptive"_: Heuristic selection of filters per scanline (recommended)

	Default: _"adaptive"_

## JXL

*[encoding.jxl] lossless* = _true_ | _false_

	Enable lossless compression for JPEG-XL encoding.
	When set to true, the image is compressed without any quality loss.
	When false, lossy compression is used with the specified distance parameter.

	Default: _false_

*[encoding.jxl] distance* = _0.0-25.0_

	Target visual distance in JND (Just Noticeable Difference) units for lossy compression.
	Lower values result in higher quality but larger file sizes.
	This parameter is ignored when lossless is set to true.

	- _0.0_: Mathematically lossless (same as lossless=true)
	- _1.0_: Visually lossless (recommended default)
	- _0.5-3.0_: Recommended range for most use cases
	- _3.0-25.0_: Higher compression, noticeable quality loss

	Default: _1.0_

*[encoding.jxl] effort* = _1-10_

	Encoder effort setting. Higher values allow more computation time for better compression.
	For lossless compression, higher effort generally produces smaller files.
	For lossy compression, higher effort should more accurately reach the target quality.

	- _1_: Lightning (fastest)
	- _2_: Thunder
	- _3_: Falcon
	- _4_: Cheetah
	- _5_: Hare
	- _6_: Wombat
	- _7_: Squirrel (default, balanced)
	- _8_: Kitten
	- _9_: Tortoise
	- _10_: Glacier (slowest, best compression)

	Default: _7_

# SEE ALSO
	- wayshot(1)
	- wayshot(7)

# AUTHORS

Maintained by Shinyzenith <aakashsensharma@gmail.com>.
For more information about development, see <https://github.com/waycrate/wayshot>.

wayshot(1) "github.com/waycrate/wayshot" "General Commands Manual"

# NAME

Wayshot - Screenshot tool for compositors implementing zwlr_screencopy_v1 such as sway and river

# SYNOPSIS

*wayshot* [_options_] [_output_]

# ARGUMENTS

*output*,
	Location to send captured screenshot to, it can be of the following types:
		1. A directory, image outputs will be saved in the default format "wayshot-yyyy-mm-dd-hh-mm-ss.png" (Parent directories will be created if they do not exist)

		2. A path (Encoding is automatically inferred from the extension).

		3. '-' which sends the screenshot to stdout.

	The *--clipboard* option can also be used simultaneously with any of the above to copy the image to the clipboard too.

# OPTIONS

*-h*,
	Print concise help messages and quit.

*--help*,
	Print full help message and quit.

*-V*, *--version*
	Print version information.

*--log-level <LOG_LEVEL>*
	Log level to be used for printing to stderr
	Possible values: trace, debug, info, warn, error

	Default value: info

*-c*, *--cursor*
	Enable cursor visibility in screenshots.

*--clipboard*
	Copy image contents to clipboard also.
	Using this flag will cause the wayshot process to fork and persist in the background offering the image
	on the wayland clipboard until some other program overwrites the clipboard.

*-e*, *--extension*
	Set the image encoder. Without this option, encoding is either inferred from the *output* filename or defaults to png.
	Valid arguments:
		- jpg
		- png (Default encoder)
		- ppm
		- qoi
		- webp
		- avif
		- jxl

*--color*
	Pick color, and print the information

*-l*, *--list-outputs*
	List all valid output names. This flag is generally used in combination with *-o* flag.

*--choose-output*
	Present a fuzzy selector for display (wl_output) selection.

*-o*, *--output*
	Choose a particular display (wl_output) to screenshot.

*-g*, *--geometry*
	If this option is passed, wayshot takes a screenshot first and then uses
	libwaysip (https://github.com/waycrate/waysip) to select a portion of that screenshot.

*--file-name-format*
	Output file name's formatting. Refer to chrono formatting rules: https://docs.rs/chrono/latest/chrono/format/strftime/index.html.

	Example: --file-name-format "%Y_%m_%d-%H_%M_%S"

	Default: wayshot-%Y_%m_%d-%H_%M_%S

*--config*
	Path to your config file. Defaults to:

	- *$XDG_CONFIG_HOME/wayshot/config.toml*
	- *$HOME/wayshot/config.toml* if *$XDG_CONFIG_HOME* variable doesn't exist
	- *None* if config isn't found, the default values will be used

	Example: *wayshot --config config.toml*


# SEE ALSO
	- wayshot(5)
	- wayshot(7)

# AUTHORS

Maintained by Shinyzenith <aakashsensharma@gmail.com>.
For more information about development, see <https://github.com/waycrate/wayshot>.

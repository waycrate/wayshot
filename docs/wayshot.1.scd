wayshot(1) "github.com/waycrate/wayshot" "General Commands Manual"

# NAME

Wayshot - Screenshot tool for compositors implementing zwlr_screencopy_v1 such as sway and river

# SYNOPSIS

*wayshot* [_options_] [_output_]

# ARGUMENTS

*output*,
	Location to send captured screenshot to, it can be of the following types:
		1. A directory, image outputs will be saved in the default format "wayshot-yyyy-mm-dd-hh-mm-ss.png"

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

## Query (print info, then exit — no screenshot is taken)

*-l*, *--list-outputs*
	List all connected output names.

*--list-outputs-info*
	List all connected outputs with their name, description, size, and position.

*--list-toplevels*, *--list-windows*
	List all active toplevel windows.

*--color*
	Click a pixel and print its RGBA and hex color value. By default the screen is frozen first;
	use *--no-freeze* to pick on the live display.

	_Requires the_ *color_picker* _feature (enabled by default)._

## Capture target (what to capture)

*-g*, *--geometry*
	Interactively select a screen region. By default the screen is frozen before the selection
	overlay is shown, giving a stable view for precise selection. Use *--no-freeze* to select on
	the live display and capture after selection.

	_Requires the_ *selector* _feature (enabled by default)._

*-o*, *--output <OUTPUT>*
	Capture a specific output/display by name. Use *--list-outputs* to discover names.

*--choose-output*
	Present a fuzzy selector for output/display selection.

*--toplevel <APP_ID TITLE>*, *--window <APP_ID TITLE>*
	Capture a specific toplevel window identified by its "app_id title" string.
	Use *--list-toplevels* to discover available strings.

*--choose-toplevel*, *--choose-window*
	Present a fuzzy selector for toplevel/window selection.

## Capture options (how to capture)

*-c*, *--cursor*
	Include the cursor in the screenshot.

*--no-freeze*
	Do not freeze the screen when selecting a region (*--geometry*) or a point (*--color*).
	Selection is done on the live display; the capture is taken after selection.
	Config: *[base] freeze* (see wayshot(5)).

## Output options (where/how to save the image)

*-e*, *--encoding*, *--extension*, *--format*, *--file-format <FILE_EXTENSION>*
	Set the image encoder. Without this option, encoding is either inferred from the *output*
	filename or defaults to png.
	Valid arguments:
		- jpg
		- png (default)
		- ppm
		- qoi
		- webp
		- avif _(requires_ *avif* _feature, enabled by default)_
		- jxl  _(requires_ *jxl* _feature, enabled by default)_

*--file-name-format <FORMAT>*
	Format string for the output file name. Refer to chrono strftime formatting rules:
	https://docs.rs/chrono/latest/chrono/format/strftime/index.html

	Example: --file-name-format "%Y_%m_%d-%H_%M_%S"

	Default: wayshot-%Y_%m_%d-%H_%M_%S

*--clipboard*
	Copy the screenshot to the Wayland clipboard.
	Wayshot forks a child process that serves the image until the clipboard is overwritten.

	_Requires the_ *clipboard* _feature (enabled by default)._

## Notification options

*--silent*, *--no-notifications*
	Suppress the desktop notification after a screenshot.

	_Requires the_ *notifications* _feature (enabled by default)._

## Global options

*--config <PATH>*
	Path to the config file. Defaults to:

	- *$XDG_CONFIG_HOME/wayshot/config.toml*
	- *$HOME/.config/wayshot/config.toml* when *$XDG_CONFIG_HOME* is unset
	- Built-in defaults when no file is found

	Example: *wayshot --config config.toml*

*--log-level <LOG_LEVEL>*
	Log level written to stderr.
	Possible values: trace, debug, info, warn, error

	Default value: info

	_Requires the_ *logger* _feature (enabled by default)._

# FEATURES

Wayshot supports optional Cargo features. All features are enabled by default in the official binary.
To build with a reduced set, use *--no-default-features* together with *--features*:

	cargo build --no-default-features --features clipboard,logger

Available features: *avif*, *clipboard*, *color_picker*, *jxl*, *logger*, *notifications*, *selector*

# SEE ALSO
	- wayshot(5)
	- wayshot(7)

# AUTHORS

Maintained by Shinyzenith <aakashsensharma@gmail.com>.
For more information about development, see <https://github.com/waycrate/wayshot>.

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

	Default: _"None"_ (current working directory)

*name_format* = _"<string>"_ | _"None"_

	Formatting of screenshot file name to save on disk.
    Refer to chrono formatting rules: https://docs.rs/chrono/latest/chrono/format/strftime/index.html

    CLI option takes precedence: *wayshot --file-name-format "wayshot-%Y_%m_%d-%H_%M_%S"*

	Default: _"None"_ (fallbacks to *"wayshot-%Y_%m_%d-%H_%M_%S"*)

*encoding* = _"png"_ | _"jpg"_ | _"ppm"_ | _"qui"_ | _"webp"_ | _"avif"_

	Screenshot file encoding

    Examples:
        - _encoding_ is _avif_, CLI: _wayshot foo_ -> screenshot file is _foo.avif_
        - _encoding_ is _avif_, CLI: _wayshot --encoding png foo_ -> screenshot file is _foo.png_
        - _encoding_ is _avif_, CLI: _wayshot --encoding png foo.jpg_ -> screenshot file is _foo.jpg_
        - _encoding_ is _avif_, CLI: _wayshot --encoding png foo.jpg --encoding png_ -> screenshot file is _foo.png_

	Default: _"png"_

# SEE ALSO
	- wayshot(1)
	- wayshot(7)

# AUTHORS

Maintained by Shinyzenith <aakashsensharma@gmail.com>.
For more information about development, see <https://github.com/waycrate/wayshot>.

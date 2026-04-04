wayshot(7) "github.com/waycrate/wayshot" "Miscellaneous Information Manual"

# NAME

Wayshot - Screenshot tool for compositors implementing zwlr_screencopy_v1 such as sway and river

# SYNOPSIS

*wayshot* [_options_] [_output_]

# REGION SELECTION

Interactively select a screen region using the waysip overlay (requires *selector* feature):

	wayshot -g

Optionally, with custom background/foreground colors:

	wayshot -g --geometry-background-color 00000050 --geometry-foreground-color ffffffff

Capture a region using a geometry string in slurp/grim format "_x_,_y_ _width_x_height_":

	wayshot FILE -g "$(slurp)"
	wayshot -g "783,746 177x251"

# CAPTURE FULLSCREEN

Capture all connected outputs:

	wayshot

# CUSTOM FILE PATH AND EXTENSION

	wayshot ../screenshot.png --extension ppm

# SCREENSHOT AND COPY TO CLIPBOARD

	wayshot --clipboard

# SCREENSHOT A PARTICULAR DISPLAY

	wayshot -l # list available outputs, pick one name
	wayshot -o eDP-1

# INTERACTIVELY CHOOSE A DISPLAY

	wayshot --choose-output

# CAPTURE A SPECIFIC TOPLEVEL WINDOW

	wayshot --list-toplevels # list active windows, copy the "app_id title" string
	wayshot --toplevel "org.mozilla.firefox Firefox"

# INTERACTIVELY CHOOSE A TOPLEVEL WINDOW

	wayshot --choose-toplevel

# PICK A COLOR WITH INFORMATION

	wayshot --color

# PICK A HEX COLOR CODE, USING IMAGEMAGICK

	wayshot -g - | convert - -format '%[pixel:p{0,0}]' txt:-|grep -E "#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})" -o

# PICK A HEX COLOR CODE WITHOUT USING IMAGEMAGICK

	wayshot -g - --encoding ppm | tail -c 3 | od -An -tuC | xargs printf '#%02X%02X%02X\n'

# PICK A COLOR, USING IMAGEMAGICK

	wayshot -g - | convert - -format '%[pixel:p{0,0}]' txt:-

# SHELL COMPLETIONS

Generate and install a completion script for your shell (requires *completions* feature):

*fish*

	wayshot --completions fish | source

*zsh*

	wayshot --completions zsh > ~/.zfunc/\_wayshot

*bash*

	wayshot --completions bash > /etc/bash\_completion.d/wayshot

*elvish*

	wayshot --completions elvish >> ~/.config/elvish/rc.elv

*PowerShell (pwsh)*

	wayshot --completions pwsh >> $PROFILE

*nushell*

	wayshot --completions nushell | save -f ~/.config/nushell/completions/wayshot.nu
	# then source it from env.nu

# AUTHORS

Maintained by Shinyzenith <aakashsensharma@gmail.com>.
For more information about development, see <https://github.com/waycrate/wayshot>.

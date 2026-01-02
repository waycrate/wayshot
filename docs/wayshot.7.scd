wayshot(7) "github.com/waycrate/wayshot" "Miscellaneous Information Manual"

# NAME

Wayshot - Screenshot tool for compositors implementing zwlr_screencopy_v1 such as sway and river

# SYNOPSIS

*wayshot* [_options_] [_output_]

# REGION SELECTION
wayshot -g

# CAPTURE FULLSCREEN

wayshot

# CUSTOM FILE PATH AND EXTENSION

wayshot ../screenshot.png --extension ppm

# SCREENSHOT AND COPY TO CLIPBOARD

wayshot --clipboard

# SCREENSHOT A PARTICULAR DISPLAY

wayshot -l # Pick any output name from the following. We use eDP-1 for this example.
wayshot -o eDP-1

# PICK A COLOR WITH INFORMATION

wayshot --color

# PICK A HEX COLOR CODE, USING IMAGEMAGICk

wayshot -g - | convert - -format '%[pixel:p{0,0}]' txt:-|grep -E "#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})" -o

# PICK A HEX COLOR CODE WITHOUT USING IMAGEMAGICK

wayshot -g - --encoding ppm | tail -c 3 | od -An -tuC | xargs printf '#%02X%02X%02X\n'

# PICK A COLOR, USING IMAGEMAGICK

wayshot -g - | convert - -format '%[pixel:p{0,0}]' txt:-

# AUTHORS

Maintained by Shinyzenith <aakashsensharma@gmail.com>.
For more information about development, see <https://github.com/waycrate/wayshot>.

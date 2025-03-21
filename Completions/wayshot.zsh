#compdef wayshot

autoload -U is-at-least

_wayshot() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" \
'--log-level=[Log level to be used for printing to stderr]:LOG_LEVEL:(trace debug info warn error)' \
'-s+[Arguments to call slurp with for selecting a region]' \
'--slurp=[Arguments to call slurp with for selecting a region]' \
'--encoding=[Set image encoder, by default uses the file extension from the OUTPUT positional argument. Otherwise defaults to png.]:FILE_EXTENSION:((jpg\:"JPG/JPEG encoder"
png\:"PNG encoder"
ppm\:"PPM encoder"
qoi\:"Qut encoder"
webp\:"WebP encoder,"))' \
'--extension=[Set image encoder, by default uses the file extension from the OUTPUT positional argument. Otherwise defaults to png.]:FILE_EXTENSION:((jpg\:"JPG/JPEG encoder"
png\:"PNG encoder"
ppm\:"PPM encoder"
qoi\:"Qut encoder"
webp\:"WebP encoder,"))' \
'--format=[Set image encoder, by default uses the file extension from the OUTPUT positional argument. Otherwise defaults to png.]:FILE_EXTENSION:((jpg\:"JPG/JPEG encoder"
png\:"PNG encoder"
ppm\:"PPM encoder"
qoi\:"Qut encoder"
webp\:"WebP encoder,"))' \
'--output-format=[Set image encoder, by default uses the file extension from the OUTPUT positional argument. Otherwise defaults to png.]:FILE_EXTENSION:((jpg\:"JPG/JPEG encoder"
png\:"PNG encoder"
ppm\:"PPM encoder"
qoi\:"Qut encoder"
webp\:"WebP encoder,"))' \
'(-s --slurp)-o+[Choose a particular output/display to screenshot]:OUTPUT: ' \
'(-s --slurp)--output=[Choose a particular output/display to screenshot]:OUTPUT: ' \
'--generate-completions=[This Command helps you generate autocomplete in your desired Shell environment]:GENERATE_COMPLETIONS:(bash elvish fish powershell zsh)' \
'--clipboard[Copy image to clipboard. Can be used simultaneously with \[OUTPUT\] or stdout. Wayshot persists in the background offering the image till the clipboard is overwritten.]' \
'-c[Enable cursor in screenshots]' \
'--cursor[Enable cursor in screenshots]' \
'-l[List all valid outputs]' \
'--list-outputs[List all valid outputs]' \
'(-s --slurp -o --output)--choose-output[Present a fuzzy selector for output/display selection]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
'::file -- Custom output path can be of the following types\:
    1. Directory (Default naming scheme is used for the image output).
    2. Path (Encoding is automatically inferred from the extension).
    3. `-` (Indicates writing to terminal \[stdout\]).:_files' \
&& ret=0
}

(( $+functions[_wayshot_commands] )) ||
_wayshot_commands() {
    local commands; commands=()
    _describe -t commands 'wayshot commands' commands "$@"
}

if [ "$funcstack[1]" = "_wayshot" ]; then
    _wayshot "$@"
else
    compdef _wayshot wayshot
fi

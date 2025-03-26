complete -c wayshot -l log-level -d 'Log level to be used for printing to stderr' -r -f -a "{trace	'',debug	'',info	'',warn	'',error	''}"
complete -c wayshot -s s -l slurp -d 'Arguments to call slurp with for selecting a region' -r
complete -c wayshot -l encoding -l extension -l format -l output-format -d 'Set image encoder, by default uses the file extension from the OUTPUT positional argument. Otherwise defaults to png.' -r -f -a "{jpg	'JPG/JPEG encoder',png	'PNG encoder',ppm	'PPM encoder',qoi	'Qut encoder',webp	'WebP encoder,'}"
complete -c wayshot -s o -l output -d 'Choose a particular output/display to screenshot' -r
complete -c wayshot -l generate-completions -d 'This Command helps you generate autocomplete in your desired Shell environment' -r -f -a "{bash	'',elvish	'',fish	'',powershell	'',zsh	''}"
complete -c wayshot -l clipboard -d 'Copy image to clipboard. Can be used simultaneously with [OUTPUT] or stdout. Wayshot persists in the background offering the image till the clipboard is overwritten.'
complete -c wayshot -s c -l cursor -d 'Enable cursor in screenshots'
complete -c wayshot -s l -l list-outputs -d 'List all valid outputs'
complete -c wayshot -l choose-output -d 'Present a fuzzy selector for output/display selection'
complete -c wayshot -s h -l help -d 'Print help (see more with \'--help\')'
complete -c wayshot -s V -l version -d 'Print version'

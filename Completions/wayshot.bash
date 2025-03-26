_wayshot() {
    local i cur prev opts cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="wayshot"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        wayshot)
            opts="-s -c -l -o -h -V --clipboard --log-level --slurp --cursor --extension --format --output-format --encoding --list-outputs --output --choose-output --generate-completions --help --version [OUTPUT]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --log-level)
                    COMPREPLY=($(compgen -W "trace debug info warn error" -- "${cur}"))
                    return 0
                    ;;
                --slurp)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -s)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --encoding)
                    COMPREPLY=($(compgen -W "jpg png ppm qoi webp" -- "${cur}"))
                    return 0
                    ;;
                --extension)
                    COMPREPLY=($(compgen -W "jpg png ppm qoi webp" -- "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "jpg png ppm qoi webp" -- "${cur}"))
                    return 0
                    ;;
                --output-format)
                    COMPREPLY=($(compgen -W "jpg png ppm qoi webp" -- "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --generate-completions)
                    COMPREPLY=($(compgen -W "bash elvish fish powershell zsh" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _wayshot -o nosort -o bashdefault -o default wayshot
else
    complete -F _wayshot -o bashdefault -o default wayshot
fi

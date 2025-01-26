_hd_fpv_video_tool() {
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
                cmd="hd_fpv_video_tool"
                ;;
            hd_fpv_video_tool,add-audio-stream)
                cmd="hd_fpv_video_tool__add__audio__stream"
                ;;
            hd_fpv_video_tool,cut-video)
                cmd="hd_fpv_video_tool__cut__video"
                ;;
            hd_fpv_video_tool,display-osd-file-info)
                cmd="hd_fpv_video_tool__display__osd__file__info"
                ;;
            hd_fpv_video_tool,fix-video-audio)
                cmd="hd_fpv_video_tool__fix__video__audio"
                ;;
            hd_fpv_video_tool,generate-man-pages)
                cmd="hd_fpv_video_tool__generate__man__pages"
                ;;
            hd_fpv_video_tool,generate-overlay-frames)
                cmd="hd_fpv_video_tool__generate__overlay__frames"
                ;;
            hd_fpv_video_tool,generate-overlay-video)
                cmd="hd_fpv_video_tool__generate__overlay__video"
                ;;
            hd_fpv_video_tool,generate-shell-autocompletion-files)
                cmd="hd_fpv_video_tool__generate__shell__autocompletion__files"
                ;;
            hd_fpv_video_tool,help)
                cmd="hd_fpv_video_tool__help"
                ;;
            hd_fpv_video_tool,play-video-with-osd)
                cmd="hd_fpv_video_tool__play__video__with__osd"
                ;;
            hd_fpv_video_tool,splice-videos)
                cmd="hd_fpv_video_tool__splice__videos"
                ;;
            hd_fpv_video_tool,transcode-video)
                cmd="hd_fpv_video_tool__transcode__video"
                ;;
            hd_fpv_video_tool__help,add-audio-stream)
                cmd="hd_fpv_video_tool__help__add__audio__stream"
                ;;
            hd_fpv_video_tool__help,cut-video)
                cmd="hd_fpv_video_tool__help__cut__video"
                ;;
            hd_fpv_video_tool__help,display-osd-file-info)
                cmd="hd_fpv_video_tool__help__display__osd__file__info"
                ;;
            hd_fpv_video_tool__help,fix-video-audio)
                cmd="hd_fpv_video_tool__help__fix__video__audio"
                ;;
            hd_fpv_video_tool__help,generate-man-pages)
                cmd="hd_fpv_video_tool__help__generate__man__pages"
                ;;
            hd_fpv_video_tool__help,generate-overlay-frames)
                cmd="hd_fpv_video_tool__help__generate__overlay__frames"
                ;;
            hd_fpv_video_tool__help,generate-overlay-video)
                cmd="hd_fpv_video_tool__help__generate__overlay__video"
                ;;
            hd_fpv_video_tool__help,generate-shell-autocompletion-files)
                cmd="hd_fpv_video_tool__help__generate__shell__autocompletion__files"
                ;;
            hd_fpv_video_tool__help,help)
                cmd="hd_fpv_video_tool__help__help"
                ;;
            hd_fpv_video_tool__help,play-video-with-osd)
                cmd="hd_fpv_video_tool__help__play__video__with__osd"
                ;;
            hd_fpv_video_tool__help,splice-videos)
                cmd="hd_fpv_video_tool__help__splice__videos"
                ;;
            hd_fpv_video_tool__help,transcode-video)
                cmd="hd_fpv_video_tool__help__transcode__video"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        hd_fpv_video_tool)
            opts="-l -h -V --log-level --help --version display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --log-level)
                    COMPREPLY=($(compgen -W "off error warn info debug trace" -- "${cur}"))
                    return 0
                    ;;
                -l)
                    COMPREPLY=($(compgen -W "off error warn info debug trace" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__add__audio__stream)
            opts="-y -h --audio-encoder --audio-bitrate --overwrite --help <INPUT_VIDEO_FILE> [OUTPUT_VIDEO_FILE]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --audio-encoder)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --audio-bitrate)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__cut__video)
            opts="-s -e -y -h --start --end --overwrite --help <INPUT_VIDEO_FILE> [OUTPUT_VIDEO_FILE]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --start)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -s)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --end)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -e)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__display__osd__file__info)
            opts="-h --help <OSD_FILE>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__fix__video__audio)
            opts="-s -v -y -h --sync --volume --overwrite --help <INPUT_VIDEO_FILE> [OUTPUT_VIDEO_FILE]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__generate__man__pages)
            opts="-h --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__generate__overlay__frames)
            opts="-v -r -s -n -f -i -o -h --target-video-file --hide-regions --hide-items --start --end --target-resolution --scaling --no-scaling --min-margins --min-coverage --font-dir --font-ident --frame-shift --help <OSD_FILE> [OUTPUT_DIR]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --target-video-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -v)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hide-regions)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hide-items)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --start)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --end)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --target-resolution)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -r)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-margins)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-coverage)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --font-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --font-ident)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --frame-shift)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__generate__overlay__video)
            opts="-v -r -s -n -f -i -o -c -y -h --target-video-file --hide-regions --hide-items --start --end --target-resolution --scaling --no-scaling --min-margins --min-coverage --font-dir --font-ident --frame-shift --codec --overwrite --help <OSD_FILE> [VIDEO_FILE]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --target-video-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -v)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hide-regions)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hide-items)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --start)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --end)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --target-resolution)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -r)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-margins)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-coverage)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --font-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --font-ident)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --frame-shift)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --codec)
                    COMPREPLY=($(compgen -W "vp8 vp9" -- "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -W "vp8 vp9" -- "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__generate__shell__autocompletion__files)
            opts="-h --help <SHELL>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help)
            opts="display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__add__audio__stream)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__cut__video)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__display__osd__file__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__fix__video__audio)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__generate__man__pages)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__generate__overlay__frames)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__generate__overlay__video)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__generate__shell__autocompletion__files)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__play__video__with__osd)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__splice__videos)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__help__transcode__video)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__play__video__with__osd)
            opts="-h --help <VIDEO_FILE> [OSD_VIDEO_FILE]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__splice__videos)
            opts="-y -h --overwrite --help [INPUT_VIDEO_FILES]... <OUTPUT>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        hd_fpv_video_tool__transcode__video)
            opts="-s -n -d -i -o -O -F -a -f -v -u -r -y -h --osd --osd-scaling --no-osd-scaling --min-osd-margins --min-osd-coverage --osd-font-dir --osd-font-ident --osd-frame-shift --osd-hide-regions --osd-hide-items --osd-overlay-video --osd-overlay-video-codec --osd-overlay-video-file --osd-file --add-audio --fix-audio --fix-audio-volume --fix-audio-sync --video-encoder --video-bitrate --video-crf --video-resolution --remove-video-defects --audio-encoder --audio-bitrate --start --end --overwrite --help <INPUT_VIDEO_FILE> [OUTPUT_VIDEO_FILE]"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --min-osd-margins)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-osd-coverage)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --osd-font-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --osd-font-ident)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --osd-frame-shift)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --osd-hide-regions)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --osd-hide-items)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --osd-overlay-video-codec)
                    COMPREPLY=($(compgen -W "vp8 vp9" -- "${cur}"))
                    return 0
                    ;;
                --osd-overlay-video-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --osd-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -F)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --video-encoder)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --video-bitrate)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --video-crf)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --video-resolution)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -r)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --remove-video-defects)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --audio-encoder)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --audio-bitrate)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --start)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --end)
                    COMPREPLY=($(compgen -f "${cur}"))
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
    complete -F _hd_fpv_video_tool -o nosort -o bashdefault -o default hd_fpv_video_tool
else
    complete -F _hd_fpv_video_tool -o bashdefault -o default hd_fpv_video_tool
fi

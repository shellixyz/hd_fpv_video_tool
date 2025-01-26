#compdef hd_fpv_video_tool

autoload -U is-at-least

_hd_fpv_video_tool() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'-l+[]:LOG_LEVEL:(off error warn info debug trace)' \
'--log-level=[]:LOG_LEVEL:(off error warn info debug trace)' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_hd_fpv_video_tool_commands" \
"*::: :->hd_fpv_video_tool" \
&& ret=0
    case $state in
    (hd_fpv_video_tool)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:hd_fpv_video_tool-command-$line[1]:"
        case $line[1] in
            (display-osd-file-info)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':osd_file:_files' \
&& ret=0
;;
(generate-overlay-frames)
_arguments "${_arguments_options[@]}" : \
'-v+[use the resolution from the specified video file to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode]:TARGET_VIDEO_FILE:_files' \
'--target-video-file=[use the resolution from the specified video file to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode]:TARGET_VIDEO_FILE:_files' \
'*--hide-regions=[hide rectangular regions from the OSD]:REGIONS:_default' \
'*--hide-items=[hide items from the OSD  Available items (font variant\: name list)\:   - Ardupilot\: gpslat, gpslon, alt, short+code, long+code   - INAV\: gpslat, gpslon, alt]:ITEM_NAMES:_default' \
'--start=[start timestamp]:[HH:]MM:SS:_default' \
'--end=[end timestamp]:[HH:]MM:SS:_default' \
'-r+[resolution used to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode]:TARGET_RESOLUTION:_default' \
'--target-resolution=[resolution used to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode]:TARGET_RESOLUTION:_default' \
'--min-margins=[minimum margins to decide whether scaling should be used and how much to scale]:horizontal:vertical:_default' \
'--min-coverage=[minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided]:percent:_default' \
'-f+[path to the directory containing font sets]:dirpath:_files' \
'--font-dir=[path to the directory containing font sets]:dirpath:_files' \
'-i+[force using this font identifier when loading fonts, default is automatic]:ident:_default' \
'--font-ident=[force using this font identifier when loading fonts, default is automatic]:ident:_default' \
'-o+[Shift the output by that number of frames. Use this option to sync the OSD to a particular video]:frames:_default' \
'--frame-shift=[Shift the output by that number of frames. Use this option to sync the OSD to a particular video]:frames:_default' \
'-s[force using scaling, default is automatic]' \
'--scaling[force using scaling, default is automatic]' \
'-n[force disable scaling, default is automatic]' \
'--no-scaling[force disable scaling, default is automatic]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':osd_file -- path to FPV.WTF .osd file:_files' \
'::output_dir -- directory in which the OSD frames will be written:_files' \
&& ret=0
;;
(generate-overlay-video)
_arguments "${_arguments_options[@]}" : \
'-v+[use the resolution from the specified video file to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode]:TARGET_VIDEO_FILE:_files' \
'--target-video-file=[use the resolution from the specified video file to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode]:TARGET_VIDEO_FILE:_files' \
'*--hide-regions=[hide rectangular regions from the OSD]:REGIONS:_default' \
'*--hide-items=[hide items from the OSD  Available items (font variant\: name list)\:   - Ardupilot\: gpslat, gpslon, alt, short+code, long+code   - INAV\: gpslat, gpslon, alt]:ITEM_NAMES:_default' \
'--start=[start timestamp]:[HH:]MM:SS:_default' \
'--end=[end timestamp]:[HH:]MM:SS:_default' \
'-r+[resolution used to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode]:TARGET_RESOLUTION:_default' \
'--target-resolution=[resolution used to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode]:TARGET_RESOLUTION:_default' \
'--min-margins=[minimum margins to decide whether scaling should be used and how much to scale]:horizontal:vertical:_default' \
'--min-coverage=[minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided]:percent:_default' \
'-f+[path to the directory containing font sets]:dirpath:_files' \
'--font-dir=[path to the directory containing font sets]:dirpath:_files' \
'-i+[force using this font identifier when loading fonts, default is automatic]:ident:_default' \
'--font-ident=[force using this font identifier when loading fonts, default is automatic]:ident:_default' \
'-o+[Shift the output by that number of frames. Use this option to sync the OSD to a particular video]:frames:_default' \
'--frame-shift=[Shift the output by that number of frames. Use this option to sync the OSD to a particular video]:frames:_default' \
'-c+[]:CODEC:(vp8 vp9)' \
'--codec=[]:CODEC:(vp8 vp9)' \
'-s[force using scaling, default is automatic]' \
'--scaling[force using scaling, default is automatic]' \
'-n[force disable scaling, default is automatic]' \
'--no-scaling[force disable scaling, default is automatic]' \
'-y[overwrite output file if it exists]' \
'--overwrite[overwrite output file if it exists]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':osd_file -- path to FPV.WTF .osd file:_files' \
'::video_file -- path of the video file to generate:_files' \
&& ret=0
;;
(cut-video)
_arguments "${_arguments_options[@]}" : \
'-s+[start timestamp]:[HH:]MM:SS:_default' \
'--start=[start timestamp]:[HH:]MM:SS:_default' \
'-e+[end timestamp]:[HH:]MM:SS:_default' \
'--end=[end timestamp]:[HH:]MM:SS:_default' \
'-y[overwrite output file if it exists]' \
'--overwrite[overwrite output file if it exists]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':input_video_file -- input video file path:_files' \
'::output_video_file -- output video file path:_files' \
&& ret=0
;;
(fix-video-audio)
_arguments "${_arguments_options[@]}" : \
'-s[fix audio sync only]' \
'--sync[fix audio sync only]' \
'-v[fix audio volume only]' \
'--volume[fix audio volume only]' \
'-y[overwrite output file if it exists]' \
'--overwrite[overwrite output file if it exists]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':input_video_file -- input video file path:_files' \
'::output_video_file -- output video file path:_files' \
&& ret=0
;;
(transcode-video)
_arguments "${_arguments_options[@]}" : \
'--min-osd-margins=[minimum margins to decide whether scaling should be used and how much to scale]:horizontal:vertical:_default' \
'--min-osd-coverage=[minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided]:percent:_default' \
'-d+[path to the directory containing font sets]:dirpath:_files' \
'--osd-font-dir=[path to the directory containing font sets]:dirpath:_files' \
'-i+[force using this font identifier when loading fonts, default is automatic]:ident:_default' \
'--osd-font-ident=[force using this font identifier when loading fonts, default is automatic]:ident:_default' \
'-o+[shift frames to sync OSD with video]:frames:_default' \
'--osd-frame-shift=[shift frames to sync OSD with video]:frames:_default' \
'*--osd-hide-regions=[hide rectangular regions from the OSD]:REGIONS:_default' \
'*--osd-hide-items=[hide items from the OSD  Available items (font variant\: name list)\:   - Ardupilot\: gpslat, gpslon, alt, short+code, long+code   - INAV\: gpslat, gpslon, alt]:OSD_ITEM_NAMES:_default' \
'--osd-overlay-video-codec=[]:OSD_OVERLAY_VIDEO_CODEC:(vp8 vp9)' \
'--osd-overlay-video-file=[path of the video file to generate]:OSD_OVERLAY_VIDEO_FILE:_files' \
'-F+[path to FPV.WTF .osd file to use to generate OSD frames to burn onto video]:OSD file path:_files' \
'--osd-file=[path to FPV.WTF .osd file to use to generate OSD frames to burn onto video]:OSD file path:_files' \
'--video-encoder=[video encoder to use]:VIDEO_ENCODER:_default' \
'--video-bitrate=[video max bitrate]:VIDEO_BITRATE:_default' \
'--video-crf=[video constant quality setting]:VIDEO_CRF:_default' \
'-r+[\[possible values\: 720p, 720p4\:3, 1080p, 1080p4\:3, <width>x<height>\]]:VIDEO_RESOLUTION:_default' \
'--video-resolution=[\[possible values\: 720p, 720p4\:3, 1080p, 1080p4\:3, <width>x<height>\]]:VIDEO_RESOLUTION:_default' \
'*--remove-video-defects=[remove video defects]:REGIONS:_default' \
'--audio-encoder=[audio encoder to use]:AUDIO_ENCODER:_default' \
'--audio-bitrate=[max audio bitrate]:AUDIO_BITRATE:_default' \
'--start=[start timestamp]:[HH:]MM:SS:_default' \
'--end=[end timestamp]:[HH:]MM:SS:_default' \
'--osd[burn OSD onto video, try to find the OSD file automatically]' \
'-s[force using scaling, default is automatic]' \
'--osd-scaling[force using scaling, default is automatic]' \
'-n[force disable scaling, default is automatic]' \
'--no-osd-scaling[force disable scaling, default is automatic]' \
'-O[generate OSD overlay video instead of burning it onto the video]' \
'--osd-overlay-video[generate OSD overlay video instead of burning it onto the video]' \
'-a[add audio stream to the output video]' \
'--add-audio[add audio stream to the output video]' \
'-f[fix DJI AU audio\: fix sync + volume]' \
'--fix-audio[fix DJI AU audio\: fix sync + volume]' \
'(-f --fix-audio)-v[fix DJI AU audio volume]' \
'(-f --fix-audio)--fix-audio-volume[fix DJI AU audio volume]' \
'(-f --fix-audio)-u[fix DJI AU audio sync]' \
'(-f --fix-audio)--fix-audio-sync[fix DJI AU audio sync]' \
'-y[overwrite output file if it exists]' \
'--overwrite[overwrite output file if it exists]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':input_video_file -- input video file path:_files' \
'::output_video_file -- output video file path:_files' \
&& ret=0
;;
(play-video-with-osd)
_arguments "${_arguments_options[@]}" : \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':video_file:_files' \
'::osd_video_file:_files' \
&& ret=0
;;
(splice-videos)
_arguments "${_arguments_options[@]}" : \
'-y[overwrite output file if it exists]' \
'--overwrite[overwrite output file if it exists]' \
'-h[Print help]' \
'--help[Print help]' \
'*::input_video_files -- input video files:_files' \
':output -- output video file path:_files' \
&& ret=0
;;
(add-audio-stream)
_arguments "${_arguments_options[@]}" : \
'--audio-encoder=[audio encoder to use]:AUDIO_ENCODER:_default' \
'--audio-bitrate=[max audio bitrate]:AUDIO_BITRATE:_default' \
'-y[overwrite output file if it exists]' \
'--overwrite[overwrite output file if it exists]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':input_video_file -- input video file path:_files' \
'::output_video_file -- output video file path:_files' \
&& ret=0
;;
(generate-shell-autocompletion-files)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':shell:_default' \
&& ret=0
;;
(generate-man-pages)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_hd_fpv_video_tool__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:hd_fpv_video_tool-help-command-$line[1]:"
        case $line[1] in
            (display-osd-file-info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(generate-overlay-frames)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(generate-overlay-video)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(cut-video)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(fix-video-audio)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(transcode-video)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(play-video-with-osd)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(splice-videos)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add-audio-stream)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(generate-shell-autocompletion-files)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(generate-man-pages)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_hd_fpv_video_tool_commands] )) ||
_hd_fpv_video_tool_commands() {
    local commands; commands=(
'display-osd-file-info:Display information about the specified OSD file' \
'generate-overlay-frames:Generate a transparent overlay frame sequence as PNG files from a .osd file' \
'generate-overlay-video:Generate an OSD overlay video to be displayed over another video' \
'cut-video:Cut a video file without transcoding by specifying the desired start and/or end timestamp' \
'fix-video-audio:Fix a DJI Air Unit video'\''s audio sync and/or volume' \
'transcode-video:Transcode a video file, optionally burning the OSD onto it' \
'play-video-with-osd:Play a video with OSD by overlaying a transparent OSD video in real time' \
'splice-videos:Splice videos files together' \
'add-audio-stream:Add a silent audio stream to a video file' \
'generate-shell-autocompletion-files:' \
'generate-man-pages:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'hd_fpv_video_tool commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__add-audio-stream_commands] )) ||
_hd_fpv_video_tool__add-audio-stream_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool add-audio-stream commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__cut-video_commands] )) ||
_hd_fpv_video_tool__cut-video_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool cut-video commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__display-osd-file-info_commands] )) ||
_hd_fpv_video_tool__display-osd-file-info_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool display-osd-file-info commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__fix-video-audio_commands] )) ||
_hd_fpv_video_tool__fix-video-audio_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool fix-video-audio commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__generate-man-pages_commands] )) ||
_hd_fpv_video_tool__generate-man-pages_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool generate-man-pages commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__generate-overlay-frames_commands] )) ||
_hd_fpv_video_tool__generate-overlay-frames_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool generate-overlay-frames commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__generate-overlay-video_commands] )) ||
_hd_fpv_video_tool__generate-overlay-video_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool generate-overlay-video commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__generate-shell-autocompletion-files_commands] )) ||
_hd_fpv_video_tool__generate-shell-autocompletion-files_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool generate-shell-autocompletion-files commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help_commands] )) ||
_hd_fpv_video_tool__help_commands() {
    local commands; commands=(
'display-osd-file-info:Display information about the specified OSD file' \
'generate-overlay-frames:Generate a transparent overlay frame sequence as PNG files from a .osd file' \
'generate-overlay-video:Generate an OSD overlay video to be displayed over another video' \
'cut-video:Cut a video file without transcoding by specifying the desired start and/or end timestamp' \
'fix-video-audio:Fix a DJI Air Unit video'\''s audio sync and/or volume' \
'transcode-video:Transcode a video file, optionally burning the OSD onto it' \
'play-video-with-osd:Play a video with OSD by overlaying a transparent OSD video in real time' \
'splice-videos:Splice videos files together' \
'add-audio-stream:Add a silent audio stream to a video file' \
'generate-shell-autocompletion-files:' \
'generate-man-pages:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'hd_fpv_video_tool help commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__add-audio-stream_commands] )) ||
_hd_fpv_video_tool__help__add-audio-stream_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help add-audio-stream commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__cut-video_commands] )) ||
_hd_fpv_video_tool__help__cut-video_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help cut-video commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__display-osd-file-info_commands] )) ||
_hd_fpv_video_tool__help__display-osd-file-info_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help display-osd-file-info commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__fix-video-audio_commands] )) ||
_hd_fpv_video_tool__help__fix-video-audio_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help fix-video-audio commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__generate-man-pages_commands] )) ||
_hd_fpv_video_tool__help__generate-man-pages_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help generate-man-pages commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__generate-overlay-frames_commands] )) ||
_hd_fpv_video_tool__help__generate-overlay-frames_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help generate-overlay-frames commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__generate-overlay-video_commands] )) ||
_hd_fpv_video_tool__help__generate-overlay-video_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help generate-overlay-video commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__generate-shell-autocompletion-files_commands] )) ||
_hd_fpv_video_tool__help__generate-shell-autocompletion-files_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help generate-shell-autocompletion-files commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__help_commands] )) ||
_hd_fpv_video_tool__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help help commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__play-video-with-osd_commands] )) ||
_hd_fpv_video_tool__help__play-video-with-osd_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help play-video-with-osd commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__splice-videos_commands] )) ||
_hd_fpv_video_tool__help__splice-videos_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help splice-videos commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__help__transcode-video_commands] )) ||
_hd_fpv_video_tool__help__transcode-video_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool help transcode-video commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__play-video-with-osd_commands] )) ||
_hd_fpv_video_tool__play-video-with-osd_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool play-video-with-osd commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__splice-videos_commands] )) ||
_hd_fpv_video_tool__splice-videos_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool splice-videos commands' commands "$@"
}
(( $+functions[_hd_fpv_video_tool__transcode-video_commands] )) ||
_hd_fpv_video_tool__transcode-video_commands() {
    local commands; commands=()
    _describe -t commands 'hd_fpv_video_tool transcode-video commands' commands "$@"
}

if [ "$funcstack[1]" = "_hd_fpv_video_tool" ]; then
    _hd_fpv_video_tool "$@"
else
    compdef _hd_fpv_video_tool hd_fpv_video_tool
fi

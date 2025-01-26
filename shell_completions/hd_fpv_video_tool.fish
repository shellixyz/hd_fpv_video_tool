# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_hd_fpv_video_tool_global_optspecs
	string join \n l/log-level= h/help V/version
end

function __fish_hd_fpv_video_tool_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_hd_fpv_video_tool_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_hd_fpv_video_tool_using_subcommand
	set -l cmd (__fish_hd_fpv_video_tool_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -s l -l log-level -r -f -a "off\t''
error\t''
warn\t''
info\t''
debug\t''
trace\t''"
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -s V -l version -d 'Print version'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "display-osd-file-info" -d 'Display information about the specified OSD file'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "generate-overlay-frames" -d 'Generate a transparent overlay frame sequence as PNG files from a .osd file'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "generate-overlay-video" -d 'Generate an OSD overlay video to be displayed over another video'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "cut-video" -d 'Cut a video file without transcoding by specifying the desired start and/or end timestamp'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "fix-video-audio" -d 'Fix a DJI Air Unit video\'s audio sync and/or volume'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "transcode-video" -d 'Transcode a video file, optionally burning the OSD onto it'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "play-video-with-osd" -d 'Play a video with OSD by overlaying a transparent OSD video in real time'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "splice-videos" -d 'Splice videos files together'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "add-audio-stream" -d 'Add a silent audio stream to a video file'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "generate-shell-autocompletion-files"
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "generate-man-pages"
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand display-osd-file-info" -s h -l help -d 'Print help'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -s v -l target-video-file -d 'use the resolution from the specified video file to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode' -r -F
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -l hide-regions -d 'hide rectangular regions from the OSD' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -l hide-items -d 'hide items from the OSD  Available items (font variant: name list):   - Ardupilot: gpslat, gpslon, alt, short+code, long+code   - INAV: gpslat, gpslon, alt' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -l start -d 'start timestamp' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -l end -d 'end timestamp' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -s r -l target-resolution -d 'resolution used to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -l min-margins -d 'minimum margins to decide whether scaling should be used and how much to scale' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -l min-coverage -d 'minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -s f -l font-dir -d 'path to the directory containing font sets' -r -F
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -s i -l font-ident -d 'force using this font identifier when loading fonts, default is automatic' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -s o -l frame-shift -d 'Shift the output by that number of frames. Use this option to sync the OSD to a particular video' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -s s -l scaling -d 'force using scaling, default is automatic'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -s n -l no-scaling -d 'force disable scaling, default is automatic'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-frames" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s v -l target-video-file -d 'use the resolution from the specified video file to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode' -r -F
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -l hide-regions -d 'hide rectangular regions from the OSD' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -l hide-items -d 'hide items from the OSD  Available items (font variant: name list):   - Ardupilot: gpslat, gpslon, alt, short+code, long+code   - INAV: gpslat, gpslon, alt' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -l start -d 'start timestamp' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -l end -d 'end timestamp' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s r -l target-resolution -d 'resolution used to decide what kind of tiles (SD/HD) would best fit and also whether scaling should be used when in auto scaling mode' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -l min-margins -d 'minimum margins to decide whether scaling should be used and how much to scale' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -l min-coverage -d 'minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s f -l font-dir -d 'path to the directory containing font sets' -r -F
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s i -l font-ident -d 'force using this font identifier when loading fonts, default is automatic' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s o -l frame-shift -d 'Shift the output by that number of frames. Use this option to sync the OSD to a particular video' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s c -l codec -r -f -a "vp8\t''
vp9\t''"
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s s -l scaling -d 'force using scaling, default is automatic'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s n -l no-scaling -d 'force disable scaling, default is automatic'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s y -l overwrite -d 'overwrite output file if it exists'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-overlay-video" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand cut-video" -s s -l start -d 'start timestamp' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand cut-video" -s e -l end -d 'end timestamp' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand cut-video" -s y -l overwrite -d 'overwrite output file if it exists'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand cut-video" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand fix-video-audio" -s s -l sync -d 'fix audio sync only'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand fix-video-audio" -s v -l volume -d 'fix audio volume only'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand fix-video-audio" -s y -l overwrite -d 'overwrite output file if it exists'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand fix-video-audio" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l min-osd-margins -d 'minimum margins to decide whether scaling should be used and how much to scale' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l min-osd-coverage -d 'minimum percentage of OSD coverage under which scaling will be used if --scaling/--no-scaling options are not provided' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s d -l osd-font-dir -d 'path to the directory containing font sets' -r -F
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s i -l osd-font-ident -d 'force using this font identifier when loading fonts, default is automatic' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s o -l osd-frame-shift -d 'shift frames to sync OSD with video' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l osd-hide-regions -d 'hide rectangular regions from the OSD' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l osd-hide-items -d 'hide items from the OSD  Available items (font variant: name list):   - Ardupilot: gpslat, gpslon, alt, short+code, long+code   - INAV: gpslat, gpslon, alt' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l osd-overlay-video-codec -r -f -a "vp8\t''
vp9\t''"
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l osd-overlay-video-file -d 'path of the video file to generate' -r -F
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s F -l osd-file -d 'path to FPV.WTF .osd file to use to generate OSD frames to burn onto video' -r -F
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l video-encoder -d 'video encoder to use' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l video-bitrate -d 'video max bitrate' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l video-crf -d 'video constant quality setting' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s r -l video-resolution -d '[possible values: 720p, 720p4:3, 1080p, 1080p4:3, <width>x<height>]' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l remove-video-defects -d 'remove video defects' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l audio-encoder -d 'audio encoder to use' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l audio-bitrate -d 'max audio bitrate' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l start -d 'start timestamp' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l end -d 'end timestamp' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -l osd -d 'burn OSD onto video, try to find the OSD file automatically'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s s -l osd-scaling -d 'force using scaling, default is automatic'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s n -l no-osd-scaling -d 'force disable scaling, default is automatic'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s O -l osd-overlay-video -d 'generate OSD overlay video instead of burning it onto the video'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s a -l add-audio -d 'add audio stream to the output video'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s f -l fix-audio -d 'fix DJI AU audio: fix sync + volume'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s v -l fix-audio-volume -d 'fix DJI AU audio volume'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s u -l fix-audio-sync -d 'fix DJI AU audio sync'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s y -l overwrite -d 'overwrite output file if it exists'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand transcode-video" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand play-video-with-osd" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand splice-videos" -s y -l overwrite -d 'overwrite output file if it exists'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand splice-videos" -s h -l help -d 'Print help'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand add-audio-stream" -l audio-encoder -d 'audio encoder to use' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand add-audio-stream" -l audio-bitrate -d 'max audio bitrate' -r
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand add-audio-stream" -s y -l overwrite -d 'overwrite output file if it exists'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand add-audio-stream" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-shell-autocompletion-files" -s h -l help -d 'Print help'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand generate-man-pages" -s h -l help -d 'Print help'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "display-osd-file-info" -d 'Display information about the specified OSD file'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "generate-overlay-frames" -d 'Generate a transparent overlay frame sequence as PNG files from a .osd file'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "generate-overlay-video" -d 'Generate an OSD overlay video to be displayed over another video'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "cut-video" -d 'Cut a video file without transcoding by specifying the desired start and/or end timestamp'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "fix-video-audio" -d 'Fix a DJI Air Unit video\'s audio sync and/or volume'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "transcode-video" -d 'Transcode a video file, optionally burning the OSD onto it'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "play-video-with-osd" -d 'Play a video with OSD by overlaying a transparent OSD video in real time'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "splice-videos" -d 'Splice videos files together'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "add-audio-stream" -d 'Add a silent audio stream to a video file'
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "generate-shell-autocompletion-files"
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "generate-man-pages"
complete -c hd_fpv_video_tool -n "__fish_hd_fpv_video_tool_using_subcommand help; and not __fish_seen_subcommand_from display-osd-file-info generate-overlay-frames generate-overlay-video cut-video fix-video-audio transcode-video play-video-with-osd splice-videos add-audio-stream generate-shell-autocompletion-files generate-man-pages help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'

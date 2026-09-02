# v2.2.0

## New Features

- **Hardware-accelerated (VAAPI) encoding** for `transcode-video` and the OSD-burning path, with automatic codec selection (AV1/H.265) when supported, a `--no-hwaccel` opt-out, and correct `-vaapi_device` / `-hwaccel` argument wiring. Enabled by default via the new `hwaccel` feature.
- **Transcode profiles**: `-p/--profile` on `transcode-video` selects a built-in profile (e.g. `digital-fpv`, `analog-fpv`) or a custom one defined in the user config file, controlling per-codec bitrate/quality defaults.
- **Optional OSD**: new `--optional-osd` flag lets `transcode-video`/burn-OSD commands continue without OSD instead of failing when no associated `.osd` file is found.
- **Speed adjustment**: `--speed` option on `transcode-video` changes both video and audio playback speed (renamed/expanded from the earlier `--speedup` option, which only affected video).
- **Audio removal**: new `-R/--remove-audio` flag on `transcode-video` (including the OSD-burn path), plus a standalone `remove-audio-stream` command that strips the audio track via stream copy (no re-encode).
- **ffmpeg process priority**: `-P/--ffmpeg-priority` option to set the OS scheduling priority (niceness) of the spawned ffmpeg process.
- **Programmatic library API**: new `transcode_with_progress`/`TranscodeConfig` and `fix_dji_air_unit_audio_with_progress` functions expose transcoding with a progress callback instead of the CLI's TTY progress bar, plus proper cancellation — the ffmpeg child process is now killed when the task is aborted/dropped.
- Automatic output file extension inference based on codec/container when an explicit output path isn't given.
- Shell completion generation improvements.

## Bug Fixes

- Fixed missing `0:v` stream mapping in both the software and hardware-accelerated transcode paths when no video filter was applied.
- Fixed video codec argument being silently ignored when `--no-hwaccel` was used.
- Fixed default video quality selection for AV1.
- Fixed the `tv --osd-overlay-video` option.
- Partial/broken output files are now cleaned up automatically if a transcode or video-generation operation fails partway through.
- Various ffmpeg progress-parsing and warning fixes.

## Internal / Maintenance

- Bumped edition to Rust 2024 (MSRV raised to 1.88) and updated most dependencies (`ffmpeg-next` 7.1 → 9.0, `clap`, `tokio` +`process`/`io-util` features, `image`, `indicatif`, `thiserror`, etc.); dropped `lazy_static` in favor of `std::sync::LazyLock`.
- Enabled `clippy::pedantic` and fixed all resulting warnings; large internal refactor deduplicating the transcode command-building/audio-filter logic shared between the CLI and programmatic APIs.
- Added a git pre-commit hook running fmt/check/clippy/tests and regenerating man pages automatically.
- CI fixes and improved local CI testing support.

**Full Changelog**: https://github.com/shellixyz/hd_fpv_video_tool/compare/v2.1.0...v2.2.0

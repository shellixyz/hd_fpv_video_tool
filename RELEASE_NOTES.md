# v2.3.0

## New Features

- **msp-osd v0.12+ support**: `.osd` files in format v2, recorded by msp-osd v0.12.0 and later, are now read. They previously failed with `UnsupportedFileFormatVersion(2)` (#14).
- **PNG OSD fonts**: the PNG fonts of msp-osd v0.12.0+ and Walksnail Avatar are supported, with 1 to 4 pages of 256 glyphs. They are looked up in the font directory by msp-osd's file names (`font_btfl_hd.png`, `font_inav.png`, ...), so fonts copied from the goggles' SD card work as they are. Bin fonts are still read: the lookup tries the firmware's PNG font, then its bin font, then the generic PNG and bin fonts. The loaded font file is logged, and the error lists every file looked up when none is found.
- **Explicit font file**: `--font-file` / `--osd-font-file` load one PNG font directly and skip the font directory lookup. Glyphs are resized when their size differs from the one the overlay settings pick.
- **More firmware detected for font lookup**: Quicksilver (`quic`) is supported, and Walksnail recordings naming Betaflight, KISS Ultra or Quicksilver now pick their font.
- **Faster OSD overlay videos**: `generate-overlay-video` and `transcode-video --osd-overlay-video` now write a packed-alpha MP4 by default. Each frame stores the OSD colors and transparency side by side, so the overlay is encoded with the normal (including hardware) encoders, much faster than VP8/VP9, while keeping smooth OSD edges. `play-video-with-osd` reads these files, preferring `<name>_osd.mp4` and falling back to `<name>_osd.webm`.
- **Software fallback for oversized frames**: when the output frame is larger than the hardware encoder supports, the default codec selection skips that encoder (AV1, then H.265, then software H.265). An explicitly chosen codec falls back to its software encoder with a warning, instead of FFmpeg failing.

## Behavior Changes

- The default overlay video format changed from transparent VP8 WebM to packed-alpha MP4. Other players can't overlay packed-alpha MP4s directly. Use `--codec vp8` / `--codec vp9` (or `--osd-overlay-video-codec` with `transcode-video`) to get a transparent WebM as before.

## Bug Fixes

- Fixed glyph 256 rendering blank with bin fonts.

## Internal / Maintenance

- Requires hd_fpv_osd_font_tool 1.2.

# v2.2.1

This release only changes how the AppImage is packaged, the application itself is unchanged.

## AppImage Fixes

- **Runs on more distributions**: the AppImage is now built on Ubuntu 22.04 and runs on any distribution with glibc 2.35 or newer (Ubuntu 22.04+, Debian 12+, Fedora, ...). The previous one required glibc 2.38+.
- Fixed startup failures caused by missing shared libraries (`libXss.so.1` on Fedora, `libxml2.so.2` on Ubuntu 26.04, `libjack.so.0` on Ubuntu). The libraries which are bundled are now chosen using the official AppImage exclude list.
- **Hardware acceleration**: the bundled libva now finds the VA-API drivers on Debian/Ubuntu, Fedora/openSUSE, Arch and NixOS, previously only on Debian-based distributions.
- Bundled FFmpeg updated to 8.1. The bundled mpv is now 0.34.
- Fixed the desktop entry (name, `Exec`, `Terminal=true`) and added a real icon, so the AppImage can be listed in the [AppImage catalog](https://appimage.github.io/).

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

# Chroma-key OSD overlays

## Decisions

- Chroma-key sidecars become the default; transparent VP8/VP9 WebM remains an explicit
  compatibility mode.
- Default chroma-key sidecars use the normal `transcode-video` encoder-selection policy:
  VAAPI AV1, then VAAPI H.265, then software H.265. They use overlay-specific
  constant-quality settings and a bitrate cap.
- Sidecars are silent and preserve existing dimensions, timing, and standalone generation.
- Chroma-key output uses a configurable color with magenta (`#FF00FF`) as the default. Generation
  must fail when rendered OSD pixels collide with the selected key color.
- New files carry mode, key color, and fixed filter-tuning metadata. Playback recognizes tagged
  chroma-key files and legacy VP8/VP9 transparent files. Untagged opaque-codec files require an
  explicit playback override.
- Default sidecar discovery prefers `<video-stem>_osd.mp4`, then the legacy
  `<video-stem>_osd.webm`.
- `transcode-video --osd-overlay-video` follows the same behavior; direct `--osd` burn-in remains
  unchanged.
- Validation uses `DJIG0029_cut.mp4` and `DJIG0029.osd` through opt-in objective and visual
  checks. There is no universal hardware-speed threshold.

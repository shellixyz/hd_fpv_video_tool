# Opaque-codec OSD overlays

The directory keeps its original `chroma-key-overlays` name. Ticket 04 replaced chroma keying
with packed alpha after tickets 01 to 03 shipped chroma-key sidecars that fringed and lost glyph
outlines; see `issues/04-packed-alpha-sidecars.md` for the diagnosis.

## Decisions

- Packed-alpha MP4 sidecars are the default; transparent VP8/VP9 WebM remains an explicit
  compatibility mode.
- Each packed-alpha frame holds the overlay's straight RGB on the left and its alpha, as gray, on
  the right. The alpha plane starts at the overlay width rounded up to a multiple of 64.
- Default sidecars use the normal `transcode-video` encoder-selection policy: VAAPI AV1, then
  VAAPI H.265, then software H.265. They use overlay-specific, per-codec constant-quality
  settings and a bitrate cap.
- Sidecars are silent and preserve existing overlay dimensions, timing, and standalone generation.
- New files carry mode and layout metadata (`hd_fpv_overlay_mode=packed-alpha`,
  `hd_fpv_overlay_size`, `hd_fpv_overlay_alpha_x`). Playback crops both planes at the recorded
  coordinates, so encoder padding is never displayed. It recognizes tagged packed-alpha files and
  VP8/VP9 transparent files and rejects anything else.
- Default sidecar discovery prefers `<video-stem>_osd.mp4`, then the legacy
  `<video-stem>_osd.webm`.
- `transcode-video --osd-overlay-video` follows the same behavior; direct `--osd` burn-in remains
  unchanged.
- Validation uses `DJIG0029_cut.mp4` and `DJIG0029.osd` through opt-in objective and visual
  checks. There is no universal hardware-speed threshold.

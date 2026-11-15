# Overlay video quality validation

The ignored `overlay_video_quality` integration test validates the public
`generate-overlay-video` command against the opt-in `DJIG0029_cut.mp4` and
`DJIG0029.osd` fixtures. The fixture directory defaults to the repository root;
set `OVERLAY_VIDEO_QUALITY_FIXTURE_DIR` when they live elsewhere. Set
`OVERLAY_VIDEO_QUALITY_FONT_DIR` to the directory containing the font files used
by the recording.

Run the check with:

```sh
OVERLAY_VIDEO_QUALITY_FONT_DIR=/path/to/fonts \
cargo test --release --test overlay_video_quality -- --ignored --nocapture
```

The test independently renders the first second of source overlay frames with
`generate-overlay-frames` and creates a first-minute packed-alpha sidecar with
`generate-overlay-video`. It reads the sidecar's recorded layout, decodes and
unpacks the first second with the same filter that playback uses, and compares
every decoded frame with its source frame:

- **Composited:** each pair of pixels is composited over black and over white.
  A pixel differs when any channel differs by more than 24 levels on either
  background. No more than 4% of the visible (non-transparent) source pixels
  may differ.
- **Alpha:** no more than 0.1% of the visible source pixels may have an alpha
  difference greater than 24.
- **Alpha range:** decoded alpha must average within 4 levels of 0 over fully
  transparent source pixels and of 255 over fully opaque ones. Alpha decoded
  from limited-range luma without expansion would be off by about 16 and 20.
- **Metadata:** the sidecar must be tagged `hd_fpv_overlay_mode=packed-alpha`
  and its recorded overlay size must match the rendered frames.

The composited threshold allows for 4:2:0 chroma subsampling, which every
default encoder uses. On this fixture, a lossless 4:2:0 encode already
differs on 2.45% of visible pixels, from thin colored strokes next to black
outlines; a lossless 4:4:4 encode differs on none. Alpha travels in
full-resolution luma, so subsampling does not affect it.

The test prints the selected stream codec, encoder tag when available, bitrate,
encoded sidecar size, and encode duration. Record that output with the test
run, alongside the host, GPU, and FFmpeg version. The encoder policy may
choose VAAPI AV1, then VAAPI H.265, then software H.265, so this check has no
universal speed threshold.

## Baseline

The fixture run on 2026-11-15 (FFmpeg 8.1.2) selected `av1_vaapi` and produced:

| Codec | Bitrate | Sidecar size | Encode duration | Composited | Alpha | Alpha range (transparent / opaque) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| AV1 | 598,381 bit/s | 4,503,461 bytes | 17.23 s | 2.999% | 0.003% | 0.029 / 1.468 |

Measured on the first 30 frames with the same layout, the other encoders give
3.3% (software H.265, CRF 28) and 2.8% to 3.3% (VAAPI H.265, QP 22 to 26).

Review the generated sidecar over `DJIG0029_cut.mp4` with
`play-video-with-osd` before accepting a change:

- Glyph edges and black outlines remain smooth and readable at normal and
  magnified playback sizes.
- No glyph pixels disappear, including thin strokes and anti-aliased edges.
- No colored halos, edge lines, or bars appear around the overlay.
- The overlay is centered over the video.

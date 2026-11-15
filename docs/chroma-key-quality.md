# Chroma-key sidecar quality validation

The ignored `chroma_key_quality` integration test validates the public
`generate-overlay-video` command against the opt-in `DJIG0029_cut.mp4` and
`DJIG0029.osd` fixtures. The fixture directory defaults to the repository root;
set `CHROMA_KEY_QUALITY_FIXTURE_DIR` when they live elsewhere. Set
`CHROMA_KEY_QUALITY_FONT_DIR` to the directory containing the font files used
by the recording.

Run the check with:

```sh
CHROMA_KEY_QUALITY_FONT_DIR=/path/to/fonts \
cargo test --test chroma_key_quality -- --ignored --nocapture
```

The test independently renders the first second of source overlay frames with
`generate-overlay-frames`, creates a first-minute chroma-key sidecar with
`generate-overlay-video` using its default `#000040` key color, then decodes
and keys the first second of the sidecar with the recorded
`chromakey=0x000040:0.05:0.0` filter. It compares every decoded
frame to the source overlay, center-cropping AV1's padded coded frame to its
display dimensions. A pixel differs when any RGBA channel differs by more than
24; no more than 6.5% of all compared pixels may differ.

The test prints the selected stream codec, encoder tag when available, bitrate,
encoded sidecar size, and encode duration. Record that output with the test
run, alongside the host, GPU, and FFmpeg version. The encoder policy may
choose VAAPI AV1, then VAAPI H.265, then software H.265, so this check has no
universal speed threshold.

## Baseline

The initial fixture run on 2026-11-15 selected `av1_vaapi` and produced:

| Codec | Bitrate | Sidecar size | Encode duration |
| --- | ---: | ---: | ---: |
| AV1 | 215,721 bit/s | 1,633,504 bytes | 85.03 s |

Review the generated sidecar over `DJIG0029_cut.mp4` before accepting a change:

- Glyph edges remain readable at normal and magnified playback sizes.
- No glyph pixels disappear, including thin strokes and anti-aliased edges.
- No magenta or other colored halos remain around keyed glyphs.

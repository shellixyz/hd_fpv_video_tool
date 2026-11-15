# Chroma-key playback follow-up

The current worktree improves chroma-key sidecar playback but does not yet
reach transparent-overlay quality. `mpv-shot0004.jpg` is better than the
magenta/near-black attempts, but glyphs still have softened dark-blue edges.
The screenshots are local artifacts and intentionally untracked.

## Current state

- Default key: `#000040`; recorded filter:
  `chromakey=0x000040:0.05:0.0`.
- `Frame::apply_chroma_key` permits a key-colored alpha-1 edge pixel; the
  original magenta collision was not an opaque glyph.
- Tagged sidecars with either the current `0.05` or historical `0.01`
  similarity are accepted. Untagged opaque sidecars still use legacy magenta.
- The fixture quality test ignores RGB differences beneath fully transparent
  pixels and has a 6.5% visible-pixel threshold. It passes with VAAPI AV1.
- The regenerated local `DJIG0029_cut_transcoded_osd.mp4` uses the current
  key and filter.

## Diagnosis

AV1's YUV conversion changes `#FF00FF` to `#FE00FD`; magenta needs a wider
threshold to remove the background. That wider threshold produces conspicuous
magenta fringes. Near-black `#010203` removes black glyph detail. Dark navy
`#000040` is the least bad tested chroma key, but flattening straight-alpha
anti-aliased glyphs against any key color cannot reproduce their original
edges exactly.

## Recommended next work

Prioritize a real-alpha sidecar mode rather than more chroma-key colors:

1. Fix transparent VP9 encoding to request `yuva420p`. The current VP9 path
   fails on this FFmpeg because it attempts unsupported `gbrap`.
2. Verify alpha end-to-end in MPV, not just FFmpeg: FFmpeg reports
   `alpha_mode=1` for a manually encoded VP9 WebM but decodes it as opaque on
   this host.
3. If VP9 alpha is not reliably playable, evaluate another true-alpha codec
   and container supported by MPV, then make that the quality-preserving
   default or expose it prominently.
4. Upgrade `tests/chroma_key_quality.rs` to compare composited output against
   composited source frames. Its global differing-pixel ratio does not
   sufficiently weight concentrated glyph-edge damage.

Validation commands used:

```sh
cargo test --lib
cargo clippy --lib -- -D warnings
CHROMA_KEY_QUALITY_FONT_DIR="$HOME/.local/share/hd_fpv_video_tool/fonts" \
CHROMA_KEY_QUALITY_FIXTURE_DIR="$PWD" \
cargo test --test chroma_key_quality -- --ignored --nocapture
```

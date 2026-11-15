# 01: Generate and play chroma-key sidecars

**What to build:** `generate-overlay-video` creates a tagged, silent, `#000040`-key MP4 by default,
and `play-video-with-osd` discovers and keys it correctly while preserving legacy transparent WebM
playback. It rejects unsafe untagged opaque sidecars unless explicitly overridden.

**Blocked by:** None (can start immediately).

**Status:** done

- [x] Default to chroma-key sidecars while retaining explicit transparent VP8/VP9 compatibility.
- [x] Reuse normal transcode codec selection and no-hardware fallback, with overlay-specific
      rate-control settings.
- [x] Emit MP4 sidecars with mode/key/filter metadata and the `_osd.mp4` naming convention.
- [x] Preserve silent output, existing resolution, timing, and standalone generation behavior.
- [x] Reject key-color collisions clearly and support configurable key colors.
- [x] Discover and key tagged chroma-key sidecars, preserve legacy transparent playback, and reject
      unsafe untagged opaque-codec sidecars unless explicitly overridden.
- [x] Add focused coverage for mode selection, metadata, naming, collision handling, detection,
      fallback, rejection, and filter construction.

## Built

- Added default chroma-key MP4 generation, container metadata, configurable key colors, collision
  handling, and alpha compositing in `src/osd/overlay.rs`.
- Added MP4-first sidecar discovery, metadata validation/keying, legacy VP8/VP9 playback, and the
  explicit untagged-opaque override in `src/video.rs`.
- Regenerated the affected man pages and added focused unit coverage.
- Review fixed MP4 metadata scope and UTF-8 color parsing; it also kept transcode-sidecar parity
  deferred to ticket 02.

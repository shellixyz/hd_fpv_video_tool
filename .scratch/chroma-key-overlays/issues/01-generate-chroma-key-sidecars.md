# 01: Generate and play chroma-key sidecars

**What to build:** `generate-overlay-video` creates a tagged, silent, magenta-key MP4 by default,
and `play-video-with-osd` discovers and keys it correctly while preserving legacy transparent WebM
playback. It rejects unsafe untagged opaque sidecars unless explicitly overridden.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Default to chroma-key sidecars while retaining explicit transparent VP8/VP9 compatibility.
- [ ] Reuse normal transcode codec selection and no-hardware fallback, with overlay-specific
      rate-control settings.
- [ ] Emit MP4 sidecars with mode/key/filter metadata and the `_osd.mp4` naming convention.
- [ ] Preserve silent output, existing resolution, timing, and standalone generation behavior.
- [ ] Reject key-color collisions clearly and support configurable key colors.
- [ ] Discover and key tagged chroma-key sidecars, preserve legacy transparent playback, and reject
      unsafe untagged opaque-codec sidecars unless explicitly overridden.
- [ ] Add focused coverage for mode selection, metadata, naming, collision handling, detection,
      fallback, rejection, and filter construction.

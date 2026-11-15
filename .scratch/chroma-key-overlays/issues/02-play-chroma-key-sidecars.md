# 02: Unify transcode-sidecar generation

**What to build:** `transcode-video --osd-overlay-video` produces the same chroma-key sidecar
behavior and controls as direct generation, while direct `--osd` burn-in remains unchanged.

**Blocked by:** 01: Generate and play chroma-key sidecars.

**Status:** done

- [x] Generate the same default chroma-key sidecar when requested through `transcode-video`.
- [x] Keep direct OSD burn-in on its existing RGBA-frame path.
- [x] Preserve the explicit transparent compatibility mode and advanced encoder controls.
- [x] Add focused coverage that verifies parity with direct overlay generation.

## Built

- Commits `d618653` and `c1a165c` default transcode sidecars to chroma-key MP4, carry the codec
  and key-color controls through to direct overlay generation, and retain VP8/VP9 WebM
  compatibility.
- Review found that the initial CLI parser tests did not verify generated sidecar configuration.
  The follow-up extracts the command construction and compares it with direct generation before
  invoking FFmpeg.

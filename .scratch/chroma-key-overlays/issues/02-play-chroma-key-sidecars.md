# 02: Unify transcode-sidecar generation

**What to build:** `transcode-video --osd-overlay-video` produces the same chroma-key sidecar
behavior and controls as direct generation, while direct `--osd` burn-in remains unchanged.

**Blocked by:** 01: Generate and play chroma-key sidecars.

**Status:** ready-for-agent

- [ ] Generate the same default chroma-key sidecar when requested through `transcode-video`.
- [ ] Keep direct OSD burn-in on its existing RGBA-frame path.
- [ ] Preserve the explicit transparent compatibility mode and advanced encoder controls.
- [ ] Add focused coverage that verifies parity with direct overlay generation.

# 03: Validate chroma-key quality

**What to build:** An opt-in `DJIG0029_cut.mp4` / `DJIG0029.osd` validation path that measures
keyed-overlay fidelity, documents visual checks, and records encoder/bitrate results.

**Blocked by:** 01: Generate and play chroma-key sidecars; 02: Unify transcode-sidecar generation.

**Status:** ready-for-agent

- [ ] Define and document the opt-in fixture invocation.
- [ ] Decode, key, and compare generated sidecars against the source overlay with a defined pixel
      tolerance.
- [ ] Document visual checks for readable glyph edges, missing pixels, and colored halos.
- [ ] Record encoder selection and benchmark results without imposing a universal speed threshold.

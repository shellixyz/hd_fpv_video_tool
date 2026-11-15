# 03: Validate chroma-key quality

**What to build:** An opt-in `DJIG0029_cut.mp4` / `DJIG0029.osd` validation path that measures
keyed-overlay fidelity, documents visual checks, and records encoder/bitrate results.

**Blocked by:** 01: Generate and play chroma-key sidecars; 02: Unify transcode-sidecar generation.

**Status:** done

- [x] Define and document the opt-in fixture invocation.
- [x] Decode, key, and compare generated sidecars against the source overlay with a defined pixel
      tolerance.
- [x] Document visual checks for readable glyph edges, missing pixels, and colored halos.
- [x] Record encoder selection and benchmark results without imposing a universal speed threshold.

## Built

- Added the ignored `chroma_key_quality` integration test. It encodes the first minute of the
  `DJIG0029_cut.mp4` / `DJIG0029.osd` fixture, then compares the first second after keying to
  independently rendered overlay frames.
- Configured VAAPI overlay encodes with the required device and hardware upload filter.
- Documented the opt-in invocation, a 24-channel/6% pixel tolerance, visual review criteria, and
  the initial AV1 VAAPI benchmark in `docs/chroma-key-quality.md`.

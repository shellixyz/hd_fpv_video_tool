# 05: Read DJI OSD files in format v2

**What to build:** read the `.osd` files that msp-osd 0.12 and later records. Today
`src/osd/dji/file.rs` accepts only format version 1 (`SUPPORTED_FORMAT_VERSIONS = 1..=1`) and
rejects these files with `UnsupportedFileFormatVersion`.

**Status:** needs-triage

- [ ] Parse the v2 header: the font variant is the MSP FC variant ID as a 5-byte NUL-terminated
      string (`BTFL`, `INAV`, `ARDU`, `ULTR`, `QUIC`, ...) in place of v1's one-byte ID, so frames
      start 4 bytes later.
- [ ] Map the v2 variant string to `FontVariant` with the same mapping as the Walksnail header
      (`src/osd/wsa/file.rs`, `FileHeaderRaw::font_variant`), shared rather than duplicated.
- [ ] Keep reading v1 files.

## Comments

Found while triaging ticket 04 (`.scratch/png-fonts/issues/04-detect-more-firmware-variants.md`).
Sources: fpv-wtf/msp-osd at `ef9d906cf43ac939799ef04287b19bfbd1bd968f`, `jni/rec/rec_pb.c` (reads
both versions; v1 IDs map to the strings above), and fpv-wtf/wtfos-configurator
`src/osd-overlay/osd.ts` (`OsdReader`, same layout). Open question for triage: which version
number v2 files carry, from a sample recording or `REC_VERSION` in msp-osd's `jni/rec/rec.h`.

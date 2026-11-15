# 05: Read DJI OSD files in format v2

**What to build:** read the `.osd` files that msp-osd 0.12 and later records. Today
`src/osd/dji/file.rs` accepts only format version 1 (`SUPPORTED_FORMAT_VERSIONS = 1..=1`) and
rejects these files with `UnsupportedFileFormatVersion`.

**Status:** done

- [x] Parse the v2 header: the font variant is the MSP FC variant ID as a 5-byte NUL-terminated
      string (`BTFL`, `INAV`, `ARDU`, `ULTR`, `QUIC`, ...) in place of v1's one-byte ID, so frames
      start 4 bytes later.
- [x] Map the v2 variant string to `FontVariant` with the same mapping as the Walksnail header
      (`src/osd/wsa/file.rs`, `FileHeaderRaw::font_variant`), shared rather than duplicated.
- [x] Keep reading v1 files.

## Built

- `src/osd/dji/file.rs`: `FileHeaderRaw` holds the fields every version shares.
  `FontVariantField` (`NumericId` in v1, `MspFcVariantId` in v2) is chosen from the format version,
  gives the field's length and parses it; other versions are rejected. `Reader` records where the
  first frame starts, so `rewind` works for both header lengths.
- `FileHeader` stores the resolved `FontVariant` and the recorded ID as a string (`"4"` in v1,
  `"QUIC"` in v2), which `display-osd-file-info` prints as before. The v2 ID is read up to the
  first NUL or the 5 bytes; an empty ID is the generic font, as msp-osd records it.
- `FontVariant::from_msp_fc_variant_id` (`src/osd/font_variant.rs`) is the shared string mapping;
  the Walksnail header calls it.
- README intro says both formats are read.
- Tests (`dji/file.rs`) write `.osd` files with the `rec.h` layout and open them with
  `Reader::open`: v1 ID and frames, v1 ID 5, v2 ID and frames after the longer header, a 5-byte
  v2 ID without NUL, an empty v2 ID, and version 3 rejected. No real v2 recording was available.

Review fixes: the v1/v2 parse was picked from the variant field's length, separately from the
version-to-length rule; `FontVariantField` now makes both decisions from the format version. Left
as judgement calls: `FileHeader` keeps `font_variant_id: String` and `font_variant` as two fields;
the Walksnail ID is not cut at a NUL as the DJI v2 ID is; an empty Walksnail ID stays `Unknown`
while an empty DJI v2 ID is `Generic`; `display-osd-file-info` prints an empty v2 ID as
`OSD Font variant:  (Generic)`; CONTEXT.md does not define "font variant" or "MSP FC variant ID".

## Comments

Found while triaging ticket 04 (`.scratch/png-fonts/issues/04-detect-more-firmware-variants.md`).
Sources: fpv-wtf/msp-osd at `ef9d906cf43ac939799ef04287b19bfbd1bd968f`, `jni/rec/rec_pb.c` (reads
both versions; v1 IDs map to the strings above), and fpv-wtf/wtfos-configurator
`src/osd-overlay/osd.ts` (`OsdReader`, same layout). Open question for triage: which version
number v2 files carry, from a sample recording or `REC_VERSION` in msp-osd's `jni/rec/rec.h`.

Version number, settled 2026-11-16: v2 files carry format version 2. GitHub issue #14 (from #13)
reports `UnsupportedFileFormatVersion(2)` on a msp-osd 0.12.2 recording, and `jni/rec/rec.h` at
the commit above defines `REC_VERSION 2`. The sample file linked from #13 is gone, so no recording
is available. Layout from `rec.h` (packed, little-endian): `magic[7]` = `"MSPOSD\0"`,
`u16 version`, then `char_width`, `char_height`, `font_width`, `font_height` (`u8` each),
`x_offset`, `y_offset` (`u16` each), then the variant: `u8` in v1, `char[5]` in v2. Frames follow
the header unchanged (`u32 frame_idx`, `u32 size`, tile data). msp-osd prints the v2 variant with
`%.5s`, so a 5-character ID can fill the field with no NUL; read up to the first NUL or 5 bytes.
`rec_pb.c` writes an empty string for an unknown v1 ID.

Triage, agreed with the user (2026-11-16): test with synthetic files built from the `rec.h`
layout; share the mapping as `FontVariant::from_msp_fc_variant_id`; `FileHeader` holds the
resolved variant. While building: the raw ID is kept as a string for `display-osd-file-info`, and
an empty v2 ID maps to `Generic` rather than `Unknown`, because msp-osd writes an empty ID for
generic (`rec_pb.c` turns v1 ID 0 into `""`).

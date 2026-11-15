# 04: Detect more firmware variants for font lookup

**What to build:** pick the firmware's font automatically for more recordings. Today `quic` is
used only through `--font-ident quic`, and Walksnail recordings name a font variant only for
`INAV` and `ARDU` (`src/osd/wsa/file.rs`, `FileHeader::font_variant`), so Betaflight, KISS Ultra
and Quicksilver Walksnail recordings fall back to the generic font.

**Status:** done

- [x] Map the Walksnail header identifiers `BTFL`, `ULTR` and `QUIC` to Betaflight, KISS Ultra and
      Quicksilver, and DJI variant ID 5 to Quicksilver.
- [x] Add a Quicksilver `FontVariant` with PNG ident `quic` if recordings identify it.
- [x] Check that `FontVariant::osd_item_names` and the hide-items code accept the new mappings.

## Built

- `FontVariant::Quicksilver` (`src/osd/font_variant.rs`) has PNG ident `quic`, no bin font set and
  no OSD items (`src/osd/item.rs`). DJI variant ID 5 (`src/osd/dji/file.rs`) and the Walksnail IDs
  `BTFL`, `ULTR` and `QUIC` (`src/osd/wsa/file.rs`) select their variants. A doc comment on the
  Walksnail mapping says the new IDs are inferred.
- Defect found by the Quicksilver lookup test: `load_with_generic_fallback` in
  `src/osd/font_dir.rs` ran the variant step only when the variant had a bin ident, so a variant
  with PNG idents and no bin font never looked up its PNG font. `LookupStep` is now an enum
  (`Named`, `Generic`), and a named step without a bin ident looks up only its PNG fonts.
- The fallback warning names the variant's first PNG ident, so Betaflight now logs `btfl` and
  KISS Ultra logs `ultr`, where they logged `bf` and `ultra` before.
- Hide-items: Quicksilver has no items, like Betaflight and KISS Ultra. `--hide-items` help skips
  it, and `erase_osd_item` reports `UnknownOSDItem`.
- README "OSD fonts" lists `quic` and no longer limits Walksnail detection to INAV and ArduPilot.
- Tests: Walksnail ID table and unknown ID (`wsa/file.rs`), DJI ID 5 (`dji/file.rs`), Quicksilver
  lookup of `font_quic.png` then the generic font (`font_dir.rs`), Quicksilver item names
  (`item.rs`).

Gaps found along the way: DJI format v2 recordings are rejected; opened
`.scratch/png-fonts/issues/05-read-dji-format-v2.md`.

Review fixes: `load_step` no longer builds an `Option<Option<&str>>`; each step kind calls the PNG
and bin loaders directly. `LookupStep::is_empty` is now `has_candidates`. The Walksnail doc comment
sits before `#[must_use]`. Left as judgement calls: the three per-variant property matches
(`font_set_ident`, `png_font_idents`, `osd_items_location_data`) that each new variant touches, and
`None` still standing for the generic font in `png_font_file_name` and `load_bin`.

## Comments

Found while implementing ticket 02 (`.scratch/png-fonts/issues/02-discover-png-fonts.md`): the
ticket lists `quic` among msp-osd's names, but `FontVariant` has no Quicksilver variant, and the
spec review pointed out that no recording selects it.

Triage, agreed with the user (2026-11-15):

- Sources: fpv-wtf/msp-osd at `ef9d906cf43ac939799ef04287b19bfbd1bd968f`. In DJI format v1,
  `font_variant_e` puts `FONT_VARIANT_QUICKSILVER` at ID 5 (`jni/font/font.h`), and
  `jni/rec/rec_pb.c` maps IDs 1-5 to `BTFL`, `INAV`, `ARDU`, `ULTR`, `QUIC`. `jni/font/font.c`
  looks up `font_<lowercased id>.png`, so Quicksilver's PNG ident is `quic`.
- Walksnail: no primary source, because the firmware is closed. The known IDs `INAV` and `ARDU` match the
  MSP FC variant strings, so `BTFL`, `ULTR` and `QUIC` are inferred by that convention. A code
  comment says so.
- Quicksilver: `font_set_ident` is `None` (generic `.bin` fallback), `osd_items_location_data` is
  empty (like KISS Ultra), `png_font_idents` is `["quic"]`.
- Out of scope, separate ticket: DJI format v2 (msp-osd >= 0.12) stores the variant as a 5-byte
  string, and our reader rejects that format (`SUPPORTED_FORMAT_VERSIONS = 1..=1`).
- Out of scope: looking up `font_<raw id>.png` for unrecognised IDs, as msp-osd does.
  `FontVariant` stays a closed enum.
- Seams under test: `wsa::file::FileHeaderRaw::font_variant` (`BTFL`/`ULTR`/`QUIC`; unknown stays
  `Unknown`), `dji::file::FileHeader::font_variant` (5 -> Quicksilver), a `font_dir` lookup test
  that Quicksilver tries `font_quic.png` then the generic font, and `osd_item_names` including
  Quicksilver with no items.

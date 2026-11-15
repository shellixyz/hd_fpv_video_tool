# 04: Detect more firmware variants for font lookup

**What to build:** pick the firmware's font automatically for more recordings. Today `quic` is
used only through `--font-ident quic`, and Walksnail recordings name a font variant only for
`INAV` and `ARDU` (`src/osd/wsa/file.rs`, `FileHeader::font_variant`), so Betaflight, KISS Ultra
and Quicksilver Walksnail recordings fall back to the generic font.

**Status:** needs-triage

- [ ] Find the firmware identifiers Walksnail writes to the OSD file header for Betaflight, KISS
      Ultra and Quicksilver, and whether DJI recordings have a Quicksilver variant ID.
- [ ] Add a Quicksilver `FontVariant` with PNG ident `quic` if recordings identify it.
- [ ] Check that `FontVariant::osd_item_names` and the hide-items code accept the new mappings.

## Comments

Found while implementing ticket 02 (`.scratch/png-fonts/issues/02-discover-png-fonts.md`): the
ticket lists `quic` among msp-osd's names, but `FontVariant` has no Quicksilver variant, and the
spec review pointed out that no recording selects it.

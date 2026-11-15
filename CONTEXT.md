# Context

## OSD fonts

- **Tile kind**: the glyph size of a font. `SD` is 36×54 pixels and `HD` is 24×36 pixels
  (`hd_fpv_osd_font_tool::osd::tile::Kind`). Walksnail names the same sizes `_36` (1080p) and
  `_24` (720p).
- **Font page**: a block of 256 glyphs. A font has 1 to 4 pages; Betaflight uses pages 1 to 3 for
  coloured glyphs.
- **Glyph index**: the number an OSD recording stores per cell: page × 256 + glyph within the
  page, so 0 to 1023.
- **Bin font**: headerless raw RGBA glyphs, one page per file. Pages 0 and 1 are
  `font[_<ident>][_hd].bin` and `font[_<ident>][_hd]_2.bin`. msp-osd stopped reading them in
  v0.12.0.
- **PNG font**: one RGBA PNG 256 glyphs tall with one column per font page, named
  `font[_<ident>][_hd].png` by msp-osd. The crate calls it an Avatar file (`avatar_file`); the
  original Walksnail Avatar font is the one-page case.
- **Font ident**: the firmware name part of a font file name. Bin fonts here use `bf`, `inav`,
  `ardu` and `ultra` (`FontVariant::font_set_ident`). msp-osd PNG fonts use the first four
  lowercase letters of the flight controller identifier (`btfl`, `inav`, `ardu`, `ultr`, `quic`)
  and accept `bf` and `ultra` as legacy aliases.
- **Font dir**: the directory fonts are discovered in (`--font-dir`, `--osd-font-dir`,
  `DJI_OSD_FONTS_DIR`, default `~/.local/share/hd_fpv_video_tool/fonts`).

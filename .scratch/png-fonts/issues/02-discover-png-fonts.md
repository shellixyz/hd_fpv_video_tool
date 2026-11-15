# 02: Discover and render PNG fonts from the font dir

**What to build:** overlay generation and transcoding find PNG fonts in the font dir by msp-osd's
names, prefer them over bin fonts at each lookup step, and render DJI and WSA recordings with up
to 1024 glyphs. See `.scratch/png-fonts/spec.md`.

**Blocked by:** 01: Read and write multi-page PNG fonts in hd_fpv_osd_font_tool. 01 must also be
pushed and tagged `v1.2.0` by the maintainer.

**Status:** done

- [x] Update the dependency with `cargo update -p hd_fpv_osd_font_tool` to the 1.2.0 release.
- [x] `FontDir` looks up `font_<v>[_hd].png` using msp-osd's variant names (`btfl`, `inav`,
      `ardu`, `ultr`, `quic`), then the aliases `bf` and `ultra`, then generic `font[_hd].png`.
      A forced `--font-ident` is used as given.
- [x] At each step, PNG before bin: ident PNG, ident bin, generic PNG, generic bin. The
      variant-then-generic fallback is otherwise unchanged, and bin-only font dirs render as
      before.
- [x] Log the font file actually loaded at info level.
- [x] Glyph indices the font lacks keep the existing warning and blank cell.
- [x] Tests cover lookup order, alias fallback, PNG-over-bin precedence, and loading a 4-page font.
- [x] Rewrite the README "OSD fonts" section: PNG (msp-osd v0.12+ and Walksnail) and bin fonts,
      the file names looked up, and the precedence.

## Built

- `Cargo.lock` takes `hd_fpv_osd_font_tool` 1.2.0 from GitHub (`1e7c331`).
- `FontDir` in `src/osd/font_dir.rs` looks up fonts in steps. The variant or forced-ident step
  tries its PNG fonts, then its bin font. The generic step tries `font[_hd].png`, then the generic
  bin font. `FontVariant::png_font_idents` in `src/osd/font_variant.rs` gives msp-osd's idents,
  then the legacy aliases. A forced ident is used as given, and an empty one loads the generic
  fonts only.
- The new `FontLoadError` replaces `bin_file::LoadError` in `DrawFrameOverlayError::FontLoadError`.
  `NotFound` lists every file name looked up. With the user's approval, a PNG font whose tile
  kind does not match its name stops the lookup with `PngTileKindMismatch`, as a bin font of the
  wrong size already did.
- The loaded font file, or bin pair, is logged at info level with its full path.
- The public `FontDir::load` is removed; nothing called it from outside.
- README "OSD fonts" is rewritten.
- 11 tests in `font_dir.rs` use temporary font dirs whose tiles carry a marker pixel per file.

Gaps found along the way:

- Fixed with the user's approval: a bin font dir loaded only the base part when the highest glyph
  index was 256, so glyph 256 rendered blank. The extended part now loads from glyph index 256 up
  (test `glyph_index_256_loads_the_extended_bin_part`).
- Not fixed here: `quic` is never discovered automatically, and WSA recordings map only `INAV`
  and `ARDU` to a font variant. Opened `.scratch/png-fonts/issues/04-detect-more-firmware-variants.md`.

Review fixes: the tile kind error names tile kinds, `load_bin` branches once on the parts it
loads, the bin log prints full paths, a dead warning branch is removed, and a test is renamed so
it does not refer to old behaviour. Left as judgement calls: bundling `tile_kind`, the highest
glyph index and the looked-up list into a lookup context, and replacing `Option<&str>` idents
(where `None` is the generic font) with a type. The parameter name `highest_used_tile_index`
predates this ticket and was kept, although the glossary term is glyph index.

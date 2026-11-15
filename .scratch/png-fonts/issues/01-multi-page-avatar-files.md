# 01: Read and write multi-page PNG fonts in hd_fpv_osd_font_tool

**What to build:** `hd_fpv_osd_font_tool` reads and writes PNG fonts of 1 to 4 font pages, so
msp-osd's bundled `font_btfl.png` loads and `convert` produces usable Betaflight and INAV PNG
fonts. See `.scratch/png-fonts/spec.md`.

**Repository:** `../hd_fpv_osd_font_tool` (its own pre-commit hook applies).

**Blocked by:** none.

**Status:** ready-for-agent

- [ ] `avatar_file::load` accepts images 256 tiles tall and 1 to 4 tiles wide, for both tile
      kinds, and returns tiles in glyph-index order (page × 256 + row).
- [ ] `load` rejects widths above 4 tiles, widths that are not a whole number of tiles, and
      heights other than 256 tiles, with errors naming the file and its dimensions.
- [ ] `avatar_file::save` writes ⌈tiles/256⌉ columns, at most 4, instead of truncating to 256.
      A partial last page is padded with blank tiles. More than 1024 tiles is an error or a
      warning that names the dropped count, matching the existing error style.
- [ ] Output for 256 tiles or fewer is byte-identical to 1.1.1.
- [ ] Update the `convert` help text for `avatar:` to describe the multi-page layout.
- [ ] Tests on generated images: round trip of 1, 2 and 4 pages for SD and HD, glyph ordering,
      the rejected dimensions, and that a 512-tile `djibin` base + extended pair converts to a
      2-column PNG.
- [ ] Bump the version to 1.2.0 and commit locally. Do not push or tag; the maintainer does that.

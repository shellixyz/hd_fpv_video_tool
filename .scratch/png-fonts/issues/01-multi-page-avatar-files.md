# 01: Read and write multi-page PNG fonts in hd_fpv_osd_font_tool

**What to build:** `hd_fpv_osd_font_tool` reads and writes PNG fonts of 1 to 4 font pages, so
msp-osd's bundled `font_btfl.png` loads and `convert` produces usable Betaflight and INAV PNG
fonts. See `.scratch/png-fonts/spec.md`.

**Repository:** `../hd_fpv_osd_font_tool` (its own pre-commit hook applies).

**Blocked by:** none.

**Status:** done

- [x] `avatar_file::load` accepts images 256 tiles tall and 1 to 4 tiles wide, for both tile
      kinds, and returns tiles in glyph-index order (page × 256 + row).
- [x] `load` rejects widths above 4 tiles, widths that are not a whole number of tiles, and
      heights other than 256 tiles, with errors naming the file and its dimensions.
- [x] `avatar_file::save` writes ⌈tiles/256⌉ columns, at most 4, instead of truncating to 256.
      A partial last page is padded with blank tiles. More than 1024 tiles is an error or a
      warning that names the dropped count, matching the existing error style.
- [x] Output for 256 tiles or fewer is byte-identical to 1.1.1.
- [x] Update the `convert` help text for `avatar:` to describe the multi-page layout.
- [x] Tests on generated images: round trip of 1, 2 and 4 pages for SD and HD, glyph ordering,
      the rejected dimensions, and that a 512-tile `djibin` base + extended pair converts to a
      2-column PNG.
- [x] Bump the version to 1.2.0 and commit locally. Do not push or tag; the maintainer does that.

## Built

- `avatar_file` in `../hd_fpv_osd_font_tool/src/osd/avatar_file.rs` finds the tile kind and page
  count from the image dimensions. `load` returns `page × 256 + row` order. `save` writes
  ⌈tiles/256⌉ pages, pads the last one, and warns with the dropped count above 1024 tiles. New
  public items: `MAX_PAGE_COUNT`, `MAX_TILE_COUNT`, and
  `TileKind::avatar_image_dimensions_for_pages`. `TILE_COUNT` keeps its name and now means tiles
  per page.
- A dimensions error names the file, its `WxH`, and the expected layout.
- Byte-identity with 1.1.1 is pinned by SHA-256 of the bundled base fonts' output, taken from a
  1.1.1 build. The hashes depend on the `image` crate's PNG encoder.
- Verified on msp-osd's `font_btfl.png`, `font_btfl_hd.png` (4 pages), `font_inav.png`, and
  `font_inav_hd.png` (2 pages). Each loads, and an `avatar:` → `avatar:` round trip gives
  identical tiles.

Gaps found along the way, fixed here with the user's approval (all in commit `a30a0b0`):

- `convert` could not read a base + extended bin pair, so it could not turn a Betaflight or INAV
  bin font into one PNG. `djibin:path:ext_path` now loads or writes the pair. Writing a pair from
  256 tiles or fewer is an error.
- `convert` capped `tiledir:` and `symdir:` sources at 512 tiles. The cap is now
  `avatar_file::MAX_TILE_COUNT` (1024).
- `save` rejected collections under 256 tiles. It now pads them to one page, and only an empty
  collection is an error. `SaveError::WrongCollectionSize` is removed; nothing downstream uses it.

Fixed after the ticket, at the user's request: the font tool's pre-commit hook regenerated man
pages, then popped a stash that still held the staged copies. When the help text had changed, the
pop conflicted, left conflict markers in `man_pages/`, and kept the stash. The font tool now uses
this repository's hook suite, with `Checked-Tree` trailers and a `just install-hooks` recipe. Its
`pre-commit` drops unstaged changes under `man_pages/` instead of restoring them. Porting the
suite exposed a defect in this repository's `pre-commit`: it deleted the patch of unstaged changes
right after writing it, then checked those paths out, so unstaged edits were lost on every
commit. Both copies now keep the patch and refuse the commit if it cannot be written.

Review fixes: the error messages interpolate the layout constants, and the rejected-dimensions
test has one error match. Left as judgement calls: renaming `TILE_COUNT` to `PAGE_TILE_COUNT` (an
API change), and bundling kind and page count into a layout type instead of a tuple.

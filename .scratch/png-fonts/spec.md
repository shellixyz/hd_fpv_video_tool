# PNG OSD fonts

Support the PNG font format of msp-osd v0.12.0 and later, which is also used by Walksnail Avatar,
alongside the existing bin fonts. Terms are defined in `CONTEXT.md`.

## Format

- An RGBA PNG (8-bit, straight alpha) exactly 256 glyphs tall and 1 to 4 glyphs wide.
- Each column is a font page. Glyph index = page × 256 + row.
- Tile sizes are the existing tile kinds: 36×54 (`tile::Kind::SD`, msp-osd SD, Walksnail `_36`)
  and 24×36 (`tile::Kind::HD`, msp-osd `_hd`, Walksnail `_24`).
- The old bin layout maps directly: `font.bin` is page 0 and `font_2.bin` is page 1.
- Sources: msp-osd `fpv-wtf/msp-osd` tag v0.12.4, `jni/font/font.c` (layout checks at lines 81-93,
  page copy at 114-127, file lookup at 18-39 and 146-215) and `jni/osd_dji_overlay_udp.c:161-163`
  (page decoding).

## Starting point

- `hd_fpv_osd_font_tool` 1.1.1 `avatar_file` reads and writes a single column of 256 glyphs only.
  `load` rejects wider images; `save` keeps the first 256 tiles and drops the rest with a warning.
  Single-page fonts (ArduPilot) already convert correctly; multi-page fonts (Betaflight 4 pages,
  INAV, Quic and KISS Ultra 2 pages) do not.
- This tool loads bin fonts only (`src/osd/font_dir.rs`). The overlay stores glyphs in a `Vec`
  and already skips, with a warning, glyph indices the font lacks (`src/osd/overlay.rs`), so up to
  1024 glyphs need no rendering change.

## Decisions

- PNG fonts are used for overlay generation and transcoding, for both DJI and WSA recordings.
  Converting fonts stays in `hd_fpv_osd_font_tool`.
- The format code lives in `hd_fpv_osd_font_tool`: `avatar_file::load` accepts 1 to 4 columns and
  rejects more; `avatar_file::save` writes ⌈tiles/256⌉ columns (at most 4) instead of truncating.
  Output for 256 tiles or fewer stays byte-identical. The `avatar:` collection spec covers both.
  Released as 1.2.0.
- Only the side-by-side layout is read. The stacked-page INAV Walksnail layout and the 257-row
  Caddx `user_bf_36.png` quirk are rejected.
- Bin fonts keep working unchanged.
- Font-dir discovery follows msp-osd's names: `font_<v>[_hd].png` with
  `btfl`/`inav`/`ardu`/`ultr`/`quic`, then the legacy aliases `bf` (for `btfl`) and `ultra` (for
  `ultr`), then generic `font[_hd].png`. A forced `--font-ident` is used as given. Walksnail's
  arbitrary names (`user_bf_24.png`, `userfont/`) are not discovered.
- At each lookup step a PNG font beats a bin font: ident PNG, ident bin, generic PNG, generic bin.
  The existing variant-then-generic fallback is otherwise unchanged. The loaded file is logged at
  info level.
- Glyph indices the font lacks keep the current behaviour: warn and leave the cell blank.
  msp-osd's page-0 fallback is a possible later ticket.
- `--font-file` (commands with `FontOptions`) and `--osd-font-file` (commands with
  `OSDFontOptions`) load one explicit PNG font. They conflict with `--font-ident` and skip
  discovery. The tile kind comes from the image dimensions; when it differs from the kind the
  overlay settings pick, the existing tile resizing scales it and an info message is logged.
- No ADR: the precedence is easy to reverse and is recorded here.

## Delivery

- Ticket 01 is implemented in `../hd_fpv_osd_font_tool` and ends with local commits that pass that
  repo's pre-commit hook. The maintainer pushes and tags `v1.2.0`.
- Ticket 02 starts with `cargo update -p hd_fpv_osd_font_tool` against the pushed release. No
  temporary `[patch]` path dependency is committed.

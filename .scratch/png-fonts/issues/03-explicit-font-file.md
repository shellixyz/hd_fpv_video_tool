# 03: Load an explicit PNG font file

**What to build:** `--font-file <png>` on commands with `FontOptions` and `--osd-font-file <png>`
on commands with `OSDFontOptions` load one PNG font directly, for fonts whose names discovery does
not recognise, such as Walksnail's `user_bf_24.png`. See `.scratch/png-fonts/spec.md`.

**Blocked by:** 02: Discover and render PNG fonts from the font dir.

**Status:** done

- [x] The options conflict with `--font-ident` / `--osd-font-ident` and skip font-dir discovery.
- [x] PNG only. The tile kind comes from the image dimensions.
- [x] When the file's tile kind differs from the one the overlay settings pick, the existing tile
      resizing scales the tiles and an info message says so.
- [x] Tests cover argument conflicts, a matching tile kind, and a mismatched tile kind.
- [x] Document both options in the README "OSD fonts" section.

## Built

- `--font-file` (`FontOptions`) and `--osd-font-file` (`OSDFontOptions`) in
  `src/cli/font_options.rs` conflict with the ident and font-dir options. `font_source` returns the
  explicit file before resolving the font dir.
- `FontSource` in `src/osd/font_dir.rs` is a font dir with a `FontIdent` (`FromVariant`, `Generic`,
  `Forced`), or an explicit file. `FontSource::load` rejects a file without a `.png` extension
  with `FontLoadError::NotPng`; otherwise the tiles keep the image's tile kind. `load_png` is shared
  with discovery, which still checks the tile kind.
- `Generator::new` takes a `FontSource`. `tile_images` in `src/osd/overlay.rs` resizes tiles of
  another tile kind to the settings' tile kind, or to the tile scaling when the settings scale, and
  logs an info message whenever the kinds differ.
- README "OSD fonts" documents both options.
- Tests: four in `font_options.rs` (conflicts, file alone, font dir not resolved with a file,
  ident mapping), three in `font_dir.rs` (file keeps its tile kind, `FontIdent` lookup, PNG only),
  three in `overlay.rs` (matching kind, mismatched kind, tile scaling).

Gaps found along the way: none needing a ticket.

Review fixes: a non-PNG image with font dimensions loaded, because the image decoder picks the
format from the extension; now rejected. The tile kind message is also logged when the settings
scale tiles. The PNG loader is shared, `font_source` takes `&Path`, and `font_source` and
`first_image_dimensions` are renamed. Approved by the user: `FontIdent` replaces
`Option<Option<&str>>` in `FontSource::Dir`. Left as judgement calls: the parallel
`FontOptions` and `OSDFontOptions` structs, `tile_images` reading the loaded tile kind from the
first tile instead of `load` returning it, and the README's "font directory" wording for the
glossary's font dir.

## Comments

Agreed with the user before implementation:

- The options also conflict with `--font-dir` / `--osd-font-dir`, so a font dir is never silently
  ignored. The font dir is not resolved with an explicit file, so it need not exist.
- Seams under test: clap parsing of `FontOptions` and `OSDFontOptions` (argument conflicts); a new
  `FontSource::load` in `src/osd/font_dir.rs` (font dir with optional ident, or explicit file)
  that returns an explicit file's tiles whatever tile kind is requested; and a tile-images function
  in `src/osd/overlay.rs` that resizes tiles to the settings' tile kind when the loaded kind
  differs. `Generator::new` takes a `FontSource` instead of a font dir and ident.

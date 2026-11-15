# 03: Load an explicit PNG font file

**What to build:** `--font-file <png>` on commands with `FontOptions` and `--osd-font-file <png>`
on commands with `OSDFontOptions` load one PNG font directly, for fonts whose names discovery does
not recognise, such as Walksnail's `user_bf_24.png`. See `.scratch/png-fonts/spec.md`.

**Blocked by:** 02: Discover and render PNG fonts from the font dir.

**Status:** ready-for-agent

- [ ] The options conflict with `--font-ident` / `--osd-font-ident` and skip font-dir discovery.
- [ ] PNG only. The tile kind comes from the image dimensions.
- [ ] When the file's tile kind differs from the one the overlay settings pick, the existing tile
      resizing scales the tiles and an info message says so.
- [ ] Tests cover argument conflicts, a matching tile kind, and a mismatched tile kind.
- [ ] Document both options in the README "OSD fonts" section.

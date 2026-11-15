# 02: Discover and render PNG fonts from the font dir

**What to build:** overlay generation and transcoding find PNG fonts in the font dir by msp-osd's
names, prefer them over bin fonts at each lookup step, and render DJI and WSA recordings with up
to 1024 glyphs. See `.scratch/png-fonts/spec.md`.

**Blocked by:** 01: Read and write multi-page PNG fonts in hd_fpv_osd_font_tool. 01 must also be
pushed and tagged `v1.2.0` by the maintainer.

**Status:** ready-for-agent

- [ ] Update the dependency with `cargo update -p hd_fpv_osd_font_tool` to the 1.2.0 release.
- [ ] `FontDir` looks up `font_<v>[_hd].png` using msp-osd's variant names (`btfl`, `inav`,
      `ardu`, `ultr`, `quic`), then the aliases `bf` and `ultra`, then generic `font[_hd].png`.
      A forced `--font-ident` is used as given.
- [ ] At each step, PNG before bin: ident PNG, ident bin, generic PNG, generic bin. The
      variant-then-generic fallback is otherwise unchanged, and bin-only font dirs render as
      before.
- [ ] Log the font file actually loaded at info level.
- [ ] Glyph indices the font lacks keep the existing warning and blank cell.
- [ ] Tests cover lookup order, alias fallback, PNG-over-bin precedence, and loading a 4-page font.
- [ ] Rewrite the README "OSD fonts" section: PNG (msp-osd v0.12+ and Walksnail) and bin fonts,
      the file names looked up, and the precedence.

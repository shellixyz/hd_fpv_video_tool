# PNG fonts: font-dir discovery

Replaces: `.scratch/handoffs/2026-11-15-png-fonts-release.md`.

Ticket 02 is done: see `.scratch/png-fonts/issues/02-discover-png-fonts.md` (Built) and the commit
that closes it. `hd_fpv_osd_font_tool` 1.2.0 is on GitHub and in `Cargo.lock`.

Next: `.scratch/png-fonts/issues/03-explicit-font-file.md`. It builds on `FontDir` and
`FontLoadError` in `src/osd/font_dir.rs`; `avatar_file::load` already returns the tiles' kind.

Still open:

- `.scratch/png-fonts/issues/04-detect-more-firmware-variants.md` (needs-triage): `quic` and the
  Walksnail Betaflight, KISS Ultra and Quicksilver identifiers.
- From `.scratch/handoffs/2026-11-15-packed-alpha-sidecars.md`: a candidate ticket to fall back to
  software encoding when a packed-alpha frame exceeds the hardware encoder's limits.

Suggested skills: `tdd`, `rust-best-practices`, `code-review`.

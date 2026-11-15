# PNG fonts: explicit font file

Replaces: `.scratch/handoffs/2026-11-15-png-fonts-discovery.md`.

Ticket 03 is done: see `.scratch/png-fonts/issues/03-explicit-font-file.md` (Built) and the commit
that closes it. Ticket 02 closed in `6de3127`.

Next: `.scratch/png-fonts/issues/04-detect-more-firmware-variants.md`. Its status is
`needs-triage`, so settle its open questions with the user first. Font lookup goes through
`FontSource` and `FontIdent` in `src/osd/font_dir.rs`; variant idents come from
`FontVariant::png_font_idents` in `src/osd/font_variant.rs`.

Still open:

- From `.scratch/handoffs/2026-11-15-packed-alpha-sidecars.md`: a candidate ticket to fall back to
  software encoding when a packed-alpha frame exceeds the hardware encoder's limits.

Format with `cargo +nightly fmt`: the tracked `.githooks/config` sets `FMT_TOOLCHAIN=nightly`,
overriding the pinned toolchain in `.githooks/pre-commit`. The pinned one reformats
`src/cli/transcode_video_args.rs` and `src/video/timestamp.rs`.

Suggested skills: `tdd`, `rust-best-practices`, `code-review`.

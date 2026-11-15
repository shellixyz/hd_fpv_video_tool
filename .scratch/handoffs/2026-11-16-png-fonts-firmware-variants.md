# PNG fonts: more firmware variants

Replaces: `.scratch/handoffs/2026-11-15-png-fonts-explicit-file.md`.

Ticket 04 is done: see `.scratch/png-fonts/issues/04-detect-more-firmware-variants.md` (Built) and
the commit that closes it. Ticket 03 closed in `04446c5`.

Next: `.scratch/png-fonts/issues/05-read-dji-format-v2.md`. Its status is `needs-triage`, so
settle its open questions with the user first. The DJI reader is `src/osd/dji/file.rs`; the
Walksnail variant-string mapping to share is `FileHeaderRaw::font_variant` in
`src/osd/wsa/file.rs`.

Still open:

- From `.scratch/handoffs/2026-11-15-packed-alpha-sidecars.md`: a candidate ticket to fall back to
  software encoding when a packed-alpha frame exceeds the hardware encoder's limits.

Format with `cargo +nightly fmt`: the tracked `.githooks/config` sets `FMT_TOOLCHAIN=nightly`,
overriding the pinned toolchain in `.githooks/pre-commit`.

Suggested skills: `tdd`, `rust-best-practices`, `code-review`.

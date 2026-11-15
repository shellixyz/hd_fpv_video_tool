# PNG fonts: font tool release

Replaces: `.scratch/handoffs/2026-11-15-png-fonts.md`.

Ticket 01 is done: see `.scratch/png-fonts/issues/01-multi-page-avatar-files.md` and commit
`a30a0b0` in `../hd_fpv_osd_font_tool`, followed there by `1e7c331`, the hook suite port (ticket 01, Built).
Those commits are local only.

Before ticket 02 can start, the maintainer pushes `../hd_fpv_osd_font_tool` and tags `v1.2.0`.

Next: `.scratch/png-fonts/issues/02-discover-png-fonts.md`. It starts with
`cargo update -p hd_fpv_osd_font_tool` against the pushed tag (see `.scratch/png-fonts/spec.md`,
Delivery). Ticket 03 follows.

Still open:

- From `.scratch/handoffs/2026-11-15-packed-alpha-sidecars.md`: a candidate ticket to fall back to
  software encoding when a packed-alpha frame exceeds the hardware encoder's limits.

Suggested skills: `tdd`, `rust-best-practices`, `code-review`.

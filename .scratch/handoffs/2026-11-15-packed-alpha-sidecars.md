# Packed-alpha sidecars

Replaces: `.scratch/handoffs/2026-11-15-chroma-key-follow-up.md`.

Ticket `.scratch/chroma-key-overlays/issues/04-packed-alpha-sidecars.md` is done. Chroma-key
sidecars are replaced by packed-alpha MP4 sidecars, which carry the overlay's real alpha. See the
ticket's diagnosis and `Built` section, `.scratch/chroma-key-overlays/spec.md`,
`docs/overlay-video-quality.md`, and `src/osd/overlay.rs` (`PackedAlphaLayout`).

The follow-up note's recommendations are superseded. Transparent VP9 output remains an explicit
compatibility mode and was not changed.

No open ticket remains in `.scratch/chroma-key-overlays/issues/`. A candidate for a new ticket
is the unfixed gap recorded in ticket 04: falling back to software encoding when the packed
frame exceeds the hardware encoder's limits.

Suggested skills: `tdd`, `rust-best-practices`, `code-review`.

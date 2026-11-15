# Software fallback for oversized frames

Replaces: `.scratch/handoffs/2026-11-16-dji-format-v2.md`.

Ticket 05 of chroma-key overlays is done: see
`.scratch/chroma-key-overlays/issues/05-software-fallback-for-oversized-frames.md` (Decisions,
Built, Review fixes) and the commit that closes it. Encoder choice now lives in
`src/video/encoder_selection.rs`.

The fallback is tested with a fake capabilities object only. On the user's machine the hardware
limits are 8192 px wide for AV1 and H.265, so a 4K-wide packed-alpha sidecar stays in hardware
there; the fallback triggers only on devices with lower limits or with an explicit H.264.

Next: no ticket is open. Ask the user what to take on, and triage it first.

GitHub issue #14 (DJI format v2) still awaits the user's decision on commenting or closing.

Format with `cargo +nightly fmt`: the tracked `.githooks/config` sets `FMT_TOOLCHAIN=nightly`.

Suggested skills: `grilling` or `domain-modeling` to shape the next ticket, then `tdd`,
`rust-best-practices`, `code-review`.

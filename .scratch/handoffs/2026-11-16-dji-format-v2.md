# DJI OSD format v2

Replaces: `.scratch/handoffs/2026-11-16-png-fonts-firmware-variants.md`.

Ticket 05 is done: see `.scratch/png-fonts/issues/05-read-dji-format-v2.md` (Built, Review fixes)
and the commit that closes it. Ticket 04 closed in `a3b3491`. The PNG fonts tickets are all done.

GitHub issue #14 reported the v2 rejection. Whether to comment on or close it is the user's call;
nothing was posted. The fix is tested only with synthetic files, because the sample recording
linked from #13 is gone.

Next: no open ticket. Ask the user what to take. The candidate from
`.scratch/handoffs/2026-11-15-packed-alpha-sidecars.md` is still unwritten: fall back to software
encoding when a packed-alpha frame exceeds the hardware encoder's limits.

Format with `cargo +nightly fmt`: the tracked `.githooks/config` sets `FMT_TOOLCHAIN=nightly`,
overriding the pinned toolchain in `.githooks/pre-commit`.

Suggested skills: `tdd`, `rust-best-practices`, `code-review`.

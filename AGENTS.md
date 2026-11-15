## Start here

Read the latest note in `.scratch/handoffs/` when one exists. It identifies the ticket to take.

## Agent skills

### Implementation cycle

A session implements one ticket, from triage to hand-off, then stops. The Git hooks enforce its
Cargo quality gate and append a `Checked-Tree` trailer to checked commits. See
`docs/agents/implementation-cycle.md`.

### Handoffs

Handoff notes live in `.scratch/handoffs/`, named `YYYY-MM-DD-<topic>.md`, and are committed
with the work they hand off. A new note names the note it replaces as the latest and cites earlier
notes by their repository paths.

### Issue tracker

Issues live as local markdown under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the canonical triage labels plus `done` for completed work. See
`docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: `CONTEXT.md` and `docs/adr/` at the repository root. See
`docs/agents/domain.md`.

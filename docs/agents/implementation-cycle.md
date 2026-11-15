# Implementation cycle

A session implements **one** ticket, then stops. Each ticket gets its own fresh session. Paths are
relative to the repository root.

1. **Read** the latest note in `.scratch/handoffs/`, when one exists, then the ticket. Read its
   `Built` and `Comments` sections for earlier findings. Read `CONTEXT.md` and the ADRs in
   `docs/adr/` that touch the area, where they exist.
2. **Triage** first when the ticket's `Status:` is not `ready-for-agent`: settle its open
   questions with the user, each with a recommended option, then agree the seams under test.
   Record both in the ticket before changing code.
3. **Implement** test-first with the `tdd` skill at the agreed seams, one slice at a time. Add
   focused regression coverage for defects and use temporary directories or other isolated
   filesystem fixtures for filesystem behavior.
4. **Check** the staged work with `bash .githooks/pre-commit`. It runs formatting, man-page
   generation, compilation, Clippy, documentation, tests, and ShellCheck against the staged tree.
   Let it finish before modifying sources that it compiles. Done means the hook is clean.
5. **Review** code changes with the `code-review` skill, using the ticket as the specification.
   Check each reported finding against the code before reporting it. Skip this step for ticket-only
   or documentation-only changes.
6. **Fix** review findings. Fix small, clear-cut findings. For important findings or judgement
   calls, ask the user with a recommended option.
7. **Handle each gap** found along the way:
   - **Small and clear-cut** (a typo, missing documentation line, or test helper): fix it in this
     ticket and record it under `Built`.
   - **A real defect, missing behavior, or a gap too large to fix here**: add it to the related
     open area ticket as a checkbox, saying where and how it was found. Open a new ticket only
     when no area fits.
   - **A judgement-call smell left unresolved by review**: mention it in the reply and do not open
     a ticket.
8. **Close** the ticket: tick its checkboxes, set its `Status:` to `done`, and record under
   `Built` what was built, gaps found along the way, related tickets opened, and review fixes
   approved by the user.
9. **Commit** when the user requests it, including the ticket and its handoff note. Commit through
   the hooks; a successful commit receives a `Checked-Tree` trailer for the staged tree it checked.
   Push only when the user asks.
10. **Hand off** the next open ticket in `.scratch/handoffs/YYYY-MM-DD-<topic>.md`. Name the note
    it replaces and the next ticket, point to commits and files rather than restating them, and
    list suggested skills. Commit the note with the work it hands off. End the session after
    outputting the note's path in plain text, with no link or code formatting. If the user requests
    follow-up work, keep the note current and commit its updates with that work.

# Ace Blocker — Technical Debt Backlog

Use [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md) for the required audit, implementation, and independent-review process.

This file tracks **verified, actionable maintainability work**, not a wishlist of speculative rewrites. Add items only after inspecting the relevant code and identifying concrete costs. Audit findings may also be marked `candidate` if they need verification.

## Queue

| ID | Priority | Status | Area | Problem / evidence | Intended outcome |
| --- | --- | --- | --- | --- | --- |
| — | — | — | — | *No audit has been recorded yet.* | — |

For each non-placeholder item, add a detailed section below using the template. Link the queue's ID to its section. Order by expected maintenance benefit and risk, not cosmetic code metrics.

## Item template

Copy this section for each verified or candidate finding. Do not fabricate evidence or completion details.

### TD-001 — Short, actionable title

- **Status:** candidate / verified / ready / in_progress / review / done / blocked / rejected
- **Priority:** P0 (urgent correctness/security) / P1 (high cost) / P2 (routine) / P3 (opportunistic)
- **Area and files:** `path/to/file.rs` — symbol/function and line or search identifier
- **Evidence:** Observed duplication, coupling, unnecessary dependency, obsolete path, or test gap. Cite concrete call sites.
- **Cost:** Why this makes maintenance harder or increases regression risk.
- **Smallest proposed improvement:** Favor simplification or deletion; state why a new abstraction is truly needed, if one is proposed.
- **In scope / out of scope:** Explicit module boundaries; no bundled unrelated rewrites.
- **Behavior and security invariants:** Identify the observable contracts to preserve.
- **Baseline / tests needed:** Commands, currently observed results, and any characterization tests to add before edits.
- **Dependencies / blockers:** None or explicit linked items.
- **Owner / branch or worktree:** Unassigned until work starts.
- **Reviewer and review findings:** Independent review record.
- **Outcome / verification / commit:** Fill in only after implementation and actual checks.

## Operating rules

1. Read-only audits may add or update backlog items; they must not alter product code.
2. Implementation tasks claim a single ID and keep code changes bounded.
3. Reviewers verify changes independently and note unresolved risks.
4. Mark `done` only after passing the relevant checks, recording any checks that could not run, and completing independent review.
5. Do not silently erase or reprioritize someone else's in-progress task.

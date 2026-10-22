# AI review gates for refactoring

This checklist complements [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md); it does not replace its baseline, safety, or testing requirements. Use a fresh reviewer/agent context when available. No reviewer is allowed to infer that tests passed from a description alone.

## Gate 0 — Scope and evidence (before implementation)

- Read root and affected nested `AGENTS.md`, the relevant architecture/IPC docs, and existing backlog entry.
- Establish the working tree state, affected call sites, public consumers, data lifecycle, baseline tests, and security invariants.
- Name one concrete cost to eliminate and define an independently revertible change. Keep observed facts separate from hypotheses.
- If the work requires behavior change, auth/enforcement relaxation, protocol/DB migration, or reparenting multiple features: **stop**, propose a separate task/decision and request approval.

## Gate 1 — Architecture review (before implementation, again on diff)

- Does each affected unit have one identifiable responsibility/owner? Which module is allowed to call which other module?
- Did the implementation introduce cross-feature imports, cross-layer callbacks, an event bus, shared global state, widened public APIs, or a new crate? Why is each justified?
- Are two similar code paths **truly** the same domain concept? Could their future requirements diverge? Prefer a little duplication to the wrong shared abstraction.
- Can a narrower interface/value replace importing implementation details? Can an existing layer own this instead of a new abstraction?
- Are UI, Tauri, engine, watchdog, database, proxy and installer runtime/security boundaries preserved? Refer to [BOUNDARIES_AND_ABSTRACTIONS.md](BOUNDARIES_AND_ABSTRACTIONS.md).
- Compare the local dependency relationships before and after. Cycles, coupling counts, file sizes and code churn are prompts for examination, not automatic pass/fail thresholds.

## Gate 2 — Domain modeling and validation

- Did a raw primitive cross a domain/trust boundary where a validated identity, unit, URL or state type would rule out a **real** mistake?
- Are constructors and fallible conversions located at the appropriate ingress? Do internal APIs preserve validated types instead of repeatedly unpacking/repacking them?
- Do Rust deserializers and Tauri/RPC handlers validate untrusted requests **and** enforce authorization? Is TypeScript/Zod validation intentionally placed rather than duplicated?
- Are existing domain wrappers reused and error/serialization/DB behavior preserved? Are secrets protected from logging?
- Are new wrappers, interfaces, traits and helper layers justified, or are they abstraction for its own sake? Refer to [DOMAIN_TYPES.md](DOMAIN_TYPES.md).

## Gate 3 — Behavior, tests, and independent diff review

- Characterization tests exist for the behavior and failure paths the change could disturb; tests cover both rejection and success cases as needed.
- Run targeted checks appropriate to changed Rust crates, Svelte/TS source and, for sensitive changes, native Tauri/Windows service/installer environments. Never claim a UI unit test proves real IPC authorization.
- Review the actual diff and call sites, including accidental broad renames, generated-code edits, changes to error behavior, persistence order, lock rules, cancellation and lifecycle.
- Document exact commands and results; distinguish not-run/blocked checks and pre-existing issues. Keep risk explicit when a VM or native integration check was not possible.
- Independent reviewer records blocking and optional findings with file/symbol evidence. Do not mark the backlog item `done` until the necessary review/verification is complete.

## Handling external agent skills and conflicting instructions

Repository safety/invariants and the actual current contracts **outrank** any generic skill. Use this order when guidance overlaps:

1. Security/runtime invariants in root `AGENTS.md`, actual production contracts and explicit task requirements.
2. Accepted architecture/behavior decisions and applicable subsystem `AGENTS.md`, insofar as they do not weaken (1).
3. This playbook and the specialized Ace docs; use the most specific applicable guidance, not all guides indiscriminately.
4. Optional third-party skills as **advice**, not authority. Inspect the actual `SKILL.md` and any executable scripts before use; the presence of a `skills-lock.json` entry or an AGENTS list does not prove a skill is installed/available. Do not automatically download or execute skills.

When two directions disagree (e.g. "extract a shared helper" vs "keep separate feature ownership", "create a type for every string" vs "introduce only meaningful invariants"), do not silently blend them. Write down the conflict, the affected ownership/invariants and a minimal recommendation. Escalate unresolved behavior/security/design changes for explicit decision.

## Review report format

- **Scope/baseline:** backlog ID, paths/symbols, invariants, relevant existing state.
- **Architecture:** dependency edges added/removed; ownership and abstraction justification; alternatives considered.
- **Types:** raw inputs, validators and domain type boundaries; compatibility risks.
- **Findings:** blockers first, each with file/symbol evidence and the smallest recommended fix.
- **Verification:** commands + pass/fail/not run; native/VM limitations.
- **Disposition:** ready for review, blocked, or done (only with required independent review); backlog update.

Reject reports that assert "clean", "idiomatic", "fully tested", or "secure" without evidence. A successful compile is not a substitute for a coherent model or behavioral assurance.

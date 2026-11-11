# AI review gates for refactoring

Use this checklist with [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md). It does not replace baseline, safety, or test rules. Use a reviewer with a separate context when one is available. Do not report a test as passed without test evidence.

## Gate 0 — Scope and evidence (before implementation)

- Read root and relevant nested `AGENTS.md` files. Read related architecture and IPC documents. Read the current backlog entry.
- Record Git status, affected call sites, public consumers, data lifetime, baseline tests, and security requirements.
- Name the real cost to remove. Define a change that can be reverted on its own. Keep facts separate from assumptions.
- **Stop and ask for approval** if the change affects behavior, relaxes security or enforcement, migrates a protocol or DB, or moves several features to a new owner. Make a separate proposal.

## Gate 1 — Architecture review (before implementation, again on diff)

- Identify the owner and responsibility of each affected unit. State which modules can call other modules.
- List each new cross-feature import, cross-layer callback, event bus, global state, public API, or crate. Explain the need for each one.
- Confirm that code paths represent the **same** domain concept before you share them. Consider future differences. Prefer some duplication to an incorrect shared abstraction.
- Prefer a small interface or value over implementation imports. Check whether an existing layer can own the behavior.
- Preserve boundaries between UI, Tauri, engine, watchdog, database, proxy, and installer. See [BOUNDARIES_AND_ABSTRACTIONS.md](BOUNDARIES_AND_ABSTRACTIONS.md).
- Compare local dependencies before and after the change. Review cycles, coupling, file size, and changed lines as warning signs. Do not use counts alone as pass/fail rules.

## Gate 2 — Domain modeling and validation

- Check for raw values that cross a domain or trust boundary. Add a validated identity, unit, URL, or state type only when it prevents a **real** mistake.
- Put constructors and fallible conversion at the boundary that owns the input. Keep validated types inside the system. Do not repeatedly unpack and rebuild them.
- Validate untrusted requests in Rust deserializers and Tauri/RPC handlers. Enforce authorization as a separate check. Use TypeScript/Zod validation only where needed.
- Reuse existing domain types. Preserve error, serialization, and DB behavior. Protect secrets from logs.
- Explain each new wrapper, interface, trait, and helper layer. Remove layers that have no proven need. See [DOMAIN_TYPES.md](DOMAIN_TYPES.md).

## Gate 3 — Behavior, tests, and independent diff review

- Add characterization tests for behavior and failure paths that could change. Test accepted and rejected operations where applicable.
- Run focused Rust, Svelte/TypeScript, and native Tauri/Windows service/installer checks for the affected scope. Do not treat a UI unit test as proof of IPC authorization.
- Review the actual diff and call sites. Check broad renames, generated code, error behavior, persistence order, locks, cancellation, and lifecycle.
- Record exact commands and results. Label tests as passed, failed, blocked, or not run. State risk when a native or VM check cannot run.
- Require an independent reviewer to record blocking and optional findings with file and symbol evidence. Do not set a backlog item to `done` before required review and tests finish.

## Handling external agent skills and conflicting instructions

Repository security requirements and current contracts **take priority** over generic skills. Follow this order when guidance conflicts:

1. Security/runtime invariants in root `AGENTS.md`, actual production contracts and explicit task requirements.
2. Accepted architecture/behavior decisions and applicable subsystem `AGENTS.md`, insofar as they do not weaken (1).
3. This playbook and the specialized Ace docs; use the most specific applicable guidance, not all guides indiscriminately.
4. Optional third-party skills as **advice**, not authority. Inspect the actual `SKILL.md` and any executable scripts before use; the presence of a `skills-lock.json` entry or an AGENTS list does not prove a skill is installed/available. Do not automatically download or execute skills.

When two instructions conflict, record the conflict, affected ownership, and constraints. Recommend the smallest safe action. Ask for an explicit decision when a behavior, security, or architecture change remains unresolved. Do not silently combine conflicting instructions.

## Review report format

- **Scope/baseline:** backlog ID, paths/symbols, invariants, relevant existing state.
- **Architecture:** dependency edges added/removed; ownership and abstraction justification; alternatives considered.
- **Types:** raw inputs, validators and domain type boundaries; compatibility risks.
- **Findings:** blockers first, each with file/symbol evidence and the smallest recommended fix.
- **Verification:** commands + pass/fail/not run; native/VM limitations.
- **Disposition:** ready for review, blocked, or done (only with required independent review); backlog update.

Reject claims such as "clean", "idiomatic", "fully tested", or "secure" without evidence. Compilation alone does not prove that the design or behavior is correct.

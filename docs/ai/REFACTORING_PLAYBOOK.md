# Ace Blocker — AI Refactoring Playbook

**Purpose:** Remove real technical debt. Keep product behavior and enforcement unchanged. Do not replace simple code with unnecessary abstractions.

**Use this guide** when an AI must audit, tidy, simplify, refactor, or remove obsolete code in the unified Ace Blocker repository. First read root `AGENTS.md` and relevant component instructions.

**Use only the guidance that applies.** Read [BOUNDARIES_AND_ABSTRACTIONS.md](BOUNDARIES_AND_ABSTRACTIONS.md) for ownership and dependencies. Read [DOMAIN_TYPES.md](DOMAIN_TYPES.md) for types and validation. Read [REVIEW_GATES.md](REVIEW_GATES.md) for separate architecture and type reviews, independent diff review, and conflicts between skills. Root security rules and approved contracts take priority. Verify that a named skill is installed before you use it.

This is a **repeatable process**. It does not approve a repository-wide rewrite. An audit can cover the repository. Keep each implementation small and separately verifiable.

## Principles

1. **Keep behavior unchanged.** A refactor changes code structure, not visible behavior. Get separate explicit approval for behavior or architecture changes.
2. **Remove code before you add abstractions.** Prefer to delete dead code, duplication, and unnecessary state. Use current libraries and patterns before you add helpers, interfaces, layers, or dependencies.
3. **Use evidence.** Identify a real cost such as defects, duplication, hard tests, coupling, unclear ownership, obsolete compatibility code, or repeated maintenance. File length alone is not proof of debt.
4. **Change one concern at a time.** Keep changes small enough to understand, test, review, and revert. Do not combine IPC migration with proxy or watchdog work.
5. **Protect security requirements.** Do not weaken enforcement, locks, privileges, authentication, process isolation, or recovery to make code simpler.
6. **Report actual test results.** State that a check passed only if it ran and passed. A unit test does not prove Windows service behavior or resistance to tampering.
7. **Keep reviews small.** Avoid unrelated formatting, renames, file moves, dependency upgrades, and speculative abstractions.
8. **Keep a record.** Put confirmed debt and completed changes in `docs/ai/TECH_DEBT_BACKLOG.md`. Do not invent evidence, findings, or test results.
9. **Keep domain boundaries.** Do not connect independent features, crates, stores, or layers because their code looks similar. Some duplication is safer than incorrect sharing.
10. **Use types for real rules.** Check raw values at domain and trust boundaries. Do not add types, traits, schemas, or wrappers without a clear benefit. Keep wire and storage contracts. Validate untrusted input where the system owns the rule.
11. **Review design separately from build results.** Passing tests and lint does not prove correct dependencies, ownership, domain models, or state lifetime.

## Workflow: audit → test → refactor → independent review → merge

### 1. Audit (read-only)

- Read the repository structure, relevant `AGENTS.md`, call sites, dependency graphs, tests, and runtime entry points. Follow behavior across Rust and JavaScript boundaries before you recommend changes.
- Check for:
  - abandoned or unreachable code, stale migration paths, unused dependencies;
  - duplicate implementations, unnecessary indirection, confusing ownership;
  - state synchronization problems, unnecessary polling, scattered error handling;
  - excessive coupling, cycles, leaky APIs, inconsistent types or naming;
  - accidental cross-feature imports, unnecessary shared stores/managers or code extraction across independently evolving concepts;
  - raw primitives masking identity, domain validation, units or secrets; and the converse problem of pointless wrapper types;
  - duplicate state, validation or implicit authority crossing UI → Tauri → remoc → engine;
  - missing regression tests in areas where cleanup would be risky.
- Separate **verified debt** with file and symbol evidence from **hypotheses** that need further study.
- Rank work by expected maintenance benefit, regression risk, user impact, and required tests. Do **not** rank primarily by lines removed.
- Record tasks in `docs/ai/TECH_DEBT_BACKLOG.md`. Make each task small enough for independent review. Do not change product code during an audit.

**Audit output:** ranked backlog, evidence for each item, risk classification, suggested test coverage, and recommended next item.

### 2. Select a bounded task and establish the baseline

Before code edits:

- Check `git status`. Keep other contributors' work. Do not reset, stage, revert, or reformat unrelated files. Use separate branches or worktrees for parallel changes.
- Select **one backlog item**. Define its scope, the complexity to remove, and the behavior to keep.
- Identify ownership, rules, consumers, and simpler choices before you add a helper, trait, type, import, or shared state. Explain why current boundaries do not work. Use the architecture and domain-type guides.
- Record **dependency and type effects**. List current and proposed dependencies and contracts. State where validation occurs. Explain why unrelated systems remain independent. Do not implement a design you cannot justify.
- Find and run relevant existing tests. Record the results. Add focused behavior and regression tests **before** risky structural changes when coverage is weak.
- Define acceptance checks and a rollback boundary for one commit or PR. Stop and report a blocker if you cannot verify a high-risk change.

**Scope rule:** Change one module, subsystem, or repeated pattern per task. Use broader mechanical edits only when their effects and tests are predictable and easy to review.

### 3. Implement the smallest behavior-preserving change

- Make small, mechanical changes. Keep behavior and serialization, IPC, and DB contracts unless you have separate approval.
- Remove unused code. Do not keep both old and new paths without a reason.
- Use current language and library features. Do not reimplement utilities that dependencies already provide. Do not share logic only because two functions look alike.
- Keep local Svelte state local. Share a stable contract only when needed. In Rust, use domain-owned types and fallible validation for real rules. Do not wrap every string.
- Keep current system contracts, stored data, time meaning, error behavior, reconnect handling, and installer behavior.
- If a cleanup needs product behavior changes, a new architecture layer, or broad cross-component changes, **propose it as a separate task**.
- Run targeted checks while you work. Do not hide warnings, omit tests, or weaken validation to get a passing result.

### 4. Independent review

Use a **separate reviewer or fresh AI context** where possible. Review the actual diff against the baseline. Do not rely only on the implementer's report.

Complete **two separate design reviews** before the general correctness review:
- **Architecture/abstraction review:** Check ownership, new dependencies, module independence, import direction, state lifetime, shared-code reasons, and unnecessary coupling.
- **Domain/types review:** Check real invariants, constructors, parsers, input validation, trust boundaries, existing types, untrusted deserialization, secret handling, and unnecessary wrappers.

Follow the fuller checklist in [REVIEW_GATES.md](REVIEW_GATES.md). Review for:
- preserved behavior and complete migration of affected call sites;
- new or weakened security boundaries, failure modes, races, or unchecked errors;
- regressions in startup/recovery, restart, shutdown, and uninstall paths;
- gratuitous abstraction, complexity transferred to another file, and unnecessary new dependencies;
- insufficient test coverage or misleading test results;
- unexpected changes outside the approved scope.

The reviewer must report **specific findings with file references**. Separate blockers from optional changes. State when evidence is missing. The implementer must fix relevant findings. The reviewer must then check the updated diff.

### 5. Verify and finish

- Run relevant compile, test, lint, and type checks. Run integration and VM checks for service or installer changes.
- Compare behavior and critical contracts with the baseline. Fix regressions before you close the task.
- Record **what ran, passed, failed, or did not run**. Include environment limits. Passing CI does not prove complete security.
- Record the result, commit or PR, evidence, and follow-up work in the backlog.
- Commit **only files in this task** after you review the final diff. Do not change unrelated work.

## Ace-specific invariants

These apply even when a proposed cleanup looks trivial:

- **Separate processes:** Keep `ace-engine` and `ace-watchdog` as independent privileged services. Keep Tauri/Svelte unprivileged. Closing the UI must not stop enforcement.
- **Enforcement:** Keep network and site blocks, schedules, Resistance Mode, stored locks, and unlock or expiry behavior unchanged.
- **Anti-tampering:** Keep watchdog authentication, fresh heartbeats, recovery, authorized maintenance, Windows SCM settings, and lock-aware uninstall checks.
- **Privilege/IPC boundaries:** Keep caller checks, named-pipe and endpoint ACLs, input validation, and engine authorization. Do not trust UI input only because it has a type.
- **Network/proxy:** Keep browser interception and bypass behavior. Do not decrypt non-browser traffic by accident. Keep external HTTP separate from local IPC.
- **Installer:** Keep WiX responsible for privileged service lifecycle, ACLs, recovery, certificates, upgrades, and uninstall checks.
- **Platform compatibility:** Keep existing Linux behavior when you change Windows-specific code.
- **Repository boundaries:** Change only this unified repository. Do not edit the original sibling repositories.

Changes in these areas are **high-risk** by default. Compilation alone is not enough. Run focused regression tests and use an isolated Windows VM where applicable. Mark work as **not fully verified** when these checks cannot run.

## Useful checks (select those that apply)

From the workspace root:

```powershell
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test -p ace-watchdog-protocol
cargo test -p ace-engine
cargo test -p ace-watchdog
```

From `apps/ace-app/`:

```powershell
bun run check
bun run test:unit
```

Use relevant integration or end-to-end tests and the installer VM test matrix for affected boundaries. **Do not run a repository-wide autofix** for a smaller task. Examples include `cargo clippy --fix`, `cargo fmt`, and mass formatting. Report old warnings separately.

Optional tools include `cargo-machete` for unused dependency candidates (check false positives), `cargo-nextest` for Rust tests, `ast-grep` for reviewed structural edits, and coverage tools for gaps. These tools do not replace code review. Use `cargo metadata`, targeted call-site searches, and Svelte/TypeScript checks for dependency evidence. Do not install tools or set arbitrary limits without a real need. File, line, import, and newtype counts are diagnostic signals, **not goals**.

Treat Skills.sh and local skill packs as optional references. Confirm that `SKILL.md` exists. Review its instructions and scripts. Follow Ace security, service, IPC, and ownership contracts when a general skill conflicts with them. Record the conflict. See [REVIEW_GATES.md](REVIEW_GATES.md).

## Backlog and delivery rules

The backlog is [TECH_DEBT_BACKLOG.md](TECH_DEBT_BACKLOG.md). Record these details for each item:

- unique ID, component, priority, status and evidence (paths/symbols);
- why the issue causes real maintenance cost, and the simplest proposed remedy;
- scope boundaries, behavior/security constraints, required regression tests;
- dependencies/blockers, owner or working branch if applicable, and completion/verification record.

Suggested statuses: `candidate` → `verified` → `ready` → `in_progress` → `review` → `done` (or `blocked` / `rejected`).

Keep active refactoring small. Prefer **one implementation task in each overlapping area**. Read-only audits can run together. Use separate worktrees and non-overlapping files for parallel writers.

## Ready-to-use agent requests

### A. Audit

> Follow `docs/ai/REFACTORING_PLAYBOOK.md` in **audit** mode. Inspect Ace Blocker for maintainability debt without modifying product code. Back each finding with exact file paths/symbols and concrete maintenance costs; distinguish verified facts from hypotheses. Update `docs/ai/TECH_DEBT_BACKLOG.md` with ranked, independently executable tasks. Prioritize deletion and simplification over new abstractions. Preserve all enforcement and anti-tampering invariants. Report your highest-value next task.

### B. Execute one backlog item

> Follow `docs/ai/REFACTORING_PLAYBOOK.md` in **implementation** mode for backlog item `<ID>`. Do not address other backlog items. Inspect call sites, establish a baseline, and add focused regression coverage where needed. Implement the smallest behavior-preserving cleanup, run relevant checks, and update the backlog with exact verification results. Do not alter unrelated uncommitted work. Arrange a separate diff review before marking complete.

### C. Independent review

> Follow `docs/ai/REFACTORING_PLAYBOOK.md` in **review** mode for backlog item `<ID>`. Review the implementation diff independently against its acceptance criteria, focusing on changed behavior, missing call sites, security/anti-tampering regressions, unnecessary complexity, and missing tests. Report findings in priority order with file references. Do not modify code unless explicitly asked.

## Definition of done

A refactor is complete only after the scope is satisfied, behavior is unchanged, new complexity is justified, checks are recorded, an independent reviewer has checked the diff, and the backlog is updated. Otherwise mark it incomplete or blocked. Do not claim unverified success.

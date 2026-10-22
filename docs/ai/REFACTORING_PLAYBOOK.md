# Ace Blocker — AI Refactoring Playbook

**Purpose:** reduce real technical debt without changing product behavior, weakening enforcement, or replacing understandable code with new abstractions.

**Use this document whenever an AI is asked to audit, tidy, simplify, refactor, or remove obsolete code in the unified Ace Blocker repository.** Read the root `AGENTS.md` and the relevant component instructions first.

**Load guidance by concern, not all at once:** [BOUNDARIES_AND_ABSTRACTIONS.md](BOUNDARIES_AND_ABSTRACTIONS.md) for ownership, imports and extraction decisions; [DOMAIN_TYPES.md](DOMAIN_TYPES.md) for newtypes, invariants and validation; and [REVIEW_GATES.md](REVIEW_GATES.md) for the two distinct architecture/type reviews, independent diff review and conflicting skill advice. Root runtime/security rules and existing accepted contracts always take priority over generic skills. Do not assume a named skill is installed until its actual contents are available.

This is a **repeatable process**, not permission for a repository-wide rewrite. An audit may cover the whole repository, but implementation must be scoped, incremental, and independently verifiable.

## Principles

1. **Preserve behavior first.** A refactor changes structure, not externally observable semantics. A deliberate behavior or architecture change is a separate, explicitly approved task.
2. **Delete and simplify before abstracting.** Prefer removing dead code, duplication, or unnecessary state over introducing helpers, interfaces, layers, frameworks, or dependencies. Favor the existing libraries and patterns.
3. **Evidence, not aesthetic preferences.** Identify a concrete cost: duplication, bugs, difficult testing, coupling, unclear ownership, abandoned compatibility code, or repeated maintenance work. Large files alone are not proof of debt.
4. **One coherent concern at a time.** Bound changes so they can be understood, tested, reviewed, and reverted separately. Do not combine IPC migration, proxy rewrites, and watchdog cleanup.
5. **Protect security invariants.** Never trade away enforcement, lock behavior, privileges, authentication, process isolation, or recovery for cleaner-looking code.
6. **Run checks and report the truth.** A compile succeeds only if it ran and passed. A unit test does not prove that Windows service behavior or anti-tampering still works.
7. **Minimize review cost.** Avoid unrelated formatting, gratuitous renaming, moving files without benefit, large dependency upgrades, and speculative abstractions.
8. **Keep a durable record.** Track discovered debt and completed work in `docs/ai/TECH_DEBT_BACKLOG.md`. Never invent findings, test outcomes, or supporting references.
9. **Respect semantic boundaries.** Do not connect unrelated features, crates, stores or layers just because their code looks similar. Small duplication can be cheaper and safer than an abstraction that couples independent behavior.
10. **Use types to enforce real invariants.** Check if an unchecked primitive crosses a meaningful domain/trust boundary; also reject newtypes, traits, schemas and wrappers with no demonstrable benefit. Preserve wire/storage compatibility and validate untrusted inputs at the authoritative boundary.
11. **Check design separately from compilation.** Green tests and lint do not prove that dependency direction, responsibility ownership, domain concepts, and state lifecycle are sound.

## Workflow: audit → test → refactor → independent review → merge

### 1. Audit (read-only)

- Inspect repository structure, the relevant component `AGENTS.md` files, call sites, dependency graphs, tests, and build/runtime entry points. Trace behavior across crate/JS boundaries before recommending a change.
- Check for:
  - abandoned or unreachable code, stale migration paths, unused dependencies;
  - duplicate implementations, unnecessary indirection, confusing ownership;
  - state synchronization problems, unnecessary polling, scattered error handling;
  - excessive coupling, cycles, leaky APIs, inconsistent types or naming;
  - accidental cross-feature imports, unnecessary shared stores/managers or code extraction across independently evolving concepts;
  - raw primitives masking identity, domain validation, units or secrets; and the converse problem of pointless wrapper types;
  - duplicate state, validation or implicit authority crossing UI → Tauri → remoc → engine;
  - missing regression tests in areas where cleanup would be risky.
- Separate **verified debt** (with exact paths/symbols and evidence) from **hypotheses** that require further investigation.
- Prioritize by expected maintenance benefit, likelihood of regression, user impact, and prerequisite tests. Do **not** use lines removed as the primary ranking metric.
- Record actionable items in `docs/ai/TECH_DEBT_BACKLOG.md`. An item should describe a single independently reviewable improvement. Do not modify product code during an audit.

**Audit output:** ranked backlog, evidence for each item, risk classification, suggested test coverage, and recommended next item.

### 2. Select a bounded task and establish the baseline

Before code edits:

- Check `git status`. Preserve other contributors' work; do not revert, reset, stage, or reformat unrelated changes. Use an isolated branch/worktree when parallel agents are active.
- Pick **one backlog item**. Define the exact boundary, the undesirable complexity to remove, and the behavior that must stay unchanged.
- Before inventing a helper, trait, new type, cross-feature import or shared state, document the owning concept, invariant/use case, simpler alternatives, consumers and the reason existing boundaries are insufficient (see architecture and domain-type guides).
- Record a **dependency/type impact sketch** for the affected scope: current edges and contracts, edges proposed, where validation happens, and why no unrelated subsystem becomes coupled. If the design cannot be justified, do not implement it as a cleanup.
- Locate and run relevant existing tests/checks; record results. For a risky path with inadequate coverage, add focused characterization/regression tests **before** structural changes.
- Write explicit acceptance criteria and a rollback boundary (one reviewable commit/PR). If you cannot establish how to verify a high-risk change, stop and report the blocker rather than guessing.

**Scope rule:** one module/subsystem or one repeated pattern per refactoring task. Wider mechanical transforms are acceptable only when their effect and tests are deterministic and easy to review.

### 3. Implement the smallest behavior-preserving change

- Prefer small, mechanical, incremental patches; preserve public behavior and serialization/IPC/database contracts unless specifically authorized otherwise.
- Remove unused code rather than keeping redundant old and new paths indefinitely.
- Use existing language/library capabilities. Do not hand-roll utilities already provided by dependencies. Do not extract shared logic merely because two implementations resemble each other.
- Keep local Svelte state local; share only a stable contract. For Rust, prefer domain-owned types and fallible validation where real invariants exist, not pervasive string wrappers.
- Maintain compatibility at system boundaries, including persisted data, time interpretation, error semantics, reconnect handling, and installer behavior.
- If a supposed cleanup requires new product semantics, a new architectural layer, or broad cross-component churn, **split it into a separately proposed task**.
- Run targeted checks during implementation; do not hide warnings, skip tests, or weaken validation to obtain green results.

### 4. Independent review

Use a **separate reviewer or fresh AI context** where practical. Review the actual diff against the baseline, not just the implementer's summary.

Perform **two explicit design passes** before general correctness review:
- **Architecture/abstraction pass:** verify ownership, new dependency edges, module independence, import direction, state lifecycle, shared-code justification and the absence of convenience-driven coupling.
- **Domain/types pass:** verify meaningful invariants, constructors/parsers, validation and trust boundaries, use of existing types, untrusted deserialization, secret handling, and absence of gratuitous wrappers.

Follow the fuller checklist in [REVIEW_GATES.md](REVIEW_GATES.md). Review for:
- preserved behavior and complete migration of affected call sites;
- new or weakened security boundaries, failure modes, races, or unchecked errors;
- regressions in startup/recovery, restart, shutdown, and uninstall paths;
- gratuitous abstraction, complexity transferred to another file, and unnecessary new dependencies;
- insufficient test coverage or misleading test results;
- unexpected changes outside the approved scope.

The reviewer must produce **actionable findings with file references**, distinguish blockers from optional improvements, and explicitly say when evidence is insufficient. The implementer addresses findings, and the reviewer rechecks the affected diff.

### 5. Verify and finish

- Run relevant compilation, tests, linting, and type checking; run integration/VM checks for service and installer changes.
- Compare behavior and critical contracts with the baseline. Fix regressions before considering the task complete.
- Report **what ran / passed / failed / was not run**, with relevant environmental limitations. Do not equate green CI with end-to-end security validation.
- Update the backlog item with outcome, commit/PR reference, evidence, and any newly discovered follow-ups.
- Stage/commit **only files belonging to this task**, after reviewing the final diff. Leave unrelated work alone.

## Ace-specific invariants

These apply even when a proposed cleanup looks trivial:

- **Separate processes:** `ace-engine` and `ace-watchdog` remain independent privileged services; the Tauri/Svelte UI is unprivileged. Closing the UI must not disable enforcement.
- **Enforcement:** network/site blocking, schedules, resistance behavior, persisted locks, and unlock/expiry rules retain exactly their behavior.
- **Anti-tampering:** watchdog authentication, heartbeat freshness, recovery, maintenance authorization, Windows SCM configuration, and lock-aware uninstall remain intact.
- **Privilege/IPC boundaries:** preserve caller validation, named-pipe/endpoint ACLs, input validation, and authorization at the engine boundary. Never trust a UI-originated claim merely because it is typed.
- **Network/proxy:** preserve correct browser interception and bypass behavior, avoid unintended decryption of non-browser traffic, and keep external HTTP integrations separate from local IPC decisions.
- **Installer:** WiX remains responsible for privileged service lifecycle, ACLs, recovery, certificates, upgrades, and uninstallation checks.
- **Platform compatibility:** do not silently drop existing Linux paths while changing Windows-specific code.
- **Repository boundaries:** work only in this unified repository; do not alter the original sibling repositories.

Changes in any of these areas are **high-risk** by default: a successful compile is insufficient. Use focused regression tests and, where applicable, an isolated Windows VM. If the environment cannot run them, mark the change as **not fully verified**.

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

Use the relevant integration/e2e tests and installer VM test matrix for changes that affect those boundaries. **Do not automatically run an autofix over the whole repository** (for example, `cargo clippy --fix`, `cargo fmt`, or mass formatting) when task scope is narrower. Treat pre-existing warnings separately from new ones.

Additional *optional* tools: `cargo-machete` for potential unused dependencies (verify false positives), `cargo-nextest` for running Rust tests, `ast-grep` for reviewed structural transformations, and coverage reports to locate gaps. These are aids, not substitutes for understanding behavior. Existing tools such as `cargo metadata`, targeted import/call-site searches, and Svelte/TypeScript type checks can provide dependency evidence. Do not install new tooling or set hard thresholds without an identified need; counts of files, lines, imports, or newtypes are diagnostic signals, **not automatic refactoring goals**.

External Skills.sh or local skill packs are optional specialist references. Check that the actual `SKILL.md` exists and inspect its instructions/security implications. If a generic skill conflicts with the root service, security, IPC or ownership rules, follow the documented Ace contract and record the conflict instead of silently reconciling it; see [REVIEW_GATES.md](REVIEW_GATES.md).

## Backlog and delivery rules

The backlog lives in [TECH_DEBT_BACKLOG.md](TECH_DEBT_BACKLOG.md). For each entry record:

- unique ID, component, priority, status and evidence (paths/symbols);
- why the issue causes real maintenance cost, and the simplest proposed remedy;
- scope boundaries, behavior/security constraints, required regression tests;
- dependencies/blockers, owner or working branch if applicable, and completion/verification record.

Suggested statuses: `candidate` → `verified` → `ready` → `in_progress` → `review` → `done` (or `blocked` / `rejected`).

Keep active refactoring work small, preferably **one implementation task at a time per overlapping area**. Multiple read-only audits may run concurrently; parallel write agents should use independent worktrees and non-overlapping files.

## Ready-to-use agent requests

### A. Audit

> Follow `docs/ai/REFACTORING_PLAYBOOK.md` in **audit** mode. Inspect Ace Blocker for maintainability debt without modifying product code. Back each finding with exact file paths/symbols and concrete maintenance costs; distinguish verified facts from hypotheses. Update `docs/ai/TECH_DEBT_BACKLOG.md` with ranked, independently executable tasks. Prioritize deletion and simplification over new abstractions. Preserve all enforcement and anti-tampering invariants. Report your highest-value next task.

### B. Execute one backlog item

> Follow `docs/ai/REFACTORING_PLAYBOOK.md` in **implementation** mode for backlog item `<ID>`. Do not address other backlog items. Inspect call sites, establish a baseline, and add focused regression coverage where needed. Implement the smallest behavior-preserving cleanup, run relevant checks, and update the backlog with exact verification results. Do not alter unrelated uncommitted work. Arrange a separate diff review before marking complete.

### C. Independent review

> Follow `docs/ai/REFACTORING_PLAYBOOK.md` in **review** mode for backlog item `<ID>`. Review the implementation diff independently against its acceptance criteria, focusing on changed behavior, missing call sites, security/anti-tampering regressions, unnecessary complexity, and missing tests. Report findings in priority order with file references. Do not modify code unless explicitly asked.

## Definition of done

A refactor is done only when the scope is satisfied, observable behavior is preserved, new complexity is justified, relevant checks are recorded, the diff has been independently reviewed, and the backlog has been updated. Otherwise mark it incomplete or blocked; never report speculative success.

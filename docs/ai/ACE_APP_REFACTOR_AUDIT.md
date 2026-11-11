# Ace App — Outstanding Refactoring Audit

**Original audit:** 2026-10-22 · **Last pruned:** 2026-10-22

**Repository:** `C:\Projects\ace-project\ace-blocker`

**Scope:** `apps/ace-app/`. This includes the SvelteKit frontend, Tauri desktop adapter, and app-facing IPC tests.
**Status:** **Open work only.** This document does not list completed findings. End-to-end native test gaps remain under **AA-07**. Completed code is not a new task only because native validation is pending.

> **Pre-release policy.** Ace Blocker has not been publicly released. Old compatibility formats do not need support. Keep the engine and watchdog independent. Keep locks, anti-tampering, capability checks, remoc Named Pipes, and installer ownership unchanged. Use typed, explicit errors. Do not encode errors as successful or empty primitive values.

## A. Current scope and architecture

The desktop WebView uses Svelte 5 and SvelteKit. It is unprivileged. The Tauri Rust adapter connects to a separate privileged engine through capability-protected remoc. The watchdog and installer own their service and lifecycle work. The external Ace Server HTTP client is not part of this local RPC.

The original audit contained 19 findings. **Ten still need work or acceptance checks:** two P1 (AA-07, AA-08), seven P2 (AA-12 through AA-18), and one P3 (AA-19). Completed P0/P1 work was removed from the active list. The duplicate initial request in AA-17 was fixed. Only response-ordering work remains for AA-17.

| Layer | Ownership and flow | Constraints |
| --- | --- | --- |
| `src/routes/` | Page loads, route-local UI/editor state, user workflows | Policy authorization must not move into components; failures must not masquerade as empty data |
| `src/lib/components/ace/` | Ace-owned domain presentation, navigation rail, status strip, inspector | Use approved visual identity; no extra global “UI framework” |
| `src/lib/deltas/` | Stages schedule, rule, pack-link and lock changes; makes existing engine calls | Does not own privileged lock policy or engine transactions |
| `src/lib/stores/` | Engine liveness, dashboard snapshot, account/legal gate, onboarding, pack sync | Keep independent lifecycles; avoid global synchronization buses |
| `src/lib/engine-client/` | TypeScript consumer facade, parameter mapping, Tauri invocation and streams | May not read capability tokens or establish named pipes |
| `src-tauri/src/lib.rs` | Reads installation capability in Rust, registers Tauri commands, forwards typed remoc calls and progress | Unprivileged desktop adapter only; neither service owner nor policy authority |
| `crates/ace-engine/` (interface only) | Typed RPC, authentication, state and policy enforcement | Remains LocalSystem service, separate from UI/watchdog/installer |
| `src/lib/paraglide/` | Generated localization artifacts | Do not hand-edit generated output |

## B. Prioritized open backlog

| ID | Priority | Status | Area | Evidence (paths below relative to `apps/ace-app/`) | Problem | Proposed refactoring | Risk | Effort |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AA-07 | P1 | Blocked: native VM unavailable | Native integration testing | `e2e/*.spec.ts`, `src/lib/engine-client/tauri-rpc.test.ts` | Browser/static and mocked tests do not prove Named Pipe or Tauri security flows | Add native Windows test gate using disposable VM | Medium | Large |
| AA-08 | P1 | Verification pending | Theme persistence | `src/routes/settings/+page.svelte:14-51` | Hydration/write-order fix is implemented but untested with delayed store responses | Add deterministic deferred-load and rapid-change persistence tests | Low | Small |
| AA-12 | P2 | Open — confirmed | Store refresh concurrency | `src/lib/stores/engine.svelte.ts:15-34`, `dashboard.svelte.ts:39-81` | Overlapping polls may apply stale snapshots; engine timer has no disposal | Coalesce or sequence refreshes; own teardown | Low | Medium |
| AA-13 | P2 | Open — needs characterization | IPC stream lifecycle | `src/lib/engine-client/tauri-rpc.ts:52-117` | Queue can grow without bound; no explicit cancellation when consumer exits | Characterize production stream lifetime; bound/backpressure or cancellation with existing primitives | Medium | Medium |
| AA-14 | P2 | Open — confirmed | Dead UI implementation | `src/lib/components/AppSidebar.svelte`, `PageCard.svelte`, `src/routes/network-blocks/_components/SectionLock.svelte` | Components have no non-generated source references after shell rebuild | Confirm dynamic consumers, remove unused files/imports in small batch | Low | Medium |
| AA-15 | P2 | Open — confirmed | Tooling/config | `vite.config.js:19-24`, `tsconfig.json:1-27` | SvelteKit reports deprecated alias and overwritten paths | Migrate alias config without breaking `$engine-client` imports | Medium | Small |
| AA-16 | P2 | Open — needs characterization | Keyboard/accessibility | `src/lib/components/ace/structural/Inspector.svelte:23-60`, `_BlockModal.svelte:245-263` | Fixed inspector does not define opening focus/return-focus behavior | Characterize keyboard path; add explicit focus lifecycle without falsely making it modal | Low | Medium |
| AA-17 | P2 | Partially completed | Site Packs data ownership | `src/routes/site-packs/+page.svelte:50-68` | Duplicate route fetch was removed; overlapping refreshes can still apply stale responses | Sequence fetch results and protect retained selection | Low | Small |
| AA-18 | P2 | Open — needs characterization | Delta IDs after save | `src/lib/deltas/schedule-delta.svelte.ts:180-199,406-442` | Newly added entries retain temporary IDs in post-save baseline | Refresh canonical IDs or close/dispose draft after successful save | Medium | Small |
| AA-19 | P3 | Open — confirmed | Stale artifacts | `docs/engine-rest-api.md`, `src/lib/stores/syncing-store.svelte.ts:134-151`, `src/lib/deltas/site-packs-connection-delta.svelte.ts:1-12` | REST/SSE and RocksDB-era descriptions remain after IPC/storage changes | Retire/retitle only confirmed obsolete documentation/comments | Low | Small |

## C. Detailed open findings

### AA-07 — Test the real native boundary, not only mocked Tauri calls

- **Priority/status:** P1 / verified coverage gap. **Evidence:** `e2e/getting-started.spec.ts`, `e2e/help.spec.ts` are current Playwright browser suites; `src/lib/engine-client/tauri-rpc.test.ts` mocks Tauri invocation; `docs/IPC_ARCHITECTURE.md:53-72` and `docs/ai/TECH_DEBT_BACKLOG.md:31-39` document the missing Windows gate.
- **Current behavior/problem:** Unit and static browser tests do not verify installation capability handling, Named Pipe ACLs, real remoc errors and events, service restart, protected operations, or uninstall checks.
- **Approach:** Use **TD-001** for shared engine and installer work. Add repeatable native tests for successful and failed app actions on a disposable Windows VM. Keep privileged service tests with the teams that own them.
- **Alternatives:** Reintroducing obsolete HTTP-mocked browser tests is rejected.
- **Dependencies/constraints:** Respect independent services and install-time ACLs; no production credential leakage into tests.
- **Risk/acceptance:** Medium. Native UI→Tauri→remoc success, unauthorized modification, offline/reconnect, stream completion/failure and locked rejection are verifiably exercised.
- **Tests:** The missing test suite must include real WebView shutdown-denial and authorized installer maintenance, locked schedule edits, partial-save reconciliation and stream failure; record environment and executed outcomes. Real native service/GUI cases have not yet been run; browser smoke tests and Rust adapter unit tests do not substitute for this gate.

### AA-08 — Verify theme hydration and serialized persistence under races

- **Priority/status:** P1 / **implementation completed, acceptance tests pending**. **Evidence:** `src/routes/settings/+page.svelte:14-51` now hydrates without persisting the default theme, guards against overwriting a user change during load, and serializes user-originated writes.
- **Remaining work:** Write tests for deferred Tauri store `load/get/set/save`. Verify delayed initial reads, user changes before hydration, and fast consecutive choices. The last user choice must not be lost. Writes must remain in order.
- **Approach:** Add focused component and store tests with controlled promise completion. Fix only reproduced defects. Do not restore automatic default-theme writes. Do not add a global theme store.
- **Risk/acceptance:** Low. No write occurs solely from initial hydration; saved non-default theme loads correctly; rapid changes persist the final user selection even when store operations resolve asynchronously.
- **Tests:** Delayed `load/get`, failure/retry, user change before hydration and multiple rapid `set/save` calls.

### AA-12 — Coordinate dashboard/liveness polling and disposal

- **Priority/status:** P2 / verified implementation pattern. **Evidence:** `src/lib/stores/engine.svelte.ts:11-34` starts a 10 s timer without retaining a cancellation handle; `src/lib/stores/dashboard.svelte.ts:39-81` writes snapshots with no sequence check; `src/lib/components/ace/domain/EngineState.svelte:25-31` triggers 5 s dashboard refreshes (and cleans that component timer).
- **Current behavior/problem:** An earlier slow response can overwrite a new snapshot. Store-owned timers have no clear lifetime. Measure polling cost before you replace it with events.
- **Approach:** Use single-flight or request sequence checks in each existing store. Give the engine timer an explicit start and stop owner. Skip hidden or offline work only if the UI can accept it.
- **Alternatives:** Do not create a generic global event bus or another RPC transport for polling.
- **Dependencies/constraints:** Preserve timely enforcement display and recover when engine restarts.
- **Risk/acceptance:** Low. Stale responses never replace new ones; no duplicate persistent timers; status updates after reconnect.
- **Tests:** Deferred-promise out-of-order tests, mount/dispose and visibility/resume tests.

### AA-13 — Define bounded/cancellable site-pack stream lifecycle

- **Priority/status:** P2 / candidate. **Evidence:** `src/lib/engine-client/tauri-rpc.ts:52-77,92-117` appends each event to `parsed: unknown[]`; the generator drains when iterated but no maximum queue is enforced and no explicit cancel is supplied to the native command. Cursor-based compaction already avoids per-event O(n) `shift` (cross-reference previously addressed **TD-016**).
- **Current behavior/problem:** Slow or abandoned iterators can leave progress events and native streams active after the user-visible operation ends. Measure real event frequency before you claim a memory defect.
- **Approach:** Measure event rates and stream lifetime first. Use current Tauri/remoc cancellation or a bounded queue when possible. Coalesce progress only when semantics permit it. Never discard terminal or error events. Do not add custom framing.
- **Alternatives:** Arbitrarily dropping events or stopping on stream close without terminal status is unsafe.
- **Dependencies/constraints:** Existing terminal-event semantics (already implemented), AA-07 native tests.
- **Risk/acceptance:** Medium. Explicit abandonment does not retain an unbounded queue; terminal state and ordering survive.
- **Tests:** Slow/abandoned iterator simulation and native cancellation/reconnect tests.

### AA-14 — Remove confirmed unused legacy UI implementations

- **Priority/status:** P2 / verified lexical nonuse, subject to final consumer audit. **Evidence:** No non-generated source references found for `src/lib/components/AppSidebar.svelte`, `PageCard.svelte`, `src/routes/network-blocks/_components/SectionLock.svelte` or `src/lib/components/_DebugLicenseSelector.svelte`. `DevToolsSheet.svelte` and `SidebarAccountWidget.svelte` appear to be referenced by the unused AppSidebar, not the active `+layout.svelte:5-13,61-73`.
- **Current behavior/problem:** Old shell and editor components remain with NavigationRail and Inspector. They add review and dependency cost. Do not delete the complete `components/ui/` library. Other parts still use it.
- **Approach:** Check dynamic imports, build references, indirect users, and developer-only paths. Confirm the UI migration plan. Remove only unused components and imports. Keep active shared primitives.
- **Alternatives:** Blanket deletion of shadcn-derived UI components before all usages migrate — rejected.
- **Dependencies/constraints:** No functional UI regressions. Cross-reference approved UI identity.
- **Risk/acceptance:** Low. All removed files have no source consumers; all application routes still compile and render.
- **Tests:** `rg` reference checks before/after; `bun run check`, unit tests, build and UI smoke screenshots.

### AA-15 — Resolve SvelteKit alias warnings without API churn

- **Priority/status:** P2 / verified. **Evidence:** `vite.config.js:19-24` uses `sveltekit({alias: ...})`; `tsconfig.json:1-27` extends SvelteKit configuration. Executed `bun run check` and `bun run build` both emit `config_option_deprecated_alias` and `tsconfig_invalid` (“paths was overwritten”).
- **Current behavior/problem:** The build passes. SvelteKit warnings show possible future upgrade failure and inconsistent alias type checking.
- **Approach:** Use the alias configuration supported by the installed SvelteKit version. Keep source imports and types unchanged. Make tooling changes separately from runtime changes.
- **Alternatives:** Silencing warnings without correcting their cause — rejected.
- **Dependencies/constraints:** None; generated Paraglide output remains generated.
- **Risk/acceptance:** Medium. No alias warnings in Svelte check/build; `$lib` and `$engine-client` resolve consistently.
- **Tests:** `bun run check`, `bun run test:unit`, `bun run build`.

### AA-16 — Specify inspector keyboard/focus lifecycle

- **Priority/status:** P2 / candidate pending actual keyboard test. **Evidence:** `src/lib/components/ace/structural/Inspector.svelte:23-60` provides a fixed `aside role="region"` with close action, but no opening/return focus management. The caller binds Escape globally in `_BlockModal.svelte:245-256`.
- **Current behavior/problem:** Keyboard and screen-reader users can lose focus when the inspector opens or closes. The inspector is **not automatically a modal**. A focus trap or inert background could change the approved behavior.
- **Approach:** Test Tab, Shift+Tab, and Escape in Network Blocks and Site Packs. Give each opener a focus target. Restore focus where needed. Select region or dialog semantics based on real interaction.
- **Alternatives:** Convert every inspector into a modal dialog for easier focus trapping — rejected absent UX justification.
- **Dependencies/constraints:** Respect `docs/ui-identity.md`; avoid purely subjective style changes.
- **Risk/acceptance:** Low. Screen reader announces inspector context; keyboard can navigate and close without getting lost; return focus is deterministic.
- **Tests:** Playwright keyboard/a11y checks on representative desktop viewports; manual Tauri light/dark checks.

### AA-17 — Finish authoritative Site Packs fetch lifecycle

- **Priority/status:** P2 / **partially completed**. **Evidence:** `src/routes/site-packs/+page.ts` no longer performs the redundant initial RPC; `src/routes/site-packs/+page.svelte:50-68` owns the fetch and explicit loading/error/retry state. It still has no request-sequence or single-flight guard.
- **Remaining work:** Delete, resync, initial load, and user refresh requests can finish out of order. An old response can replace new Site Pack data or the selected item's identity.
- **Approach:** Keep collection fetches in this component. Add a request sequence check or an existing narrow mechanism. Update `selectedPack` only from the newest accepted data. Do not show old errors or loading states.
- **Alternatives:** No global Site Packs cache or additional data layer merely for this route.
- **Dependencies/constraints:** Preserve explicit error-vs-empty behavior and `syncStore` progress.
- **Risk/acceptance:** Low. Late responses never replace a newer snapshot; selection survives when its ID remains present, and clears correctly when deleted.
- **Tests:** Deferred overlapping results, delete/resync and selection restoration tests.

### AA-18 — Reconcile temporary schedule IDs after persisted adds

- **Priority/status:** P2 / candidate impact (normal modal currently closes on successful save). **Evidence:** `src/lib/deltas/schedule-delta.svelte.ts:180-199` stages `newTempId()` and inserts it into `currentState`. The add RPC response is not reconciled at `406-435`; `initialEntries` is replaced with `currentState` at `438-442`.
- **Current behavior/problem:** A reused delta can mistake temporary IDs for stored IDs after a successful save. The current editor closes after save (`_BlockModal.svelte:217-229`). This reduces risk but does not prove it is absent.
- **Approach:** Either make the draft single-use and dispose it after saving, or reload canonical stored entries. Do not add an adapter for old temporary IDs.
- **Alternatives:** Generate client UUIDs and assume the server adopted them — rejected.
- **Dependencies/constraints:** Align with the implemented partial-save reconciliation and authoritative refresh strategy.
- **Risk/acceptance:** Medium. Successful add yields canonical IDs before subsequent edit/delete, or instance cannot be reused.
- **Tests:** Two sequential edits using the same delta instance; partial failure; delete after successful add.

### AA-19 — Remove obsolete REST/SSE-era descriptions after consumer check

- **Priority/status:** P3 / verified stale terminology. **Evidence:** `docs/engine-rest-api.md` remains in app docs after remoc migration; `syncing-store.svelte.ts:134-151` describes an SSE stream although it now consumes typed Tauri events; `site-packs-connection-delta.svelte.ts:1-12` mentions RocksDB and old “Network List” terminology.
- **Current behavior/problem:** Old HTTP terms can make readers think the app still uses local REST control. Current control uses typed RPC.
- **Approach:** Check whether the old document is still authoritative. Archive, remove, or mark it as historical. Update code comments for typed IPC and current storage ownership. Keep valid external Ace Server HTTP documents.
- **Alternatives:** Delete all HTTP references everywhere — incorrect because remote Ace Server and browser updates legitimately use HTTPS.
- **Dependencies/constraints:** None.
- **Risk/acceptance:** Low. No documentation claims that Ace App uses a local REST control server; valid remote API docs remain.
- **Tests:** Documentation link and terminology search; no runtime tests required.

## D. Recommended execution order

| Batch | Active findings | Acceptance focus |
| --- | --- | --- |
| 1 | AA-08, AA-17 | Deterministic deferred-store and stale-response tests; patch only remaining race bugs |
| 2 | AA-12, AA-18 | Polling ownership, latest-snapshot ordering and post-save ID lifecycle |
| 3 | AA-13, AA-16 | Stream cancellation/bounds and keyboard/focus behavior |
| 4 | AA-14, AA-15, AA-19 | Consumer-checked dead-code cleanup, SvelteKit aliases and obsolete documentation |
| Native gate | **AA-07** | Disposable Windows VM: app↔remoc IPC, credential/ACL boundaries, locked operations, hostile WebView shutdown denial, installer maintenance, reconnect and stream error paths |

Do not use this audit to approve a broad rewrite, a global state framework, or a custom IPC transport. Keep each batch small enough for independent review.

## E. Verification and blocking conditions

**Checks already passed on 2026-10-22 after implementation:** `bun run check` (0 Svelte diagnostic errors/warnings); `bun run test:unit` (82 tests / 10 suites); `bun run build`; `bun run test:e2e` (4 browser-only tests); `cargo fmt -p ace-app -- --check`; `cargo check -p ace-app`; and `cargo test -p ace-app --lib` (2 Rust tests).

**Still outstanding:** AA-07 native end-to-end tests have **not** run. CodexPro CAP had no registered disposable Windows VM image. The `vm images` command returned an empty list. Rust unit tests cover Tauri WebView-side `shutdown` denial. They do **not** prove security in an installed system with hostile WebView calls or locked and unlocked states. The native tests must also cover partial saves, locked schedules, real remoc serialization and reconnect, account/legal error recovery, stream completion and EOF, and installer maintenance/uninstall. Use `docs/ai/ACE_APP_NATIVE_TEST_GATE.md` as the test plan. Do not report this gate as passed without recorded results.

AA-08 still needs deferred-theme tests. AA-17 still needs stale-response tests. The 82-test result does **not** satisfy either item.

**Known tooling debt:** SvelteKit still reports `config_option_deprecated_alias` and `tsconfig_invalid`. This is **AA-15 / TD-017**. It remains open although type checking and builds passed.

For each future batch, run `bun run check`, `bun run test:unit`, and `bun run build`. Add focused regression tests. Get an independent diff review. Run native service and security tests only in a disposable Windows VM.

## F. Boundaries and cross-references

- **No legacy migrations or compatibility layers** for old, unpublished app, storage, and IPC formats. Keep current active development data and current security behavior.
- Do not let the UI stop the privileged engine. Installer maintenance uses a separate authenticated path.
- Do not add local REST/HTTP fallback, a generic Svelte transaction manager, or unreviewed lock and watchdog changes.
- **Related guidance:** root `AGENTS.md`, `apps/ace-app/AGENTS.md`, `docs/ai/REFACTORING_PLAYBOOK.md`, `docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`, `docs/ai/DOMAIN_TYPES.md`, `docs/ai/REVIEW_GATES.md`, `docs/ai/TECH_DEBT_BACKLOG.md`, `docs/IPC_ARCHITECTURE.md`, `apps/ace-app/docs/ui-identity.md`.
- **Cross-reference:** TD-001 covers native IPC and installer checks. TD-017 covers alias warnings. AA-13 only covers stream lifetime and queue bounds. It does **not** reopen the `Array.shift()` performance issue fixed under TD-016.

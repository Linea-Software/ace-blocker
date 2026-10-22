# Ace App — Outstanding Refactoring Audit

**Original audit:** 2026-10-22 · **Last pruned:** 2026-10-22

**Repository:** `C:\Projects\ace-project\ace-blocker`

**Scope:** `apps/ace-app/`, including SvelteKit frontend, Tauri desktop adapter and app-facing IPC verification.
**Status:** **Open work only.** Fully implemented findings have been removed from the active backlog and detailed findings. Already implemented code is not a new task merely because end-to-end native validation is pending; all cross-cutting native requirements are consolidated under **AA-07**.

> **Pre-release policy.** Ace Blocker is not publicly released and has no legacy compatibility requirement. Keep engine/watchdog independence, lock/anti-tamper behavior, capability security, remoc Named Pipes and installer ownership intact. Errors must remain typed and explicit, never primitive success/empty sentinels.

## A. Current scope and architecture

The desktop WebView is an unprivileged Svelte 5/SvelteKit UI. Its Tauri Rust adapter uses a capability-protected remoc channel to a separate privileged engine service. The watchdog and installer retain their own service/lifecycle responsibilities. The external Ace Server HTTP client is separate from this local RPC.

The original audit contained 19 findings; **10 still require work or acceptance verification**: 2 P1 (AA-07, AA-08), 7 P2 (AA-12 through AA-18), and 1 P3 (AA-19). The completed P0/P1 implementations were removed from this document rather than preserved as a stale to-do list. The P2 AA-17 duplicate initial request is also already fixed; only response-ordering work remains.

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
- **Current behavior/problem:** Unit/static browser tests cannot prove installation capability handling, Named Pipe ACLs, real remoc errors/events, service restart, lock-protected operations, or uninstall gating.
- **Approach:** Cross-reference **TD-001** instead of duplicating engine/installer audit work. Establish a repeatable native test harness for app-facing successful and failure flows on a disposable Windows VM; keep privileged service tests with the owning team.
- **Alternatives:** Reintroducing obsolete HTTP-mocked browser tests is rejected.
- **Dependencies/constraints:** Respect independent services and install-time ACLs; no production credential leakage into tests.
- **Risk/acceptance:** Medium. Native UI→Tauri→remoc success, unauthorized modification, offline/reconnect, stream completion/failure and locked rejection are verifiably exercised.
- **Tests:** The missing test suite must include real WebView shutdown-denial and authorized installer maintenance, locked schedule edits, partial-save reconciliation and stream failure; record environment and executed outcomes. Real native service/GUI cases have not yet been run; browser smoke tests and Rust adapter unit tests do not substitute for this gate.

### AA-08 — Verify theme hydration and serialized persistence under races

- **Priority/status:** P1 / **implementation completed, acceptance tests pending**. **Evidence:** `src/routes/settings/+page.svelte:14-51` now hydrates without persisting the default theme, guards against overwriting a user change during load, and serializes user-originated writes.
- **Remaining work:** The deferred Tauri store `load/get/set/save` tests have not been written. A delayed initial read, a user choice before hydration, and rapid consecutive choices must not reset the user's last intended theme or persist writes out of order.
- **Approach:** Add focused component/store test coverage with controlled promise resolution; correct only reproduced problems. Do not restore default-theme automatic writes or introduce a global theme store.
- **Risk/acceptance:** Low. No write occurs solely from initial hydration; saved non-default theme loads correctly; rapid changes persist the final user selection even when store operations resolve asynchronously.
- **Tests:** Delayed `load/get`, failure/retry, user change before hydration and multiple rapid `set/save` calls.

### AA-12 — Coordinate dashboard/liveness polling and disposal

- **Priority/status:** P2 / verified implementation pattern. **Evidence:** `src/lib/stores/engine.svelte.ts:11-34` starts a 10 s timer without retaining a cancellation handle; `src/lib/stores/dashboard.svelte.ts:39-81` writes snapshots with no sequence check; `src/lib/components/ace/domain/EngineState.svelte:25-31` triggers 5 s dashboard refreshes (and cleans that component timer).
- **Current behavior/problem:** Delayed earlier responses can overwrite newer snapshots when refreshes overlap, and store-owned background timer lifetimes are implicit. Polling traffic should be assessed before replacing it with event plumbing.
- **Approach:** Single-flight or monotonic request sequence in each existing store; explicit start/stop ownership for engine timer; skip redundant work while hidden/offline if UX tolerates it.
- **Alternatives:** Do not create a generic global event bus or another RPC transport for polling.
- **Dependencies/constraints:** Preserve timely enforcement display and recover when engine restarts.
- **Risk/acceptance:** Low. Stale responses never replace new ones; no duplicate persistent timers; status updates after reconnect.
- **Tests:** Deferred-promise out-of-order tests, mount/dispose and visibility/resume tests.

### AA-13 — Define bounded/cancellable site-pack stream lifecycle

- **Priority/status:** P2 / candidate. **Evidence:** `src/lib/engine-client/tauri-rpc.ts:52-77,92-117` appends each event to `parsed: unknown[]`; the generator drains when iterated but no maximum queue is enforced and no explicit cancel is supplied to the native command. Cursor-based compaction already avoids per-event O(n) `shift` (cross-reference previously addressed **TD-016**).
- **Current behavior/problem:** When consumers fall behind or abandon iteration, progress events and native streaming work can outlive the visible operation. Whether this causes material memory use depends on real event frequency.
- **Approach:** First measure/corroborate event rates and stream lifetimes. Prefer existing Tauri/remoc cancellation or bounded coalescing **only where progress semantics permit**; terminal/error events must never be discarded. Avoid custom framing/transport.
- **Alternatives:** Arbitrarily dropping events or stopping on stream close without terminal status is unsafe.
- **Dependencies/constraints:** Existing terminal-event semantics (already implemented), AA-07 native tests.
- **Risk/acceptance:** Medium. Explicit abandonment does not retain an unbounded queue; terminal state and ordering survive.
- **Tests:** Slow/abandoned iterator simulation and native cancellation/reconnect tests.

### AA-14 — Remove confirmed unused legacy UI implementations

- **Priority/status:** P2 / verified lexical nonuse, subject to final consumer audit. **Evidence:** No non-generated source references found for `src/lib/components/AppSidebar.svelte`, `PageCard.svelte`, `src/routes/network-blocks/_components/SectionLock.svelte` or `src/lib/components/_DebugLicenseSelector.svelte`. `DevToolsSheet.svelte` and `SidebarAccountWidget.svelte` appear to be referenced by the unused AppSidebar, not the active `+layout.svelte:5-13,61-73`.
- **Current behavior/problem:** Legacy alternate shell/editor surfaces remain alongside the new NavigationRail/Inspector and impose review and dependency cost. Presence alone is not sufficient to delete the whole legacy `components/ui/` library, whose other consumers are active.
- **Approach:** Check dynamic imports, build references, indirect component users, dev-only references and design migration plan. Remove only demonstrably unreachable components and their unused imports; keep active shared primitives.
- **Alternatives:** Blanket deletion of shadcn-derived UI components before all usages migrate — rejected.
- **Dependencies/constraints:** No functional UI regressions. Cross-reference approved UI identity.
- **Risk/acceptance:** Low. All removed files have no source consumers; all application routes still compile and render.
- **Tests:** `rg` reference checks before/after; `bun run check`, unit tests, build and UI smoke screenshots.

### AA-15 — Resolve SvelteKit alias warnings without API churn

- **Priority/status:** P2 / verified. **Evidence:** `vite.config.js:19-24` uses `sveltekit({alias: ...})`; `tsconfig.json:1-27` extends SvelteKit configuration. Executed `bun run check` and `bun run build` both emit `config_option_deprecated_alias` and `tsconfig_invalid` (“paths was overwritten”).
- **Current behavior/problem:** Build currently passes, but warnings indicate a future upgrade break and potential alias typechecking drift.
- **Approach:** Update import alias mechanism per installed SvelteKit, preserving existing source imports and types; isolate config changes from runtime work.
- **Alternatives:** Silencing warnings without correcting their cause — rejected.
- **Dependencies/constraints:** None; generated Paraglide output remains generated.
- **Risk/acceptance:** Medium. No alias warnings in Svelte check/build; `$lib` and `$engine-client` resolve consistently.
- **Tests:** `bun run check`, `bun run test:unit`, `bun run build`.

### AA-16 — Specify inspector keyboard/focus lifecycle

- **Priority/status:** P2 / candidate pending actual keyboard test. **Evidence:** `src/lib/components/ace/structural/Inspector.svelte:23-60` provides a fixed `aside role="region"` with close action, but no opening/return focus management. The caller binds Escape globally in `_BlockModal.svelte:245-256`.
- **Current behavior/problem:** Keyboard/screen-reader users may remain focused behind the inspector or lose their place on close. It is a side inspector, **not automatically a modal**, so indiscriminate focus traps or inert backgrounds could violate the approved interaction model.
- **Approach:** Test actual tab/Shift+Tab/Escape sequences in both Network Blocks and Site Packs. Give each opener an explicit focus target and restore focus where appropriate; choose region/dialog semantics based on real interaction behavior.
- **Alternatives:** Convert every inspector into a modal dialog for easier focus trapping — rejected absent UX justification.
- **Dependencies/constraints:** Respect `docs/ui-identity.md`; avoid purely subjective style changes.
- **Risk/acceptance:** Low. Screen reader announces inspector context; keyboard can navigate and close without getting lost; return focus is deterministic.
- **Tests:** Playwright keyboard/a11y checks on representative desktop viewports; manual Tauri light/dark checks.

### AA-17 — Finish authoritative Site Packs fetch lifecycle

- **Priority/status:** P2 / **partially completed**. **Evidence:** `src/routes/site-packs/+page.ts` no longer performs the redundant initial RPC; `src/routes/site-packs/+page.svelte:50-68` owns the fetch and explicit loading/error/retry state. It still has no request-sequence or single-flight guard.
- **Remaining work:** Overlapping post-delete, resync, initial and user-triggered refreshes can settle out of order. An older response can overwrite newer Site Pack state or selected-item identity.
- **Approach:** Keep the component as the single owner of collection retrieval. Add a per-request sequence guard (or equivalent existing narrow mechanism), reconcile `selectedPack` only from the latest accepted data, and avoid stale error/loading flags.
- **Alternatives:** No global Site Packs cache or additional data layer merely for this route.
- **Dependencies/constraints:** Preserve explicit error-vs-empty behavior and `syncStore` progress.
- **Risk/acceptance:** Low. Late responses never replace a newer snapshot; selection survives when its ID remains present, and clears correctly when deleted.
- **Tests:** Deferred overlapping results, delete/resync and selection restoration tests.

### AA-18 — Reconcile temporary schedule IDs after persisted adds

- **Priority/status:** P2 / candidate impact (normal modal currently closes on successful save). **Evidence:** `src/lib/deltas/schedule-delta.svelte.ts:180-199` stages `newTempId()` and inserts it into `currentState`. The add RPC response is not reconciled at `406-435`; `initialEntries` is replaced with `currentState` at `438-442`.
- **Current behavior/problem:** Reusing this delta instance after a successful save may treat temporary IDs as persisted server IDs. The current editor closes after save (`_BlockModal.svelte:217-229`), reducing but not proving absence of risk.
- **Approach:** Choose either single-use draft lifetime (enforce disposal) or refresh canonical persisted entries after success. Do not keep a fake historical-ID adapter.
- **Alternatives:** Generate client UUIDs and assume the server adopted them — rejected.
- **Dependencies/constraints:** Align with the implemented partial-save reconciliation and authoritative refresh strategy.
- **Risk/acceptance:** Medium. Successful add yields canonical IDs before subsequent edit/delete, or instance cannot be reused.
- **Tests:** Two sequential edits using the same delta instance; partial failure; delete after successful add.

### AA-19 — Remove obsolete REST/SSE-era descriptions after consumer check

- **Priority/status:** P3 / verified stale terminology. **Evidence:** `docs/engine-rest-api.md` remains in app docs after remoc migration; `syncing-store.svelte.ts:134-151` describes an SSE stream although it now consumes typed Tauri events; `site-packs-connection-delta.svelte.ts:1-12` mentions RocksDB and old “Network List” terminology.
- **Current behavior/problem:** AI/code readers can confuse obsolete HTTP terminology with current local RPC design.
- **Approach:** Confirm the old document is no longer linked as authoritative, archive/remove it or relabel as historical; update comments to match current typed IPC and real storage ownership. Keep actual Ace Server HTTP documentation separate.
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

Do not convert this audit into a broad rewrite, a new global state framework or custom IPC transport. Keep each batch independently reviewable.

## E. Verification and blocking conditions

**Checks already passed on 2026-10-22 after the implementation work:** `bun run check` (0 Svelte diagnostic errors/warnings), `bun run test:unit` (82 tests / 10 suites), `bun run build`, `bun run test:e2e` (4 browser-only tests), `cargo fmt -p ace-app -- --check`, `cargo check -p ace-app` and `cargo test -p ace-app --lib` (2 Rust tests).

**Still outstanding:** AA-07 native end-to-end checks have **not** run because CodexPro CAP had no registered disposable Windows VM image (`vm images` returned an empty list). The Tauri WebView-side `shutdown` restriction has Rust unit tests, **not installed-system hostile-WebView or locked/unlocked proof**. The native gate must also cover partial-save failures, locked-schedule parity, actual remoc serialization/reconnection, account/legal error recovery, stream completion/EOF and installer maintenance/uninstall invariants. The detailed native test plan lives in `docs/ai/ACE_APP_NATIVE_TEST_GATE.md`; do not duplicate it or mark it passed without recorded results.

AA-08's deferred-theme tests and AA-17's stale-response tests are separately outstanding and **not** satisfied by the general 82-test result.

**Known tooling debt:** SvelteKit still reports `config_option_deprecated_alias` and `tsconfig_invalid`; this belongs to **AA-15 / TD-017**, despite successful typecheck/build.

For each future batch, run `bun run check`, `bun run test:unit` and `bun run build`; add focused regression tests, independently review the diff, and run native service/security tests only in a disposable Windows VM.

## F. Boundaries and cross-references

- **No legacy migrations or compatibility layers** for unshipped application/storage/IPC formats; preserve current active development data and current security behavior.
- No UI-controlled shutdown of the privileged engine. Installer maintenance is a separate authenticated path.
- No local REST/HTTP control fallback, new generic transaction manager in Svelte, or unreviewed change to lock and watchdog policies.
- **Related guidance:** root `AGENTS.md`, `apps/ace-app/AGENTS.md`, `docs/ai/REFACTORING_PLAYBOOK.md`, `docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`, `docs/ai/DOMAIN_TYPES.md`, `docs/ai/REVIEW_GATES.md`, `docs/ai/TECH_DEBT_BACKLOG.md`, `docs/IPC_ARCHITECTURE.md`, `apps/ace-app/docs/ui-identity.md`.
- **Cross-reference:** TD-001 covers native IPC/installer verification; TD-017 covers alias warnings. Previously addressed TD-016's queue `shift` issue is **not** reopened by AA-13, which targets only stream lifecycle and boundedness.

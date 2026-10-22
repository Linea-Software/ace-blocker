# Ace App — Refactoring Audit

**Audit date:** 2026-10-22
**Repository:** `C:\Projects\ace-project\ace-blocker`
**Scope:** `apps/ace-app/` (frontend, Tauri adapter, tests and configuration). Engine code was inspected only to confirm the existing control-plane interface.
**Baseline:** `main` at `c865a15`; Git working tree clean at start and before writing this document.
**Status:** Audit only. No application code, dependencies or IPC contracts changed.

> **Binding pre-release policy.** Ace Blocker has not been publicly released. There is **no requirement for backward compatibility** with historical databases, obsolete application or IPC formats, abandoned internal APIs, or previous development versions. Legacy-only code and data-format adapters may be removed after confirming current consumers. This does **not** permit losing current active development data without explicit notice, weakening lock or anti-tamper behavior, bypassing authorization, relaxing engine/watchdog independence, or replacing remoc with REST. Failures must be expressed as explicit errors (Rust `Result<T,E>` / typed error carriers; no primitive error sentinels).

## A. Executive summary

The UI is Svelte 5/SvelteKit running in the unprivileged Tauri desktop process. Native Tauri commands forward to an authenticated remoc client over Windows Named Pipes; enforcement and authorization remain in the independent Ace Engine. The UI has an Ace-specific component layer, route-local editors, engine-client facade, and several application-wide reactive stores. The separate external Ace Server and any generated external client must **not** be confused with this local RPC facade.

**Highest-risk finding:** the Network Block save flow can report success after ignored RPC errors, and independently executing delta batches can partially persist user changes after an overall failure. The local schedule lock preview also has an overnight coverage logic error; the engine must remain the authority enforcing locks. The Svelte RPC facade erases important static types and manually aligns operation mappings with the Rust dispatch. Tests do not yet exercise actual Windows Tauri/remoc capability, service, or reconnection behavior.

**Audit outcome: 19 findings** — P0: **2**; P1: **9**; P2: **7**; P3: **1**. “Verified” means the problematic control flow or missing check was directly observed, **not** that an end-to-end defect was experimentally reproduced. “Candidate” means a plausible impact needs targeted characterization.

The existing suite passes (42 tests) and Svelte diagnostics are clean. Static compilation does not validate distributed failure paths or Windows security.

### Inventory and boundaries surveyed

The source inventory contains **269 non-generated `.ts`, `.svelte`, and `.rs` files** under `src/`, `src-tauri/`, and `e2e/` after excluding generated Paraglide, generated Svelte artifacts, dependencies and build output. This count includes the existing reusable legacy `components/ui/` component family; file counts are not evidence of debt. Surveyed: layouts/routes, Network Blocks and schedule editors, Resistance, Categories/Custom Lists, onboarding/help, settings/backup, shared Svelte stores/deltas, Tauri command registration, IPC facades, configuration, and test inventory.

## B. Architecture overview

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

Reference interfaces: `docs/IPC_ARCHITECTURE.md:8-42`; `apps/ace-app/src-tauri/src/lib.rs:25-69,83-97`; `apps/ace-app/src/lib/engine-client/tauri-rpc.ts:13-49,52-117`; `apps/ace-app/src/routes/+layout.svelte:11-73`.

### Backlog legend

- **P0:** Current correctness/data integrity or critical user behavior; gate other cleanup on these.
- **P1:** Significant reliability, security-boundary review or architectural maintainability.
- **P2:** Useful bounded robustness, maintenance or accessibility work.
- **P3:** Low-risk documentation/cleanup.
- **Verified:** concrete source-level issue observed. **Candidate:** behavior or severity awaits characterization. **Partially addressed:** existing mitigation is present but incomplete.
- **Risk** is the *implementation/regression* risk, not the severity.

## C. Prioritized refactoring backlog

| ID | Priority | Status | Area | Evidence (paths below relative to `apps/ace-app/`) | Problem | Proposed refactoring | Risk | Effort |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AA-01 | P0 | Verified | Network Block editor | `src/routes/network-blocks/_BlockModal.svelte:197-215` | Ignored icon and safe-search mutation errors allow false success | Check every RPC result and gate completion | Medium | Small |
| AA-02 | P0 | Verified | Network Block/delta commit | `_BlockModal.svelte:149-235`; `src/lib/deltas/common.ts:32-56` | Concurrent independent commits do not form a transaction or coordinated rollback | Define coherent partial-commit/reconcile behavior; preserve lock checks | High | Large |
| AA-03 | P1 | Verified | Lock schedule preview | `src/lib/deltas/schedule-delta.svelte.ts:28-87,152-160` | Overnight periods can incorrectly pass the frontend strictness test | Use tested weekly coverage semantics, engine remains authoritative | High | Medium |
| AA-04 | P1 | Verified | TypeScript/remoc boundary | `src/lib/engine-client/sdk.ts:10-145`, `rpc-parameters.ts:1-151`, `tauri-rpc.ts:15-49` | Generic strings and `any` casts conceal RPC contract drift | Improve compile-time alignment with existing Rust contract, add contract tests | High | Large |
| AA-05 | P1 | Verified | Error contract | `tauri-rpc.ts:30-49`, `src/routes/network-blocks/+page.svelte:79-103` | RPC error shape and caller HTTP-era `response.status` assumptions differ | Canonical typed error mapping at facade and caller updates | Medium | Medium |
| AA-06 | P1 | Verified | Site Pack import | `src/routes/site-packs/_ImportPackModal.svelte:127-172` | Successful return from a stream with no terminal event is not treated as failure | Require COMPLETED/FAILED; explicitly error on premature EOF | Low | Small |
| AA-07 | P1 | Verified | Native integration testing | `e2e/*.spec.ts`, `src/lib/engine-client/tauri-rpc.test.ts` | Browser/static and mocked tests do not prove Named Pipe or Tauri security flows | Add native Windows test gate using disposable VM | Medium | Large |
| AA-08 | P1 | Candidate | Theme persistence | `src/routes/settings/+page.svelte:14-47` | Async read races initial reactive save of default theme | Hydrate before enabling writes; serialize persistence | Low | Small |
| AA-09 | P1 | Verified | Page loading/empty states | `src/routes/network-blocks/+page.ts:17-42`, `src/routes/site-packs/+page.ts:4-9` | RPC errors can yield an apparently valid empty collection | Distinguish offline/error/empty explicitly | Low | Small |
| AA-10 | P1 | Candidate | Tauri command surface | `src-tauri/src/lib.rs:41-52`, `src/lib/engine-client/sdk.ts:137`; `crates/ace-engine/src/service/api/bridge.rs:403-407` | Generic WebView-dispatchable operations include `shutdown` | Security-review allowed desktop calls; retain engine authorization | High | Medium |
| AA-11 | P1 | Partially addressed | Account/legal gate | `src/lib/stores/user-store.svelte.ts:85-166`, `src/routes/+layout.svelte:17-28` | Persisting read errors can leave legal gate in a loading state without explicit retry/error view | Preserve fail-closed gate, expose retriable loading failure and lifecycle | Medium | Medium |
| AA-12 | P2 | Verified | Store refresh concurrency | `src/lib/stores/engine.svelte.ts:15-34`, `dashboard.svelte.ts:39-81` | Overlapping polls may apply stale snapshots; engine timer has no disposal | Coalesce or sequence refreshes; own teardown | Low | Medium |
| AA-13 | P2 | Candidate | IPC stream lifecycle | `src/lib/engine-client/tauri-rpc.ts:52-117` | Queue can grow without bound; no explicit cancellation when consumer exits | Characterize production stream lifetime; bound/backpressure or cancellation with existing primitives | Medium | Medium |
| AA-14 | P2 | Verified | Dead UI implementation | `src/lib/components/AppSidebar.svelte`, `PageCard.svelte`, `src/routes/network-blocks/_components/SectionLock.svelte` | Components have no non-generated source references after shell rebuild | Confirm dynamic consumers, remove unused files/imports in small batch | Low | Medium |
| AA-15 | P2 | Verified | Tooling/config | `vite.config.js:19-24`, `tsconfig.json:1-27` | SvelteKit reports deprecated alias and overwritten paths | Migrate alias config without breaking `$engine-client` imports | Medium | Small |
| AA-16 | P2 | Candidate | Keyboard/accessibility | `src/lib/components/ace/structural/Inspector.svelte:23-60`, `_BlockModal.svelte:245-263` | Fixed inspector does not define opening focus/return-focus behavior | Characterize keyboard path; add explicit focus lifecycle without falsely making it modal | Low | Medium |
| AA-17 | P2 | Verified | Site Packs data ownership | `src/routes/site-packs/+page.ts:4-9`, `+page.svelte:39-76,155-182` | Page load fetch is unused; component fetches again and concurrent refreshes lack sequencing | Choose one owner and prevent stale refresh completion | Low | Medium |
| AA-18 | P2 | Candidate | Delta IDs after save | `src/lib/deltas/schedule-delta.svelte.ts:180-199,406-442` | Newly added entries retain temporary IDs in post-save baseline | Refresh canonical IDs or close/dispose draft after successful save | Medium | Small |
| AA-19 | P3 | Verified | Stale artifacts | `docs/engine-rest-api.md`, `src/lib/stores/syncing-store.svelte.ts:134-151`, `src/lib/deltas/site-packs-connection-delta.svelte.ts:1-12` | REST/SSE and RocksDB-era descriptions remain after IPC/storage changes | Retire/retitle only confirmed obsolete documentation/comments | Low | Small |

## D. Detailed findings

### AA-01 — Verify every Network Block mutation before acknowledging save

- **Priority/status:** P0 / verified. **Evidence:** `src/routes/network-blocks/_BlockModal.svelte:197-215,217-235`. `setNetworkBlockIcon()` is awaited without checking `error`; the `setNetworkBlockSafeSearch()` promise is included in `Promise.all` but its returned error object is ignored. The facade ordinarily resolves with `{error}` rather than rejecting (`tauri-rpc.ts:30-49`).
- **Current behavior/problem:** A failed engine mutation may still call `onSave` and close the inspector, falsely showing an applied policy. This is correctness-relevant when safe-search configuration was requested.
- **Approach:** Require explicit result checking for icon and safe search; propagate typed mutation failures before success UI. Do not perform policy writes from presentation callbacks outside the established facade. Prefer a single local save coordinator over individually scattered early returns.
- **Alternatives:** Throwing on all RPC calls globally is too broad; use the current result contract at this boundary first.
- **Dependencies/constraints:** Depends on AA-05 for long-term consistent errors; engine still owns authorization/locks.
- **Risk/acceptance:** Medium. A forced failure of either mutation keeps the editor open, presents an error, and never reports full success; actual successful writes still work.
- **Tests:** Mock RPC error returns for icon/safe search; assert `onSave` is not called. Native locked/error checks under AA-07.

### AA-02 — Resolve independently committed delta batches on partial failure

- **Priority/status:** P0 / verified. **Evidence:** `_BlockModal.svelte:149-215` concurrently starts rules, schedules, pack assignments and safe search. Each delta runs a separate `withRollback` (`src/lib/deltas/common.ts:32-56`), registers independent inverses and considers itself committed after its own success (`schedule-delta.svelte.ts:438-442`; `site-packs-connection-delta.svelte.ts:224-229`; `user-defined-rules-delta.svelte.ts:203-207`).
- **Current behavior/problem:** Failure of one concurrent batch cannot roll back another successful batch. Rename/icon may already be written even before these batches begin. Rollback errors are only logged while the original exception propagates. This can leave live engine state different from the still-open editor draft.
- **Approach:** First characterize partial-save semantics. Prefer sequential, independently checked groups with a fresh authoritative snapshot and explicit “some changes applied” recovery rather than promising unsupported atomicity. If true cross-resource atomicity is a required product guarantee, propose an explicitly approved engine-owned operation separately; **do not** build a distributed transaction manager in Svelte.
- **Alternatives:** Merely swapping `Promise.all` for sequential awaits reduces interleaving but does not create atomicity. Full client-side rollback is risky for locks, newly allocated server IDs and concurrent external writers.
- **Dependencies/constraints:** AA-01, AA-04/05, AA-18; strict enforcement and auth remain engine responsibilities.
- **Risk/acceptance:** High. Induced failure at every step yields a correct status (“all applied”, “partially applied”, or “none applied”), refreshes from engine when necessary, and does not falsely claim rollback of irreversible operations.
- **Tests:** Failure injection for each mutation ordering, successful earlier commits plus failed later commit, failed rollback, active lock, and reopen/reconcile. Windows native gate AA-07.

### AA-03 — Fix overnight lock-schedule coverage calculation

- **Priority/status:** P1 / verified source-level logic. **Evidence:** `src/lib/deltas/schedule-delta.svelte.ts:28-30,45-79,83-87,152-160`. It compares `currentPos = oldStart` directly with `oldEnd` on each selected weekday, without splitting overnight periods. `_SchedulePeriodEditor.svelte:42-46` explicitly accepts `endTime < startTime` as crossing midnight.
- **Current behavior/problem:** An original 22:00→02:00 interval has oldStart=1320, oldEnd=120. With zero new intervals, `currentPos < oldEnd` is false, so frontend `isStricterThan` incorrectly returns true for that day. This is a misleading preview; it must **not** be relied upon for lock enforcement.
- **Approach:** Reuse or align with the existing tested week-minute coverage implementation (`src/lib/utils/weekly-coverage.ts`), including weekday rollover and day-boundary semantics. First confirm the engine's authoritative schedule interpretation rather than independently redefining policy.
- **Alternatives:** Patch only the numerical `end < start` comparison; rejected because next-day weekday coverage and multi-interval unions must also be correct.
- **Dependencies/constraints:** Engine-authoritative policy; no loosening a locked schedule.
- **Risk/acceptance:** High. Overnight, weekday-wrap (Sunday→Monday), exact midnight, contiguous unions, empty coverage and normal daytime examples agree with engine decisions.
- **Tests:** Dedicated `schedule-delta.test.ts` table-driven lock coverage tests, plus native locked-edit rejection. Existing weekly-coverage utility tests alone do not exercise this validator.

### AA-04 — Remove unchecked type erasure at the Svelte/Tauri RPC seam

- **Priority/status:** P1 / verified. **Evidence:** `src/lib/engine-client/sdk.ts:10-145` contains pervasive `as any` assertions around `rpcRequest`; `tauri-rpc.ts:15-35` accepts arbitrary operation strings and maps loosely typed `path/query/body`; `rpc-parameters.ts:1-151` lists parameters manually; Rust accepts `op: String, args: Value` (`src-tauri/src/lib.rs:41-44`).
- **Current behavior/problem:** TS call sites appear strongly typed, but alignment between operation names, parameter mapping and Rust `bridge` is not checked at compile time. A renamed or differently shaped Rust request can be silently accepted by TS and fail only at runtime.
- **Approach:** Inventory actual RPC operations and call sites against the existing Rust remoc trait/bridge. Tighten narrow method signatures and use existing serialization type-generation tooling only if it can be maintained from one authoritative contract; otherwise add exhaustive checked operation/argument tests with explicit response shape assertions. Do not manually edit any **generated external Ace Server** client.
- **Alternatives:** Additional custom RPC protocol/generic transport framework is explicitly rejected; no return to REST/OpenAPI for local IPC.
- **Dependencies/constraints:** AA-05, AA-07; maintain IPC credential entirely in Rust.
- **Risk/acceptance:** High. Any mismatch in operation names or arguments is detected by compile-time or contract tests; all current operations and stream shapes are covered; no changes to network enforcement or security boundaries.
- **Tests:** TS per-operation argument mapping tests; Rust bridge dispatch coverage; round-trip in native Tauri context. `cargo check -p ace-app` after an actual implementation.

### AA-05 — Standardize explicit typed errors in the client facade

- **Priority/status:** P1 / verified. **Evidence:** `src/lib/engine-client/tauri-rpc.ts:30-49` normally returns `{data}` or `{error}` and does not return a Fetch `Response`; `src/routes/network-blocks/+page.svelte:83-95` reads `response?.status` for 403 handling; `src/routes/resistance-mode/+page.svelte:94-114` also accepts a response argument; `rpc-types.ts:1-20` does not model a response object.
- **Current behavior/problem:** HTTP-era status tests may be dead or misleading. Generic `unknown` error types cause repeated ad hoc `.code` checks and hide distinctions between transport, authorization and domain failures. Do not mistake a boolean, empty array or undefined data for the error itself.
- **Approach:** Keep a discriminated explicit result/error carrier; normalize engine `RpcError` fields once, and update affected components to branch on typed `error.code/status`. Preserve special `confirmUnlock` cooldown semantics (`tauri-rpc.ts:36-44`).
- **Alternatives:** Blanket `catch => false` or synthetic successful defaults are forbidden; avoid introducing an additional general-purpose error event bus.
- **Dependencies/constraints:** AA-04 prerequisite desirable; engine's security-related refusal remains authoritative.
- **Risk/acceptance:** Medium. Distinct offline/403/425/validation failures map to intended UI messages; no invalid successful result is synthesized.
- **Tests:** Facade unit tests for rejection/structured error/cooldown; UI mutation error-path tests.

### AA-06 — Treat site-pack import stream EOF without terminal event as error

- **Priority/status:** P1 / verified. **Evidence:** `src/routes/site-packs/_ImportPackModal.svelte:127-172`. The `for await` loop only handles FAILED or COMPLETED, and after normal exhaustion no branch marks the attempt as failed. In contrast `src/lib/stores/syncing-store.svelte.ts:153-187` explicitly treats missing terminal state as a failure.
- **Current behavior/problem:** A stream that closes before terminal status leaves creation incomplete without explicit feedback. Calling `onStreamError` that throws does not replace checking the iterator's terminal condition.
- **Approach:** Reuse the existing proven terminal-event semantics, or a small import-local equivalent if ownership differs. Maintain distinct technical error and domain FAILED handling; close/reset only after COMPLETED.
- **Alternatives:** Assume end-of-stream implies success — rejected.
- **Dependencies/constraints:** AA-13 covers cancellation separately.
- **Risk/acceptance:** Low. Early EOF, thrown iterator, FAILED and COMPLETED each produce correct UI state and never false success.
- **Tests:** Fake async iterable for four terminal cases; editor stays actionable after failed attempt.

### AA-07 — Test the real native boundary, not only mocked Tauri calls

- **Priority/status:** P1 / verified coverage gap. **Evidence:** `e2e/getting-started.spec.ts`, `e2e/help.spec.ts` are current Playwright browser suites; `src/lib/engine-client/tauri-rpc.test.ts` mocks Tauri invocation; `docs/IPC_ARCHITECTURE.md:53-72` and `docs/ai/TECH_DEBT_BACKLOG.md:31-39` document the missing Windows gate.
- **Current behavior/problem:** Unit/static browser tests cannot prove installation capability handling, Named Pipe ACLs, real remoc errors/events, service restart, lock-protected operations, or uninstall gating.
- **Approach:** Cross-reference **TD-001** instead of duplicating engine/installer audit work. Establish a repeatable native test harness for app-facing successful and failure flows on a disposable Windows VM; keep privileged service tests with the owning team.
- **Alternatives:** Reintroducing obsolete HTTP-mocked browser tests is rejected.
- **Dependencies/constraints:** Respect independent services and install-time ACLs; no production credential leakage into tests.
- **Risk/acceptance:** Medium. Native UI→Tauri→remoc success, unauthorized modification, offline/reconnect, stream completion/failure and locked rejection are verifiably exercised.
- **Tests:** The gate **is** the missing test suite; record environment and executed outcomes. No such tests were executed during this audit.

### AA-08 — Prevent default theme write from racing persisted theme load

- **Priority/status:** P1 / candidate (control-flow risk; requires reproduced scheduling test). **Evidence:** `src/routes/settings/+page.svelte:12-25` asynchronously loads `settings.json` in `onMount`; `$effect` at 27-43 starts an independent async write for initially `"system"`.
- **Current behavior/problem:** The initial effect may overwrite the saved theme before the asynchronous read resolves; later rapid changes can also finish out of order. `setMode` is invoked from both hydration and user change.
- **Approach:** Gate persistence behind completion of a single read/hydration step; use serialized saves or last-write-wins with a sequence guard; keep theme state local.
- **Alternatives:** Another global theme store is not justified; duplicate saving in every setting component is worse.
- **Dependencies/constraints:** Preserve current preference file format for **current** data; no legacy migration required.
- **Risk/acceptance:** Low. Reopening settings never overwrites a previously saved non-default theme; rapid changes persist the final selection.
- **Tests:** Mock deferred Tauri store `load/get/set/save`, vary resolution order, assert no pre-hydration save.

### AA-09 — Surface retrieval errors separately from genuinely empty collections

- **Priority/status:** P1 / verified. **Evidence:** `src/routes/network-blocks/+page.ts:23-42` produces `blocks: []` on RPC failure and sets `engineOffline` from a different liveness store; if ping succeeds but snapshot fails, `engineOffline=false` and the empty state renders (`+page.svelte:169-190`). `src/routes/site-packs/+page.ts:4-9` always falls back to empty array on error.
- **Current behavior/problem:** A real read failure can look like an empty policy/category inventory. This is especially misleading for security-sensitive configuration.
- **Approach:** Return a discriminated load state (`loaded/empty/offline/error`) or throw an explicit route load error with retry. Preserve last known-good visible state where warranted without pretending refresh succeeded.
- **Alternatives:** Treating an empty array as “error” is also wrong; empty is legitimate data.
- **Dependencies/constraints:** AA-05 for canonical error status; don't infer actual enforcement state from a failed UI request.
- **Risk/acceptance:** Low. Failed snapshot while engine ping is healthy renders a visible failure, not “no blocks”; successful empty results retain normal empty onboarding view.
- **Tests:** Page-load tests for healthy-empty, engine offline, server/pipe failure, successful populated reads.

### AA-10 — Review the generic WebView-to-engine operation allowlist

- **Priority/status:** P1 / candidate security hardening (no demonstrated bypass). **Evidence:** `src-tauri/src/lib.rs:41-52` registers `engine_call(op: String,args: Value)`; `src/lib/engine-client/sdk.ts:137` exports `shutdown`; engine-side `bridge.rs:403-407` includes a `shutdown` dispatch arm. `src-tauri/capabilities/default.json:5-31` defines general main-window capabilities.
- **Current behavior/problem:** Any code executing in the entrusted WebView can request operations not exposed in ordinary UI, including shutdown. This is **not proof** shutdown succeeds under lock: engine-authored capability and lock checks are a separate control that must remain in place. A generic JS entry point nevertheless increases the review surface.
- **Approach:** Characterize the security model and all intended user-session consumers. Where appropriate restrict desktop-invocable operations in the *Tauri adapter* while keeping engine's privileged maintenance/uninstall pathway independent. Harden against XSS as a separate security concern.
- **Alternatives:** Blindly deleting the engine shutdown RPC would break installer maintenance; bypassing engine authorization is forbidden.
- **Dependencies/constraints:** Root AGENTS non-negotiable no UI-driven engine shutdown; AA-07 native authorization tests.
- **Risk/acceptance:** High. Any WebView operation restriction is behavior-reviewed; lock and authorized maintenance shutdown are still enforced by engine, installer path unaffected.
- **Tests:** Real locked/unlocked engine shutdown and unauthorized-WebView call tests on VM before any production code change.

### AA-11 — Make legal/account loading failures explicit while retaining secure gating

- **Priority/status:** P1 / partially addressed. **Evidence:** `src/lib/stores/user-store.svelte.ts:85-105,122-166` rightly preserves last good entitlement and retries transient read failures. `src/routes/+layout.svelte:17-28,51-64` bases the legal loading mask on `!userStore.accountInfoLoaded` when engine reports alive; no distinct failure state is modeled.
- **Current behavior/problem:** On persistent account-info failure with a live engine, the user can remain in “loading” indefinitely without clear error or deliberate retry. Clearing Pro/ToS state on transport error would **not** be a safe fix.
- **Approach:** Add a typed/retriable account-load status and a UX path that preserves the ToS gate; keep single-flight refresh and last-known-good behavior. Clarify ownership of focus/visibility listeners (`user-store.svelte.ts:45-76`).
- **Alternatives:** Fall back to accepted ToS/Free on error — rejected.
- **Dependencies/constraints:** AA-05; must preserve auth/entitlement semantics.
- **Risk/acceptance:** Medium. Persistent read error produces meaningful feedback and retry; no unverified legal entitlement acceptance.
- **Tests:** Mock repeated pipe failures, engine recovery, logout, Pro downgrade, ToS not accepted, and window focus events.

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
- **Dependencies/constraints:** AA-06 terminal handling, AA-07 native tests.
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

### AA-17 — Give Site Packs one authoritative fetch lifecycle

- **Priority/status:** P2 / verified. **Evidence:** `src/routes/site-packs/+page.ts:4-9` fetches Site Packs and returns page data, but `+page.svelte:1-76` does not consume `PageData` and fetches on mount; `fetchPacks()` is also called after resync/delete (`+page.svelte:155-182,190-219`) without request sequence guard.
- **Current behavior/problem:** Unused initial fetching adds an IPC call and silently converts errors to empty data. Concurrent refreshes may apply older results after newer mutations.
- **Approach:** Remove the redundant load or make it the single source of initial truth; keep refresh responsibility clearly route-local with stale-response guards. Preserve retained selection when item still exists.
- **Alternatives:** Introduce a global Site Packs cache/store with unrelated consumers — not justified.
- **Dependencies/constraints:** AA-09 explicit errors; preserve progress state in `syncStore`.
- **Risk/acceptance:** Low. Initial page requests once, each mutation refresh is ordered, no stale update overwrites a recent action.
- **Tests:** Deferred overlapping results, delete/resync and selection restoration tests.

### AA-18 — Reconcile temporary schedule IDs after persisted adds

- **Priority/status:** P2 / candidate impact (normal modal currently closes on successful save). **Evidence:** `src/lib/deltas/schedule-delta.svelte.ts:180-199` stages `newTempId()` and inserts it into `currentState`. The add RPC response is not reconciled at `406-435`; `initialEntries` is replaced with `currentState` at `438-442`.
- **Current behavior/problem:** Reusing this delta instance after a successful save may treat temporary IDs as persisted server IDs. The current editor closes after save (`_BlockModal.svelte:217-229`), reducing but not proving absence of risk.
- **Approach:** Choose either single-use draft lifetime (enforce disposal) or refresh canonical persisted entries after success. Do not keep a fake historical-ID adapter.
- **Alternatives:** Generate client UUIDs and assume the server adopted them — rejected.
- **Dependencies/constraints:** AA-02 reconciliation should choose consistent post-save strategy.
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

## E. Suggested implementation order

| Batch | Findings | Goal / prerequisite | Independent verification |
| --- | --- | --- | --- |
| 0 | AA-01, AA-06, AA-09 | Immediate explicit error/terminal handling; no new abstractions | Targeted component/load/stream failure tests + `bun run check` |
| 1 | AA-03 | Protect overnight schedule preview semantics | Table-driven coverage/lock tests, compare engine-authoritative behavior |
| 2 | AA-02 | Characterize and reconcile partial Network Block commits | Fault-injection tests for each step and rollback; no false success |
| 3 | AA-05, then AA-04 | Typed error contract first; narrow RPC type-safety improvements after | Complete mapping/error contract tests; Rust check; AA-07 where available |
| 4 | AA-07 and AA-10 | Establish native security/IPC gate before high-risk transport hardening | Windows VM capability/lock/reconnect and installer integration |
| 5 | AA-08, AA-11, AA-12, AA-17, AA-18 | Reactive lifecycle, persistence and data freshness | Deferred-promise race tests, UI retry/visibility scenarios |
| 6 | AA-13, AA-16 | Stream consumer lifetime and inspector accessibility | Native stream lifecycle and keyboard smoke tests |
| 7 | AA-14, AA-15, AA-19 | Remove dead UI/config/doc debt | Reference checks, static checks, build and UI smoke |

Batches should be separately reviewable commits. **No item here authorizes a wholesale Svelte rewrite, an additional app-wide state framework, a custom remoc transport layer, or an engine change without a separate scope decision.**

## F. Open architectural questions

Only decisions that could change product-visible semantics or the security boundary need owner approval:

1. **Network Block save partial success (AA-02):** (A, recommended initial default) preserve current engine RPCs and explicitly reconcile/report each committed group after any failure; (B) require all-or-nothing edit semantics, which necessitates a separately designed engine-owned atomic operation, authorization/lock review and native tests. **Default A** for refactoring; escalate if atomic save is a product requirement.
2. **Desktop access to engine maintenance operations (AA-10):** (A, recommended) normal WebView cannot call privileged maintenance/shutdown from arbitrary generic operation names; authorized installer/uninstall path remains distinct; (B) retain full generic desktop bridge with explicit rationale and hardened trust model. Must confirm no legitimate UI workflow depends on direct shutdown before changing the allowlist. **Default A subject to native compatibility proof**.

AA-08, AA-16 and AA-18 are engineering/test questions, not invitations to solicit visual or product preferences without evidence.

## G. Verification strategy and actual checks

### Executed during this audit (2026-10-22)

- **Git baseline:** `main` at `c865a15`, clean status before editing.
- **Inventory:** traversed `src/`, `src-tauri/`, `e2e/`; classified generated and legacy UI; inspected source and call sites across routes, deltas, stores, remoc facade, Tauri Rust, configs and tests.
- **`cd apps/ace-app; bun run check`: PASS** — 0 Svelte errors and 0 Svelte diagnostic warnings. Configuration warnings reported separately: `config_option_deprecated_alias`, `tsconfig_invalid`.
- **`cd apps/ace-app; bun run test:unit`: PASS** — five test files, **42 tests passed**.
- **`cd apps/ace-app; bun run build`: PASS** — Vite/SvelteKit build completed, same configuration warnings.

### Not executed / insufficient as proof

- Native `cargo check -p ace-app` and cargo tests: **not run**; no Rust source changed.
- `bun run test:e2e`: **not run**; current Playwright suites test static/browser routes, not real remoc.
- Real Tauri GUI screenshot and keyboard audit, Windows VM/service/Named Pipe ACL, hostile WebView, installer/uninstall and watchdog checks: **not run**.
- No mutation-failure injection or reproduction of speculative races was run; impact of *candidate* findings remains to be proven.

### Required implementation gates

Run `bun run check`, `bun run test:unit`, and `bun run build` from `apps/ace-app` after every relevant frontend batch. Add explicit failure-path tests **before** refactoring P0/P1 behavior. For Tauri/IPC changes run `cargo check -p ace-app` from repository root, relevant engine remoc unit/contract suites as scoped by ownership, and disposable native Windows VM checks defined in `docs/IPC_ARCHITECTURE.md`. Verify behavior/architecture/type gates separately from compilation; record exact outcomes per implementation batch. Review the final diff for accidental source or generated-code modifications.

## H. Explicit non-goals

- **No backward compatibility** for legacy Ace Blocker pre-release databases, historical backups/formats, old REST control endpoints, obsolete clients or old IPC payload shapes, except currently supported runtime behavior and active development data.
- No implementation changes in this audit. No edits to engine, watchdog, installer, or generated external Ace Server client.
- No local REST/HTTP fallback. Keep typed remoc over Named Pipes, Rust-owned capability and independent LocalSystem enforcement.
- No global state/event bus merely to coordinate otherwise independent feature UI; no generic transaction framework in Svelte; no pointless types, wrappers, crates or dependencies.
- No assumption that a successful UI check proves Windows service security; no automatic deletion solely from file size, appearance or lexical nonuse.
- No speculative redesign of correct screens; subjective theme preferences are not audit findings. `apps/ace-app/docs/ui-identity.md` governs any future visible UI changes.

## Cross-reference

- Root `AGENTS.md`, nested `apps/ace-app/AGENTS.md`, `docs/ai/REFACTORING_PLAYBOOK.md`, `docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`, `docs/ai/DOMAIN_TYPES.md`, `docs/ai/REVIEW_GATES.md`, `docs/ai/TECH_DEBT_BACKLOG.md`, `docs/ai/ACE_ENGINE_REFACTOR_DECISIONS.md`, `docs/IPC_ARCHITECTURE.md`, `apps/ace-app/docs/ui-identity.md`.
- Existing **TD-001** (native IPC test gap) and **TD-017** (alias warnings) remain the project-wide tracking references. Previously addressed **TD-016** (stream queue `shift` cost) must not be re-opened as a finding; AA-13 is limited to lifecycle/boundedness.
- **Audit conclusion:** apply small correctness and failure-reporting fixes before optional cleanup, retain server/engine policy authority, and never convert a failed operation into an apparently successful primitive or empty data shape.

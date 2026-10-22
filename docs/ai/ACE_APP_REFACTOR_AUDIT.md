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
- **Approach:** Single-flight or monotonic request sequence in each existing store; explicit start/stop ownership for engine timer; skip redundant work while hidden/o

## I. P0/P1 implementation progress (2026-10-22; supersedes original audit status labels)

The finding descriptions above are the **pre-implementation audit snapshot**.
This follow-up records actual changes and verification without marking
non-native tests as proof of installed security or runtime correctness.

| Finding | Implementation status | Scope and remaining gate |
| --- | --- | --- |
| **AA-01 (P0)** | Implemented; unit verified | Edit icon and Safe Search RPC failures are checked and are never reported as successful saves. Native mutation errors not yet exercised. |
| **AA-02 (P0)** | Implemented for explicit failure/reconciliation; native gate open | Independent writes now execute sequentially, with explicit `saved/failed` outcomes and reload or editor lockdown after ambiguous partial commits. No cross-resource atomicity claimed. |
| **AA-03 (P1)** | **Partial; product/engine decision required** | Pure table-tested week-minute coverage for UI preview; proposed invalid overnight input is rejected in preview. Rust `ScheduleEntry::from_raw` rejects `22:00–02:00` but `contains` has special `end=00:00` and `Schedule::is_strictier_than` uses raw end-time ordering. Matching all Rust lock/midnight semantics requires a separate security-reviewed engine decision and native tests. |
| **AA-04 (P1)** | Implemented; typed facade and contract tests pass | 61 request and 2 stream `as any` wrappers replaced by typed generics; operation names and JSON keys compared with actual Rust bridge source. Actual end-to-end serialization remains for AA-07. |
| **AA-05 (P1)** | Implemented; unit verified | Canonical `RpcFailure` in TypeScript, structured serialized Rust/transport errors, 425 cooldown retained; removed HTTP-era `response.status` consumers. |
| **AA-06 (P1)** | Implemented; unit verified | Site Pack import recognizes only `COMPLETED` as success; tests cover COMPLETED, FAILED, iterator exception and early EOF. |
| **AA-07 (P1)** | **Blocked by test environment** | `CodexPro CAP vm images` returned no registered images. No disposable Windows VM, no native WebView/Named Pipe/installer validation. See `ACE_APP_NATIVE_TEST_GATE.md`, and TD-001. |
| **AA-08 (P1)** | Implemented; behavior requires native/deferred UI verification | Removed default-theme auto-save before hydration; user changes serialized. Deferred store load/rapid writes are not yet covered by component tests. |
| **AA-09 (P1)** | Implemented; static check/build verified | Network Blocks read errors are separate from legitimate empty data with retry; Site Packs already owned error state in Svelte page, duplicate hidden route fetch removed. |
| **AA-10 (P1)** | **Pending approval + native gate** | Reviewed generic WebView Tauri operation and engine's separate installer shutdown path. Do **not** remove engine shutdown or silently change lock/maintenance policy. Proposed desktop-only deny restriction in `ACE_APP_NATIVE_TEST_GATE.md`; VM required first. |
| **AA-11 (P1)** | Implemented; native error-path gate open | Account status loading/error/ready and legal-gate retry; no inferred ToS acceptance or Free downgrade on read failure; focus listeners and retry timer disposed with store. |

**Tests executed in this implementation session:**
`cd apps/ace-app; bun run check` **PASS**, Svelte 0 errors / 0
diagnostic warnings (configuration still warns about deprecated SvelteKit
alias and overwritten TS `paths`, tracked as AA-15 / TD-017).
`bun run test:unit` **PASS: 67 tests / 9 suites** (baseline 42 / 5).
`bun run build` **PASS**.
`bun run test:e2e` **PASS: 4 browser-only smoke tests** (not native Tauri/remoc).
`cargo check -p ace-app` **PASS** with two existing ace-engine unused-function warnings.
The direct Tauri UI, browser/engine liveness, service ACL, native stream
and locked maintenance/uninstall cases have **not** run; no VM was available.

**Adjacent work:** AA-17's redundant Site Packs `+page.ts` initial fetch was
removed; stale in-component response ordering remains separately open as P2.
The original P2/P3 findings remain in the audit.

**Open decisions:** 1) whether Ace's authoritative engine should support
overnight intervals crossing midnight or retain existing constrained
representation, with separate lock-semantics review; 2) authorize AA-10's
desktop-only shutdown restriction after registering a disposable Windows
test VM. No legacy compatibility is required before public release, but
current security and independent service enforcement remain mandatory.

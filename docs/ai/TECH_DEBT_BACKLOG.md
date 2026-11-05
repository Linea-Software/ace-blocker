# Ace Blocker — Technical Debt Backlog

Use [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md) for the required audit, implementation, and independent-review process.

This file tracks **verified, actionable maintainability work**, not a wishlist of speculative rewrites. Add items only after inspecting the relevant code and identifying concrete costs. Audit findings may also be marked `candidate` if they need verification.

## Queue

| ID | Priority | Status | Area | Problem / evidence | Intended outcome |
| --- | --- | --- | --- | --- | --- |
| [TD-001](#td-001--native-ipc-integration-coverage) | P1 | verified | Native IPC testing | Removed browser E2E suites mocked the retired REST port; unit tests alone do not exercise installed Windows services | Add Windows Tauri/remoc/installer E2E coverage |
| [TD-002](#td-002--safe-and-efficient-local-site-pack-imports) | P1 | review | Local site pack importer | Local resync rebuilt the live SQLite DB; multiple wrapper streams repeated terminal handling; huge single INSERTs exceeded SQLite bind-variable limits | Stage and validate before swap, consolidate import streams, and transactionally chunk inserts |
| [TD-003](#td-003--live-policy-consistency) | P1 | review | Policy caches and writes | Cached URL decisions survived mutations; failed SQLite writes could leave modified live state | Invalidate caches and stage validation before committing mutations |
| [TD-004](#td-004--interception-fast-path) | P1 | review | Proxy and NetworkManager | HTML buffering, per-request DB locking, duplicated host scans and catalog cloning | Stream unaffected responses and lower per-request work |
| [TD-005](#td-005--bounded-network-caches-and-attribution) | P2 | review | mitm-proxy / ace-tun | Origin, cert and DNS caches were unbounded; Windows attribution allocated large buffers | Bound growth; reduce Windows allocation and align native buffers |
| [TD-006](#td-006--control-plane-and-rule-overhead) | P2 | review | API/dashboard and rule engine | Exclusive dashboard lock; repeated rule compilation; verbose traffic logs | Shared reads, incremental domain updates and leaner logging |
| [TD-007](#td-007--tunnel-policy-event-driven-evaluation) | P1 | candidate | Tunnel policy lifecycle | 500 ms per-tunnel polling persists despite cheaper evaluation | Preserve immediate edit and scheduled policy transitions with change notification or bounded efficient checks |
| [TD-008](#td-008--regex-interception-scope) | P1 | candidate | TLS interception policy | Arbitrary active regexes conservatively select all domains for MITM | Safely narrow only provably host-scoped regexes; retain opaque default |
| [TD-009](#td-009--bounded-oversized-html-handling) | P1 | candidate | HTTP response mutation | Eligible HTML response collection and decompression remain unbounded | Bound memory without silently disabling resistance or breaking large legitimate responses |
| [TD-010](#td-010--http1-upstream-reuse) | P2 | blocked | mitm-proxy upstream | HTTP/1.1 reconnects per request; a simple idle-sender pool serializes live streams | Design body-lifetime-aware bounded pooling; streaming concurrency regression committed |
| [TD-011](#td-011--windows-pid-table-attribution) | P2 | candidate | Windows TUN attribution | Every flow still snapshots/scans IP-Helper ownership tables | Consider safe reuse of ownership snapshots after attribution-race and PID-reuse tests |
| [TD-012](#td-012--rpc-and-production-h3-footprint) | P3 | candidate | UI RPC and proxy features | Bridge repeats dispatch boilerplate and opens connections per call; H3 unused by Ace production | Consider generated dispatch/persistent reconnecting client and feature gate with native integration tests |
| [TD-013](#td-013--network-block-mutation-lock-guard) | P2 | review | NetworkManager block mutations | Deletion, safe-search edits and renaming independently duplicate missing-block and active-lock checks | Centralize this repeated guard within the block manager; preserve error and persistence order |
| [TD-014](#td-014--site-pack-link-persistence-consistency) | P1 | review | NetworkManager site-pack assignments | Attach/detach mutate the live block before a fallible SQLite write | Add failure-path regressions and make DB/memory assignment changes consistent |
| [TD-015](#td-015--allocation-free-hosts-token-scanning) | P2 | review | Domain blocklist hosts parser | Collects all whitespace tokens into a temporary `Vec` for each input line | Iterate tokens directly while retaining error order and alias validation |
| [TD-016](#td-016--linear-time-tauri-stream-queue) | P2 | review | Ace App Tauri RPC stream | Uses `Array.shift()` for each event in site-pack progress streams | Replace head-removal with cursor-based draining without changing event/error ordering |
| [TD-017](#td-017--sveltekit-alias-configuration-warnings) | P2 | candidate | Ace App tooling | Current SvelteKit build reports deprecated config alias and overwritten tsconfig paths | Validate and separately migrate aliases without breaking imports |
| [TD-018](#td-018--linux-inode-attribution-cache-metadata) | P3 | done | ace-tun Linux process attribution | An inode→PID cache stores a timestamp that no reader uses | Remove unused metadata without touching endpoint TTL, rescan or routing behavior |

For each non-placeholder item, add a detailed section below using the template. Link the queue's ID to its section. Order by expected maintenance benefit and risk, not cosmetic code metrics.

### TD-001 — Native IPC integration coverage

- **Status:** verified; **priority:** P1.
- **Area:** `apps/ace-app/e2e/`, `apps/ace-app/src/lib/engine-client/tauri-rpc.test.ts`, `crates/ace-engine/src/service/api/rpc.rs`, `installer/`.
- **Evidence:** Nine removed Playwright suites intercepted the obsolete engine HTTP endpoint at `127.0.0.1:10567`; they could not exercise production remoc calls. The replacement Tauri adapter unit tests mock `invoke`, and Rust remoc tests use an in-process duplex connection.
- **Cost:** Regressions in installed Windows Named Pipe ACLs, SCM service startup, watchdog recovery, WebView-to-Tauri IPC, and uninstall gating could escape current automated checks.
- **Smallest improvement:** Add native Windows integration tests covering those boundaries and run them on a disposable VM, separately from the static browser smoke tests.
- **Security invariants:** Never bypass installation capability checks, lock rules, Windows service separation or uninstall refusal.
- **Verified checks during cleanup:** `cargo check --workspace --all-targets`; `cargo check --release -p ace-engine -p ace-watchdog -p ace-app`; `cargo test -p ace-engine --lib -p ace-watchdog-protocol` (206 + 3 passed); `bun run check`; `bun run test:unit` (39 passed).
- **Dependency:** Native Windows test harness and a disposable VM. No release-installation validation performed in this cleanup.

### TD-002 — Safe and efficient local site pack imports

- **Status:** review; **priority:** P1.
- **Area:** `crates/ace-engine/src/core/network_block/site_pack/{importer/importer.rs,site_pack_db.rs,mod.rs}`, `crates/ace-engine/src/manager/network/site_pack.rs`.
- **Evidence:** The previous local resync closed and cleared the live database before parsing; failed or cancelled imports could leave enforcement unloaded. `execute_resync` and separate manager streams duplicated forwarding and completion handling. The 50,000-row Diesel INSERT conflicted with SQLite's bound-variable limits.
- **Change:** Build into a unique sibling staging directory and swap via rollback-capable directory renames after successful import and metadata validation; keep old enforcement available until then. Consolidate local import/resync coordination, remove one forwarding stream layer, cap statements at 500 variables with a transaction per 50,000-row batch; buffer input at 64 KiB and aggregate malformed-line warnings. Preserve RPC progress events, sync guards, and remote backup behavior.
- **Verification (Windows workspace):** `cargo check -p ace-engine --lib` passed; `cargo test -p ace-engine --lib resync` passed (2 tests); `cargo test -p ace-engine --lib core::network_block::site_pack` passed (19 tests). No benchmarks run. Repository-wide `cargo fmt --all --check` reports pre-existing formatting elsewhere; only changed source files were formatted.
- **Residual review:** Independent diff review and installed Windows service/integration verification remain outstanding; no measured performance claims. Existing unrelated compiler warnings remain.
- **Owner/branch:** `main`. **Scope:** engine local importer and shared backup activation; no protocol or UI contract changes.

### TD-003 — Live policy consistency

- **Status:** review; **priority:** P1. Commits: `5564dd9`, `0616022`, `7bc31a7`.
- **Evidence:** `NetworkBlock::results_cache` previously retained decisions after rule/exception/link changes and site-pack swaps. Block property setters mutated memory before fallible SQLite writes, while rule/exception additions persisted before complete matchers were compiled.
- **Change:** Invalidate dependent caches on mutations and successful local/remote/background pack activation; persist simple block edits first; validate complete matchers before persistence. Shared cache handles let async import streams invalidate after activation.
- **Tests:** Cache allow/block mutation regressions; abort-triggered SQLite write failures; invalid exception persistence; full `cargo test -p ace-engine --lib`. **Risk:** high (enforcement); independent review and installed Windows verification outstanding.

### TD-004 — Interception fast path

- **Status:** review; **priority:** P1. Commits: `4e6eab0`, `f6a5cb3`, `e72573a`, `a850523`, `e51f213`.
- **Evidence:** All top-level HTML buffered despite non-eligibility; blocked attempts acquired exclusive policy lock for synchronous SQLite upsert; repeated CONNECT and established-tunnel checks traversed policy and rebuilt script catalog; supposedly cached Pro flag accessed SQLite.
- **Change:** Select streaming before collecting non-injectable HTML; persist block counts via `spawn_blocking` outside policy lock; use an atomic Pro snapshot synchronized after DB writes; avoid full script cloning and merge CONNECT host/block evaluation. The existing 500 ms tunnel polling and its schedule semantics remain unchanged.
- **Tests:** Existing proxy/interception tests, Pro clone/restart regression, full engine lib suite. **Limit:** no speed benchmarks; response-body size is still unbounded for eligible HTML.

### TD-005 — Bounded network caches and attribution

- **Status:** review; **priority:** P2. Commits: `7c19dd1`, `61ad98e`, `ab8d9c2`.
- **Evidence:** MITM leaf-cert and origin caches had no capacity limits; DNS snoop map could retain expired entries indefinitely; each Windows PID lookup allocated a 64 KiB path buffer. IP-Helper table buffers were byte-vector aligned despite cast to typed MIB rows.
- **Change:** Cap leaf certificates at 512 entries and origin protocol/auth hints at 1024 entries, DNS IP observations at 8192; prune DNS expiry when at capacity; use aligned native table storage; start path queries with 512 UTF-16 units and grow on insufficient buffer.
- **Tests:** `cargo test -p ace-tun --lib` (132 passed), `cargo test -p mitm-proxy --lib` (56 passed), Windows process-attribution smoke tests. **Risk:** eviction can cost a cache miss; native Windows VM review outstanding.

### TD-006 — Control-plane and rule overhead

- **Status:** review; **priority:** P2. Commits: `2a0546e`, `7e43bf6`, `973fa72`.
- **Evidence:** The dashboard held the exclusive NetworkManager lock while fetching per-block SQLite counts. Domain-only rule edits recompiled the full regex set. Routine flows and requests logged at INFO.
- **Change:** Read policy under a shared lock; count user rules from memory, fetch today's blocked count after releasing it; use read-only getters in the schedule/site-pack APIs; update domain-only rules without regex recompilation; move routine traffic logging to DEBUG.
- **Tests:** Entire engine library suite, domain duplicate/invalid-batch tests, and workspace all-target check. **Limit:** installed UI behavior not tested.

### TD-007 — Tunnel policy event-driven evaluation

- **Status:** candidate; **priority:** P1; **scope:** `mitm-proxy/src/http.rs` tunnel termination polling and policy update notifications.
- **Evidence:** Every established tunnel checks policy at 500 ms intervals, including in the absence of a policy transition. Existing checks are cheaper after TD-004, but periodic work remains.
- **Risk / gate:** Preserve schedule transitions, mid-session enforcement, reconnect semantics, and opaque/intercepted distinction. Requires clock-controlled tunnel tests and integration coverage before replacing the poll loop.

### TD-008 — Regex interception scope

- **Status:** candidate; **priority:** P1; **scope:** `NetworkBlock::requires_request_inspection_for_host`.
- **Evidence:** An active arbitrary full-URI regex forces TLS inspection on unrelated hosts because path-dependent matching cannot be safely decided using CONNECT metadata.
- **Risk / gate:** Regexes matching a host *within the URI path* cannot be treated as host-specific. Narrow only patterns proven safe by conservative analysis; characterize cases before any changes.

### TD-009 — Bounded oversized HTML handling

- **Status:** candidate; **priority:** P1; **scope:** `mitm-proxy/src/http.rs`, `service/proxy/mod.rs`.
- **Evidence:** Eligible response bodies are collected without a hard cap; compressed HTML may decompress without an upper bound.
- **Risk / gate:** A simple 413/502 cutoff would break large sites, while streaming through oversized documents without injection would weaken Resistance Mode. Design an explicit, tested fallback/enforcement policy before imposing limits.

### TD-010 — HTTP/1 upstream reuse

- **Status:** blocked; **priority:** P2; **scope:** `mitm-proxy/src/upstream.rs`, `tests/proxy_http1.rs`.
- **Evidence:** The HTTP/2 origin sender is reusable, while HTTP/1.1 requests perform new handshakes. A small per-origin sender pool successfully reused sequential completed responses, but a new concurrency test caught head-of-line blocking: the second request waited for the first indefinite streaming response. Pooling code was rolled back in this session; no behavior change shipped.
- **Regression coverage:** Commit `30c7be3` (`busy_streamed_http1_response_does_not_delay_other_requests`) covers concurrent streaming and regular responses across browser tunnels. `cargo test -p mitm-proxy --test proxy_http1` passes with existing behavior.
- **Risk / gate:** The sender must return to the pool only after response-body completion or cancellation. Hyper's `Response<Incoming>` return type cannot directly carry a custom end-of-body callback. A body-lifetime-aware adapter/connection pool plus streaming, idle-expiry, WebSocket, TLS-client-certificate and cancellation tests is required before enabling reuse.

### TD-011 — Windows PID table attribution

- **Status:** candidate; **priority:** P2; **scope:** `ace-tun/src/platform/windows/process.rs`.
- **Evidence:** Each new flow still queries and linearly scans a fresh TCP/UDP ownership table. TD-005 only improves memory use and alignment.
- **Risk / gate:** PID/port reuse and process attribution races are enforcement-sensitive. Require Windows stress and negative-attribution cases before caching OS tables.

### TD-012 — RPC and production H3 footprint

- **Status:** candidate; **priority:** P3; **scope:** `service/api/bridge.rs`, `transport.rs`, `mitm-proxy/src/http3.rs`, crate feature graph.
- **Evidence:** The Tauri bridge has repetitive operation dispatch and new remoc handshakes for calls; Ace's TUN suppresses browser QUIC, while standalone mitm-proxy supports H3.
- **Risk / gate:** Persisting clients requires robust pipe-restart and capability authorization behavior; feature gating must keep standalone proxy/test functionality. Requires native Windows IPC and H3 tests.

**Audit implementation verification (2026-10-19):** `cargo check --workspace --all-targets` passed; `cargo test -p ace-engine --lib -- --test-threads=1` passed (216 after all engine changes), `cargo test -p ace-tun --lib` passed (132), `cargo test -p mitm-proxy --tests` passed with one intentionally ignored external-network test (before the new concurrent-stream test), and `cargo test -p mitm-proxy --test proxy_http1` passed with the new test. One parallel engine run failed on a missing global resistance-script test fixture; the failing test passed individually and the full suite passed serially. No performance benchmarks were run (by request). Existing generated-client/watchdog/test warnings remain. Native installer/service and independent code review have not run.

### TD-013 — Network block mutation lock guard

- **Status:** review; **priority:** P2; **scope:** `crates/ace-engine/src/manager/network/network_blocks.rs`.
- **Evidence:** `delete_network_block`, `set_network_block_safe_search`, and `rename_network_block` each independently check existence then active lock state with the same error cases.
- **Cost / change:** Repeated authorization checks make future lock-related maintenance error-prone. Replace these three branches with one private, narrowly scoped guard; leave the strengthening-only `set_network_block_enabled` condition and all persisted write ordering unchanged.
- **Security constraints:** Missing blocks yield `NetworkBlockNotFound`; actively locked blocks yield `NetworkBlockIsLocked`; successful calls still persist before mutating live state. No rule, lock, or IPC semantics change.
- **Baseline (Windows, 2026-10-21):** `cargo test -p ace-engine --lib manager::network::network_blocks::tests -- --test-threads=1` passed (7 tests). Two warnings in generated `ace_server_api/types.rs`.
- **Outcome (Windows, 2026-10-21):** Added a private `require_unlocked_network_block` guard for these three mutations only. Added two regression tests before refactoring for missing-block errors and active locks (including persisted policy after rejected writes). Left strengthening operations and DB-before-memory write ordering unchanged.
- **Verification:** Pre-refactor tests passed (9/9 after adding characterization coverage); post-refactor focused tests passed (9/9); full `cargo test -p ace-engine --lib -- --test-threads=1` passed (221/221); `cargo check -p ace-engine --bin ace-engine` passed; scoped `rustfmt --check --edition 2024 crates/ace-engine/src/manager/network/network_blocks.rs` passed. Two pre-existing generated-client dead-code warnings persist. No Windows installed-service/VM tests were run because the change is restricted to a previously tested manager guard.
- **Review:** In-session diff inspection found no changed public interfaces, error variants, persistence order, or strengthening-only lock exception. An independent reviewer has not yet reviewed the diff, so this item remains `review` rather than `done`.

### TD-014 — Site-pack link persistence consistency

- **Status:** review; **priority:** P1; **scope:** `crates/ace-engine/src/manager/network/network_blocks_to_site_pack.rs`. Separate task; not part of TD-013.
- **Evidence:** Previously, both attach and detach changed the live block before a fallible SQLite write, potentially diverging policy and persistence.
- **Change (2026-10-22):** Validate block and pack preconditions first; commit SQLite assignment before publishing the in-memory link change. Existing lock checks, unsynced rejection, and cache invalidation remain intact.
- **Verification:** New test `site_pack_assignment_sql_failures_do_not_change_live_links` injects SQL failures for both add/remove, asserting policy remains unchanged. Complete engine serial library suite **227/227 passed**; `cargo check -p ace-engine --all-targets --all-features` passed. No installer/SCM/VM test performed.
- **Residual gate:** Independent security/domain/architecture diff review and installed-service validation remain open; therefore `review`, not `done`.

### TD-015 — Allocation-free hosts token scanning

- **Status:** review; **priority:** P2; **owner:** `main`.
- **Area / evidence:** `crates/ace-domain-blocklist/src/format/hosts.rs`, `HostsListStandardFormat::parse_line` collects `line.split_whitespace().collect::<Vec<&str>>()` solely to inspect the first token and process the remaining tokens. Imports potentially process many lines, so this is a redundant per-line allocation and intermediate collection.
- **Scope / simplest improvement:** Within this parser alone, use the `split_whitespace` iterator and preserve its redirect-first, alias-second validation, comment handling, alias order and error variants. No shared helper or new crate/type.
- **Architecture / domain impact:** Ownership stays within `ace-domain-blocklist::format::hosts`; external public parsing trait and `DomainName` validator remain unchanged, as do the importer integration and any persisted data. No new dependency edges, protocol modifications or unchecked primitives.
- **Baseline (Windows, 2026-10-22):** `cargo test -p ace-domain-blocklist`: 54 passed. Existing tests cover comments, redirects, multiple aliases, malformed input, bare IP rejection and first-invalid-domain reporting. Add focused characterization coverage for whitespace-only and long alias lists if warranted.
- **Risk / acceptance:** Low. Exact `Result` and error precedence for all inputs must remain identical; same 54 tests plus new cases and scoped formatting/check. Rollback: revert only the parser commit.
- **Outcome (2026-10-22):** Replaced per-line token collection with an iterator and added a 128-alias order regression. `cargo test -p ace-domain-blocklist`: 55/55 passed; `cargo check -p ace-domain-blocklist`: passed; `rustfmt --check --edition 2024 crates/ace-domain-blocklist/src/format/hosts.rs`: passed; `git diff --check`: passed. No benchmark or installed-service testing attempted. In-session architecture review: no dependency edges or authority changes; domain/type review: unchanged `DomainName` validation and `DomainLineParsingError`; behavior review: same missing-redirect and missing-alias precedence, valid aliases and malformed-input errors. Independent reviewer unavailable in active CodexPro toolset; remains `review` rather than `done`.

### TD-016 — Linear-time Tauri stream queue

- **Status:** review; **priority:** P2; **owner:** `main`.
- **Area / evidence:** `apps/ace-app/src/lib/engine-client/tauri-rpc.ts`, `rpcStream`: every queued event is consumed using `parsed.shift()` (array head removal). When multiple events accumulate before the UI drains them, repeated head removal moves remaining array elements. Event-stream consumers include `src/lib/stores/syncing-store.svelte.ts` and site-pack import UI.
- **Scope / simplest improvement:** Keep the same local typed-Tauri adapter API but use an indexed queue, reclaim consumed entries without quadratic array head shifts and add asynchronous burst/error regression tests. No new third-party library or shared state.
- **Architecture / type impact:** UI stream adapter only; `engine-client` façade, Tauri channel and remoc wire, auth, error callback and consumers remain unchanged. `unknown` event values remain opaque Tauri payloads; avoid adding a competing payload schema.
- **Baseline (Windows, 2026-10-22):** `bun run test:unit` from `apps/ace-app`: 39 passed, including 4 existing RPC-adapter tests. `bun run check` and focused tests required after implementation.
- **Risk / acceptance:** Low-to-moderate due to asynchronous notification. Preserve FIFO, completion, error propagation, callback behavior, no lost wakeups, and all existing response shapes. Native remoc stream integration remains a separate TD-001 obligation.
- **Outcome (2026-10-22):** Switched event queue dequeue from repeated `Array.shift()` to an index and occasional consumed-prefix reclamation; no new module, transport or payload types. Added regression cases for 256 buffered/interleaved events, notification after asynchronous wait, and callback + error propagation. The new tests passed before the change (6/6 with the first two tests) and after it (7/7 with the wakeup test). `bun run check` passed with 0 diagnostic errors/warnings (pre-existing SvelteKit configuration notices persist); `bun run build` passed; final `bun run test:unit` passed (42/42); final `bun run check` passed (0 code diagnostics); `git diff --check` passed; `cargo check --workspace --all-targets` passed. Independently executing fresh-context code reviewer unavailable; in-session architecture pass found no cross-layer edge additions, type pass found unchanged opaque event payloads, and behavior diff pass found the existing completion/error ordering retained. Leave at `review` until independently reviewed.

### TD-017 — SvelteKit alias configuration warnings

- **Status:** candidate; **priority:** P2; **area:** `apps/ace-app/vite.config.js:19-23`, `apps/ace-app/tsconfig.json`, generated SvelteKit TS configuration.
- **Evidence (Windows, 2026-10-22):** Both `bun run check` and `bun run build` succeed but report `config_option_deprecated_alias` and `tsconfig_invalid` (`paths` overwritten). The config currently passes an `alias` object to `sveltekit(...)`; the inherited TS config may override paths. `bun run check` still reports zero code diagnostics.
- **Cost / scope:** Potential future SvelteKit upgrade breakage and confusing import resolution. Inspect current framework configuration guidance and generated TS paths, characterize `$lib` and `$engine-client` import resolution and remove obsolete aliases only after equivalent compile/build tests. Do not change app logic, remoc, or component design.
- **Architecture / type gate:** Existing alias setup is build tooling, not a request for a new module or type. Preserve project-local source imports and build paths; migration is a separate task from TD-016. No solution assumed until current SvelteKit supported configuration is verified.
- **Review / verification:** Candidate only; native Tauri/Windows integration tests not related. No configuration modification attempted.

### TD-018 — Linux inode attribution cache metadata

- **Status:** done; **priority:** P3; **scope:** `crates/ace-tun/src/platform/linux/process.rs`, `ProcessCache::inodes` and `inode_to_pid`. Linux-only; no engine, proxy policy, rule matcher or Windows/macOS code changes.
- **Evidence / maintenance cost:** The cached inode→PID map was stored as `Option<(HashMap<u64, u32>, Instant)>`, but both readers destructured and ignored the timestamp. This misleadingly suggested a time-based expiry that the code does not implement.
- **Change:** Retain only `Option<HashMap<u64, u32>>`. The cache still returns hits, rescans `/proc` on misses, and holds the same mutex through the same operations. The distinct endpoint→PID cache retains its 250 ms TTL. No new abstractions, dependencies, types, changed OS calls or wire/privilege boundaries.
- **Behavior and verification (2026-11-05):** Before and after, `cargo test -p ace-tun --lib --locked` passed **119/119**, including Linux live TCP/UDP socket attribution and browser/QUIC routing tests. `cargo check --locked -p ace-tun --all-targets` passed. Browser-process list tests passed **4/4**, and `mitm-proxy` library tests passed **61/61**, unchanged. `git diff --check` passed. The focused Rustfmt check reports pre-existing formatting differences outside the touched lines; no whole-file formatting was applied.
- **Architecture/type/independent review:** The Linux attribution module retains full ownership; no new import edge, public interface, IPC model or validation occurs. Separate read-only audits identified the unused field. Independent diff review found no introduced defect and confirmed the same cache lookup/refresh paths. This does **not** prove Windows/macOS, installed Engine, traffic interception or any specific third-party browser is "undetectable"; those paths were not modified or tested.

## Item template

Copy this section for each verified or candidate finding. Do not fabricate evidence or completion details.

### TD-001 — Short, actionable title

- **Status:** candidate / verified / ready / in_progress / review / done / blocked / rejected
- **Priority:** P0 (urgent correctness/security) / P1 (high cost) / P2 (routine) / P3 (opportunistic)
- **Area and files:** `path/to/file.rs` — symbol/function and line or search identifier
- **Evidence:** Observed duplication, coupling, unnecessary dependency, obsolete path, or test gap. Cite concrete call sites.
- **Cost:** Why this makes maintenance harder or increases regression risk.
- **Smallest proposed improvement:** Favor simplification or deletion; state why a new abstraction is truly needed, if one is proposed.
- **Architecture/type evidence:** Affected ownership, existing/proposed dependency edges, whether shared code represents the same stable concept, and any weak primitive/invalid-state risk. State why a new type, helper, cross-feature import or crate is genuinely needed; otherwise record why not.
- **In scope / out of scope:** Explicit module boundaries; no bundled unrelated rewrites.
- **Behavior and security invariants:** Identify observable contracts, validation/serialization/DB compatibility and trust/privilege boundaries to preserve.
- **Baseline / tests needed:** Commands, currently observed results, and any characterization tests to add before edits.
- **Dependencies / blockers:** None or explicit linked items.
- **Owner / branch or worktree:** Unassigned until work starts.
- **Reviewer and review findings:** Independent architecture/abstraction and domain/type passes, plus behavior/security review with file references; record unresolved conflicts.
- **Outcome / verification / commit:** Fill in only after implementation and actual checks.

## Operating rules

1. Read-only audits may add or update backlog items; they must not alter product code.
2. Implementation tasks claim a single ID and keep code changes bounded.
3. Reviewers verify changes independently and note unresolved risks.
4. Mark `done` only after passing the relevant checks, recording any checks that could not run, and completing independent review.
5. Do not silently erase or reprioritize someone else's in-progress task.

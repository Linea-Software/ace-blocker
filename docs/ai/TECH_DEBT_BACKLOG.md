# Ace Blocker — Technical Debt Backlog

Follow [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md) for the required audit, implementation, and independent review.

This file records **verified, actionable maintenance work**. It does not list speculative rewrites. Inspect the relevant code before you add an item. Record the specific maintenance cost. Mark an audit finding `candidate` if it still needs verification.

## Queue

| ID | Priority | Status | Area | Problem / evidence | Intended outcome |
| --- | --- | --- | --- | --- | --- |
| [TD-001](#td-001--native-ipc-integration-coverage) | P1 | verified | Native IPC testing | Nine removed browser E2E suites used the retired REST port. Unit tests do not test installed Windows services. | Add Windows E2E tests for Tauri, remoc, and the installer |
| [TD-002](#td-002--safe-and-efficient-local-site-pack-imports) | P1 | review | Local site pack importer | Local resync rebuilt the live SQLite DB. Wrapper streams repeated completion handling. Large INSERTs exceeded SQLite bind-variable limits. | Stage and validate imports before a swap. Consolidate streams. Split inserts into transactional batches |
| [TD-003](#td-003--live-policy-consistency) | P1 | review | Policy caches and writes | Cached URL decisions remained after policy changes. Failed SQLite writes could leave changed live state. | Invalidate dependent caches. Validate and persist changes before updating live state |
| [TD-004](#td-004--interception-fast-path) | P1 | review | Proxy and NetworkManager | The proxy buffered HTML. Requests held DB locks, scanned hosts again, and cloned the catalog. | Stream unaffected responses. Reduce work for each request |
| [TD-005](#td-005--bounded-network-caches-and-attribution) | P2 | review | mitm-proxy / ace-tun | Origin, certificate, and DNS caches had no limits. Windows attribution allocated large buffers. | Limit cache sizes. Reduce Windows allocations. Align native buffers |
| [TD-006](#td-006--control-plane-and-rule-overhead) | P2 | review | API/dashboard and rule engine | The dashboard used an exclusive lock. Rule edits recompiled regexes. Traffic logs were verbose. | Use shared reads, update domains without full compilation, and reduce routine logs |
| [TD-007](#td-007--tunnel-policy-event-driven-evaluation) | P1 | candidate | Tunnel policy lifecycle | Each tunnel still polls policy every 500 ms. | Preserve edit and schedule transitions with change notifications or efficient bounded checks |
| [TD-008](#td-008--regex-interception-scope) | P1 | candidate | TLS interception policy | Active arbitrary regexes can cause MITM inspection of all domains. | Narrow only regexes proven to target a host. Keep the opaque default |
| [TD-009](#td-009--bounded-oversized-html-handling) | P1 | candidate | HTTP response mutation | Eligible HTML collection and decompression have no size limit. | Limit memory without disabling Resistance Mode or breaking large legitimate responses |
| [TD-010](#td-010--http1-upstream-reuse) | P2 | blocked | mitm-proxy upstream | HTTP/1.1 reconnects for each request. A simple sender pool delays active streams. | Design a bounded pool that tracks response-body lifetime. Keep the committed concurrency regression |
| [TD-011](#td-011--windows-pid-table-attribution) | P2 | candidate | Windows TUN attribution | Each flow queries and scans IP-Helper ownership tables. | Test attribution races and PID reuse before reusing table snapshots |
| [TD-012](#td-012--rpc-and-production-h3-footprint) | P3 | candidate | UI RPC and proxy features | The RPC bridge repeats dispatch code. ACE selectively enables HTTP/3 for host-level blocks. | Assess generated dispatch, a reconnecting client, and native H3 release-size gates |
| [TD-013](#td-013--network-block-mutation-lock-guard) | P2 | review | NetworkManager block mutations | Delete, safe-search, and rename operations repeat missing-block and active-lock checks. | Use one block-manager guard. Preserve errors and persistence order |
| [TD-014](#td-014--site-pack-link-persistence-consistency) | P1 | review | NetworkManager site-pack assignments | Attach and detach changed live blocks before SQLite writes that could fail. | Test write failures. Keep DB and memory assignments consistent |
| [TD-015](#td-015--allocation-free-hosts-token-scanning) | P2 | review | Domain blocklist hosts parser | The hosts parser collects whitespace tokens into a temporary `Vec` for each line. | Iterate over tokens directly. Preserve error order and alias validation |
| [TD-016](#td-016--linear-time-tauri-stream-queue) | P2 | review | Ace App Tauri RPC stream | Site-pack progress streams call `Array.shift()` for each event. | Use cursor-based draining. Preserve event and error order |
| [TD-017](#td-017--sveltekit-alias-configuration-warnings) | P2 | candidate | Ace App tooling | SvelteKit reports a deprecated alias setting and overwritten tsconfig paths. | Verify the alias settings before a separate migration. Preserve imports |
| [TD-018](#td-018--linux-inode-attribution-cache-metadata) | P3 | done | ace-tun Linux process attribution | The inode→PID cache stores a timestamp that readers do not use. | Remove the unused value. Preserve endpoint TTL, rescans, and routing |
| [TD-019](#td-019--http3-native-release-and-mtls-gates) | P1 | review | ACE TUN / H3 MITM | Selective host-level H3 interception is implemented. H3 mTLS preflight for request rules and native browser/VPN/WFP tests remain open. | Keep the TCP fallback for other policies. Test disposable Windows and Linux installations before release |

Add a detailed section for each non-placeholder item. Use the template below. Link each queue ID to its section. Order items by maintenance benefit and risk, not by code-size metrics.

### TD-001 — Native IPC integration coverage

- **Status:** verified; **priority:** P1.
- **Area:** `apps/ace-app/e2e/`, `apps/ace-app/src/lib/engine-client/tauri-rpc.test.ts`, `crates/ace-engine/src/service/api/rpc.rs`, `installer/`.
- **Evidence:** Nine removed Playwright suites intercepted the old engine HTTP endpoint at `127.0.0.1:10567`. They did not test production remoc calls. The replacement Tauri adapter unit tests mock `invoke`. Rust remoc tests use an in-process duplex connection.
- **Cost:** Current automated tests can miss regressions in installed Windows Named Pipe ACLs, SCM startup, watchdog recovery, WebView-to-Tauri IPC, and uninstall gating.
- **Smallest improvement:** Add native Windows integration tests for these boundaries. Run the tests in a disposable VM. Keep them separate from static browser smoke tests.
- **Security invariants:** Do not bypass installation capability checks, lock rules, Windows service separation, or uninstall refusal.
- **Verified checks during cleanup:** `cargo check --workspace --all-targets`; `cargo check --release -p ace-engine -p ace-watchdog -p ace-app`; `cargo test -p ace-engine --lib -p ace-watchdog-protocol` (206 + 3 passed); `bun run check`; `bun run test:unit` (39 passed).
- **Dependency:** This work needs a native Windows test harness and a disposable VM. The cleanup did not include release-installation validation.

### TD-002 — Safe and efficient local site pack imports

- **Status:** review; **priority:** P1.
- **Area:** `crates/ace-engine/src/core/network_block/site_pack/{importer/importer.rs,site_pack_db.rs,mod.rs}`, `crates/ace-engine/src/manager/network/site_pack.rs`.
- **Evidence:** The old local resync closed and cleared the live database before parsing. A failed or canceled import could leave enforcement unloaded. `execute_resync` and manager streams repeated forwarding and completion handling. A Diesel INSERT with 50,000 rows exceeded SQLite's bound-variable limits.
- **Change:** Build imports in a unique sibling staging directory. Validate imports and metadata before swapping directories. Use directory renames that permit rollback. Keep the old enforcement data available until the swap. Consolidate local import/resync control and remove one forwarding stream layer. Limit statements to 500 variables. Use one transaction for each 50,000-row batch. Buffer input at 64 KiB. Aggregate warnings for malformed lines. Preserve RPC progress events, sync guards, and remote backup behavior.
- **Verification (Windows workspace):** `cargo check -p ace-engine --lib` passed. `cargo test -p ace-engine --lib resync` passed (2 tests). `cargo test -p ace-engine --lib core::network_block::site_pack` passed (19 tests). No benchmarks ran. The repository-wide `cargo fmt --all --check` reported pre-existing formatting issues elsewhere. Only changed source files were formatted.
- **Residual review:** Independent diff review and installed Windows service/integration tests remain open. There are no measured performance results. Unrelated compiler warnings remain.
- **Owner/branch:** `main`. **Scope:** The engine local importer and shared backup activation changed. Protocol and UI contracts did not change.

### TD-003 — Live policy consistency

- **Status:** review; **priority:** P1. Commits: `5564dd9`, `0616022`, `7bc31a7`.
- **Evidence:** `NetworkBlock::results_cache` previously kept decisions after changes to rules, exceptions, links, and site packs. Block property setters changed memory before SQLite writes that could fail. Rule and exception additions were persisted before complete matchers were compiled.
- **Change:** Invalidate dependent caches after mutations and successful local, remote, or background pack activation. Persist simple block edits before changing memory. Validate complete matchers before persistence. Shared cache handles let asynchronous import streams invalidate caches after activation.
- **Tests:** Test cached allow/block decisions after mutations. Test SQLite write failures triggered by aborts. Test persistence of invalid exceptions. Run the full `cargo test -p ace-engine --lib` suite. **Risk:** Enforcement risk is high. Independent review and installed Windows verification remain open.

### TD-004 — Interception fast path

- **Status:** review; **priority:** P1. Commits: `4e6eab0`, `f6a5cb3`, `e72573a`, `a850523`, `e51f213`.
- **Evidence:** The proxy buffered all top-level HTML, including responses that could not receive injected content. Blocked attempts took an exclusive policy lock for a synchronous SQLite upsert. CONNECT and established-tunnel checks repeated policy scans and rebuilt the script catalog. The supposedly cached Pro flag still read SQLite.
- **Change:** Stream HTML that cannot receive injection without first collecting it. Persist block counts through `spawn_blocking` outside the policy lock. Use an atomic Pro snapshot and synchronize it after DB writes. Avoid cloning the full script catalog. Combine CONNECT host and block evaluation. Keep the 500 ms tunnel polling and schedule behavior unchanged.
- **Tests:** Run the existing proxy and interception tests, the Pro clone/restart regression, and the full engine library suite. **Limit:** No speed benchmarks ran. Eligible HTML response bodies still have no size limit.

### TD-005 — Bounded network caches and attribution

- **Status:** review; **priority:** P2. Commits: `7c19dd1`, `61ad98e`, `ab8d9c2`.
- **Evidence:** MITM leaf-certificate and origin caches had no capacity limits. The DNS snoop map could retain expired entries indefinitely. Each Windows PID lookup allocated a 64 KiB path buffer. IP-Helper table buffers had byte-vector alignment but were cast to typed MIB rows.
- **Change:** Limit leaf certificates to 512 entries, origin protocol/auth hints to 1024 entries, and DNS IP observations to 8192 entries. Prune expired DNS entries when the cache reaches capacity. Use aligned native storage for tables. Start path queries with 512 UTF-16 units. Increase the buffer if it is too small.
- **Tests:** `cargo test -p ace-tun --lib` passed (132 tests). `cargo test -p mitm-proxy --lib` passed (56 tests). Windows process-attribution smoke tests ran. **Risk:** Eviction can cause a cache miss. Native Windows VM review remains open.

### TD-006 — Control-plane and rule overhead

- **Status:** review; **priority:** P2. Commits: `2a0546e`, `7e43bf6`, `973fa72`.
- **Evidence:** The dashboard held an exclusive NetworkManager lock while it fetched per-block SQLite counts. Domain-only rule edits recompiled every regex. Routine flows and requests logged at INFO.
- **Change:** Read policy under a shared lock. Count user rules in memory. Fetch today's blocked count after releasing the lock. Use read-only getters in the schedule and site-pack APIs. Update domain-only rules without recompiling regexes. Move routine traffic logs to DEBUG.
- **Tests:** Run the full engine library suite, domain duplicate/invalid-batch tests, and workspace all-target check. **Limit:** Installed UI behavior was not tested.

### TD-007 — Tunnel policy event-driven evaluation

- **Status:** candidate; **priority:** P1; **scope:** `mitm-proxy/src/http.rs` tunnel termination polling and policy update notifications.
- **Evidence:** Each established tunnel checks policy every 500 ms, even without a policy change. TD-004 made each check less expensive. The repeated checks remain.
- **Risk / gate:** Preserve scheduled transitions, enforcement during a session, reconnect behavior, and the difference between opaque and intercepted tunnels. Add clock-controlled tunnel tests and integration tests before replacing polling.

### TD-008 — Regex interception scope

- **Status:** candidate; **priority:** P1; **scope:** `NetworkBlock::requires_request_inspection_for_host`.
- **Evidence:** An active arbitrary full-URI regex triggers TLS inspection of unrelated hosts. CONNECT metadata cannot safely determine whether a path-dependent regex matches.
- **Risk / gate:** A regex that matches a hostname in the URI path is not host-specific. Narrow inspection only for patterns that conservative analysis proves safe. Characterize these patterns before changing behavior.

### TD-009 — Bounded oversized HTML handling

- **Status:** candidate; **priority:** P1; **scope:** `mitm-proxy/src/http.rs`, `service/proxy/mod.rs`.
- **Evidence:** The proxy collects eligible response bodies without a size limit. It also decompresses compressed HTML without a size limit.
- **Risk / gate:** A simple 413/502 limit would break large sites. Passing large documents through without injection would weaken Resistance Mode. Define and test a fallback and enforcement policy before setting limits.

### TD-010 — HTTP/1 upstream reuse

- **Status:** blocked; **priority:** P2; **scope:** `mitm-proxy/src/upstream.rs`, `tests/proxy_http1.rs`.
- **Evidence:** The proxy can reuse the HTTP/2 origin sender. HTTP/1.1 requests start new handshakes. A small per-origin sender pool reused completed sequential responses. A concurrency test then found head-of-line blocking. A second request waited for the first response, which streamed indefinitely. The pooling change was rolled back during this session. No behavior change shipped.
- **Regression coverage:** Commit `30c7be3` added `busy_streamed_http1_response_does_not_delay_other_requests`. It tests concurrent streamed and regular responses across browser tunnels. `cargo test -p mitm-proxy --test proxy_http1` passes with the existing behavior.
- **Risk / gate:** Return a sender to the pool only after its response body completes or is canceled. Hyper's `Response<Incoming>` type does not support a custom end-of-body callback directly. Before enabling reuse, add a pool or adapter that tracks body lifetime. Test streaming, idle expiry, WebSocket, TLS client certificates, and cancellation.

### TD-011 — Windows PID table attribution

- **Status:** candidate; **priority:** P2; **scope:** `ace-tun/src/platform/windows/process.rs`.
- **Evidence:** Each new flow still queries and scans a new TCP/UDP ownership table. TD-005 improved only memory use and alignment.
- **Risk / gate:** PID/port reuse and process-attribution races can affect enforcement. Test Windows stress cases and negative attribution before caching OS tables.

### TD-012 — RPC and production H3 footprint

- **Status:** candidate; **priority:** P3; **scope:** `service/api/bridge.rs`, `transport.rs`, `mitm-proxy/src/http3.rs`, crate feature graph.
- **Evidence:** The Tauri bridge repeats operation-dispatch code. Each call establishes a new remoc handshake. ACE now enables H3 selectively for hostname-level blocks. Release verification must measure the H3/TUN footprint and native compatibility.
- **Risk / gate:** A persistent client must handle pipe restarts and capability authorization. Feature gates must preserve standalone proxy and test functions. Run native Windows IPC and H3 tests.

**Audit implementation verification (2026-10-19):** `cargo check --workspace --all-targets` passed. `cargo test -p ace-engine --lib -- --test-threads=1` passed (216 tests after all engine changes). `cargo test -p ace-tun --lib` passed (132 tests). `cargo test -p mitm-proxy --tests` passed with one intentionally ignored external-network test. This was before the new concurrent-stream test. `cargo test -p mitm-proxy --test proxy_http1` passed with the new test. One parallel engine run failed because a global resistance-script test fixture was missing. The affected test passed alone. The full suite also passed serially. No performance benchmarks ran, as requested. Existing generated-client, watchdog, and test warnings remain. Native installer/service tests and independent code review did not run.

### TD-013 — Network block mutation lock guard

- **Status:** review; **priority:** P2; **scope:** `crates/ace-engine/src/manager/network/network_blocks.rs`.
- **Evidence:** `delete_network_block`, `set_network_block_safe_search`, and `rename_network_block` each check whether the block exists and whether an active lock protects it. They return the same errors.
- **Cost / change:** Repeated authorization checks increase the risk of maintenance errors. Replace the three checks with one private, narrowly scoped guard. Preserve the strengthening-only condition in `set_network_block_enabled`. Preserve the order of all database writes.
- **Security constraints:** Return `NetworkBlockNotFound` if the block is missing. Return `NetworkBlockIsLocked` if an active lock protects it. On success, persist changes before updating live state. Do not change rule, lock, or IPC behavior.
- **Baseline (Windows, 2026-10-21):** `cargo test -p ace-engine --lib manager::network::network_blocks::tests -- --test-threads=1` passed (7 tests). The generated `ace_server_api/types.rs` had two warnings.
- **Outcome (Windows, 2026-10-21):** Added the private `require_unlocked_network_block` guard only for these three changes. Before refactoring, added two regression tests for missing-block errors and active locks. The tests also check persisted policy after rejected writes. The strengthening operations and DB-before-memory write order did not change.
- **Verification:** Pre-refactor tests passed (9/9 after characterization coverage). Post-refactor focused tests passed (9/9). `cargo test -p ace-engine --lib -- --test-threads=1` passed (221/221). `cargo check -p ace-engine --bin ace-engine` passed. `rustfmt --check --edition 2024 crates/ace-engine/src/manager/network/network_blocks.rs` passed. Two pre-existing generated-client dead-code warnings remain. No Windows installed-service/VM tests ran. The change is limited to a previously tested manager guard.
- **Review:** In-session diff inspection found no changes to public interfaces, error variants, persistence order, or the strengthening-only lock exception. No independent reviewer has reviewed the diff. The status remains `review`, not `done`.

### TD-014 — Site-pack link persistence consistency

- **Status:** review; **priority:** P1; **scope:** `crates/ace-engine/src/manager/network/network_blocks_to_site_pack.rs`. This task is separate from TD-013.
- **Evidence:** Attach and detach previously changed the live block before SQLite writes that could fail. This could make live policy differ from stored policy.
- **Change (2026-10-22):** Check block and pack prerequisites first. Persist the SQLite assignment before updating the in-memory link. Keep lock checks, unsynced rejection, and cache invalidation unchanged.
- **Verification:** Added `site_pack_assignment_sql_failures_do_not_change_live_links`. The test injects SQL failures into add and remove operations. It verifies that live policy does not change. The full serial engine library suite passed (**227/227**). `cargo check -p ace-engine --all-targets --all-features` passed. No installer, SCM, or VM test ran.
- **Residual gate:** Independent security, domain, and architecture diff reviews remain open. Installed-service validation is also open. Keep the status `review`, not `done`.

### TD-015 — Allocation-free hosts token scanning

- **Status:** review; **priority:** P2; **owner:** `main`.
- **Area / evidence:** In `crates/ace-domain-blocklist/src/format/hosts.rs`, `HostsListStandardFormat::parse_line` calls `line.split_whitespace().collect::<Vec<&str>>()`. It uses the resulting collection only to inspect the first token and process the remaining tokens. Imports can process many lines. The temporary collection adds an allocation for each line.
- **Scope / simplest improvement:** Change only this parser. Iterate over `split_whitespace` directly. Preserve redirect-first and alias-second validation, comment handling, alias order, and error variants. Do not add a shared helper, crate, or type.
- **Architecture / domain impact:** Keep ownership in `ace-domain-blocklist::format::hosts`. Do not change the public parsing trait, `DomainName` validator, importer integration, or stored data. Do not add dependency edges, change protocols, or introduce unchecked primitives.
- **Baseline (Windows, 2026-10-22):** `cargo test -p ace-domain-blocklist` passed (54 tests). Existing tests cover comments, redirects, multiple aliases, malformed input, bare IP rejection, and the first invalid domain. Add focused cases for whitespace-only input and long alias lists if needed.
- **Risk / acceptance:** Risk is low. Preserve each `Result` and the error precedence for all inputs. Run the 54 existing tests, new cases, and focused formatting/check commands. To roll back, revert only the parser commit.
- **Outcome (2026-10-22):** Replaced per-line token collection with an iterator. Added a regression for the order of 128 aliases. `cargo test -p ace-domain-blocklist` passed (55/55). `cargo check -p ace-domain-blocklist` passed. `rustfmt --check --edition 2024 crates/ace-domain-blocklist/src/format/hosts.rs` passed. `git diff --check` passed. No benchmarks or installed-service tests ran. In-session architecture review found no new dependency edges or authority changes. Domain/type review confirmed unchanged `DomainName` validation and `DomainLineParsingError`. Behavior review confirmed the same precedence for missing redirects and aliases, valid aliases, and malformed-input errors. No independent reviewer was available in the active CodexPro toolset. Keep the status `review`, not `done`.

### TD-016 — Linear-time Tauri stream queue

- **Status:** review; **priority:** P2; **owner:** `main`.
- **Area / evidence:** In `apps/ace-app/src/lib/engine-client/tauri-rpc.ts`, `rpcStream` removes each queued event with `parsed.shift()`. This removes the first array element. If several events arrive before the UI reads them, each removal moves the remaining elements. Consumers include `src/lib/stores/syncing-store.svelte.ts` and the site-pack import UI.
- **Scope / simplest improvement:** Keep the typed local Tauri adapter API. Use an indexed queue. Reclaim entries already consumed without repeated, quadratic array head shifts. Add asynchronous burst and error regression tests. Do not add third-party libraries or shared state.
- **Architecture / type impact:** Change only the UI stream adapter. Keep the `engine-client` façade, Tauri channel, remoc wire, authentication, error callback, and consumers unchanged. Treat `unknown` event values as opaque Tauri payloads. Do not introduce another payload schema.
- **Baseline (Windows, 2026-10-22):** `bun run test:unit` in `apps/ace-app` passed (39 tests, including 4 RPC-adapter tests). Run `bun run check` and focused tests after implementation.
- **Risk / acceptance:** Asynchronous notification gives this change low-to-moderate risk. Preserve FIFO order, completion, error propagation, callback behavior, all response shapes, and wakeups. Do not lose wakeups. Native remoc stream integration remains part of TD-001.
- **Outcome (2026-10-22):** Replaced repeated `Array.shift()` with an index and occasional reclamation of consumed entries. No new module, transport, or payload type was added. Added regressions for 256 buffered/interleaved events, notification after an asynchronous wait, and callback and error propagation. The new tests passed before the change (6/6 with the first two tests) and after it (7/7 with the wakeup test). `bun run check` passed with 0 code errors or warnings; pre-existing SvelteKit configuration notices remain. `bun run build` passed. Final `bun run test:unit` passed (42/42). Final `bun run check` passed (0 code diagnostics). `git diff --check` passed. `cargo check --workspace --all-targets` passed. No independent fresh-context code reviewer was available. The in-session architecture check found no new cross-layer dependency. The type check found no change to opaque event payloads. The behavior diff check found the existing completion and error order unchanged. Keep the status `review` until independent review.

### TD-017 — SvelteKit alias configuration warnings

- **Status:** candidate; **priority:** P2; **area:** `apps/ace-app/vite.config.js:19-23`, `apps/ace-app/tsconfig.json`, generated SvelteKit TS configuration.
- **Evidence (Windows, 2026-10-22):** `bun run check` and `bun run build` both passed. Both reported `config_option_deprecated_alias` and `tsconfig_invalid` (`paths` overwritten). The configuration passes an `alias` object to `sveltekit(...)`. The inherited TS configuration can override paths. `bun run check` reported zero code diagnostics.
- **Cost / scope:** A future SvelteKit upgrade could break imports or confuse path resolution. Check the current framework guidance and generated TS paths. Test `$lib` and `$engine-client` import resolution. Remove obsolete aliases only after equivalent compile and build tests. Do not change app logic, remoc, or component design.
- **Architecture / type gate:** These aliases are build settings. They do not justify a new module or type. Preserve local source imports and build paths. Keep this migration separate from TD-016. Do not assume a solution before confirming the supported SvelteKit settings.
- **Review / verification:** This is a candidate. Native Tauri and Windows integration tests do not apply. No configuration changes were made.

### TD-018 — Linux inode attribution cache metadata

- **Status:** done; **priority:** P3; **scope:** `crates/ace-tun/src/platform/linux/process.rs`, `ProcessCache::inodes`, and `inode_to_pid`. The scope is Linux only. Engine, proxy policy, rule matching, and Windows/macOS code did not change.
- **Evidence / maintenance cost:** The inode→PID cache used `Option<(HashMap<u64, u32>, Instant)>`. Both readers ignored the stored timestamp. This could suggest a time-based expiry that does not exist.
- **Change:** Use `Option<HashMap<u64, u32>>` only. Keep cache hits, `/proc` rescans on misses, and the same mutex scope. Preserve the separate endpoint→PID cache and its 250 ms TTL. Do not add abstractions, dependencies, types, OS calls, or changes to wire and privilege boundaries.
- **Behavior and verification (2026-11-05):** Before and after the change, `cargo test -p ace-tun --lib --locked` passed (**119/119**). The tests include Linux live TCP/UDP socket attribution and browser/QUIC routing. `cargo check --locked -p ace-tun --all-targets` passed. Browser-process list tests passed (**4/4**). `mitm-proxy` library tests passed (**61/61**) without changes. `git diff --check` passed. The focused Rustfmt check reported pre-existing formatting differences outside the changed lines. The team did not format the full file.
- **Architecture/type/independent review:** The Linux attribution module still owns this logic. No import edge, public interface, IPC model, or validation changed. Separate read-only audits found the unused field. Independent diff review found no new defect and confirmed unchanged cache lookup and refresh paths. This does **not** prove Windows/macOS behavior, installed Engine behavior, traffic interception, or that a third-party browser is "undetectable". Those paths did not change and were not tested.

### TD-019 — HTTP/3 native release and mTLS gates

- **Status:** review; **priority:** P1; **scope:** `crates/ace-tun/src/netstack.rs`, `crates/mitm-proxy/src/http3.rs`, `crates/ace-engine/src/service/proxy/mod.rs`, Windows WFP and Linux TUN routing.
- **Test-first baseline:** Commit `5ecaa00` defines nine TUN decisions and one selective HTTP/3 handshake test. These came before implementation and recorded RED compiler failures. Commit `43fd567` defines complete Linux process names. Independent review led to more RED tests. These cover QUIC migration, UDP Proxy fail-open, complete process names after updates, and the real first-datagram bridge.
- **Architecture:** TUN selects QUIC flows and forwards them to a local UDP endpoint. When proxying, TUN must not forward them to the original origin. MITM controls QUIC TLS, SNI, and HTTP authority checks. Engine controls dynamic hostname policy, the CA, and the listener lifecycle. Do not add a privileged listener or remoc endpoint.
- **Scope restraint:** Allow H3 decryption only for host-level blocked sites. Keep the proven TCP/CONNECT path for hosts that need full HTTP, path, or script inspection. Use that path for client-certificate compatibility too. The H3 certificate resolver is synchronous. It cannot probe upstream client-certificate support during the H3 handshake. That feature needs a separately reviewed design before expanding admission.
- **Verified tests:** Tests cover the UDP flow matrix (IPv4/IPv6, alternate ports, unknown process, migrated QUIC). They cover the in-memory TUN UDP bridge's first datagram and reply. They also cover direct UDP per-datagram QUIC transition guards, DNS false positives, and DNS-flow validation. HTTP/3 tests cover blocked responses without an origin, SNI/authority denial, real Quinn readiness, and fake UDP rejection. Native Windows WFP v6 transitions, Chrome/Brave/Firefox Alt-Svc behavior, VPNs, and ECH remain a **release blocker**. Test these on disposable hosts before release. Rust tests alone do not prove cross-platform production support.
- **Separate runtime caveat:** A root-owned Engine service runs in the Linux test environment. Do not restart or replace this live service. Do not weaken filesystem permissions to run tests.

## Item template

Copy this section for each verified or candidate finding. Record only observed evidence and completed checks. Do not invent evidence or results.

### TD-001 — Short, actionable title

- **Status:** candidate / verified / ready / in_progress / review / done / blocked / rejected
- **Priority:** P0 (urgent correctness/security) / P1 (high cost) / P2 (routine) / P3 (opportunistic)
- **Area and files:** `path/to/file.rs` — symbol/function and line or search identifier
- **Evidence:** Describe the observed duplication, coupling, unnecessary dependency, obsolete path, or test gap. Give specific call sites.
- **Cost:** Explain the maintenance cost or regression risk.
- **Smallest proposed improvement:** Prefer simplification or deletion. If you propose an abstraction, explain why it is necessary.
- **Architecture/type evidence:** Identify ownership and existing or proposed dependency edges. State whether shared code represents one stable concept. Identify risks from weak primitives or invalid states. Explain why each new type, helper, cross-feature import, or crate is necessary. Otherwise, explain why it is not.
- **In scope / out of scope:** Name the exact module boundaries. Do not include unrelated rewrites.
- **Behavior and security invariants:** List the observable contracts to preserve. Include validation, serialization, database compatibility, and trust and privilege boundaries.
- **Baseline / tests needed:** Record commands and observed results. Identify characterization tests to add before edits.
- **Dependencies / blockers:** State `None` or link the affected items.
- **Owner / branch or worktree:** Leave unassigned until work begins.
- **Reviewer and review findings:** Record independent architecture, abstraction, domain, type, behavior, and security reviews. Cite file references. Record unresolved conflicts.
- **Outcome / verification / commit:** Complete this field only after implementation and actual checks.

## Operating rules

1. Read-only audits may add or update backlog items; they must not alter product code.
2. Implementation tasks claim a single ID and keep code changes bounded.
3. Reviewers verify changes independently and note unresolved risks.
4. Mark `done` only after passing the relevant checks, recording any checks that could not run, and completing independent review.
5. Do not silently erase or reprioritize someone else's in-progress task.

# Ace Engine — refactoring audit and remaining backlog

**Last updated:** 2026-10-22
**Repository:** `C:\Projects\ace-project\ace-blocker` (`main`, remaining backlog after incremental changes)
**Scope:** `crates/ace-engine/` (core, manager, DB, services, API/RPC, proxy adapter, lifecycle, CLI, tests, and documentation). Review `ace-domain-blocklist`, `ace-tun`, `mitm-proxy`, and `ace-watchdog` dependencies **only** at engine contracts and integration points. **Ace App is out of scope.**
**Procedure:** Read `AGENTS.md`, `crates/ace-engine/AGENTS.md`, and `docs/ai/{REFACTORING_PLAYBOOK,BOUNDARIES_AND_ABSTRACTIONS,DOMAIN_TYPES,REVIEW_GATES}.md`. Compare with `TECH_DEBT_BACKLOG.md` and `docs/IPC_ARCHITECTURE.md`.

> **Note:** This document tracks **only work that remains**. Completed AE items are no longer in the queue or detailed findings. The audit covered the source tree and the main subsystem flows. It used focused call-site reviews and static searches. It was not a line-by-line review of every generated source file, migration, or test. `verified` means the structure or behavior is directly visible in the files. It **does not** mean that a runtime test reproduced a defect. `candidate` needs more checks. `decision` needs behavior approval before a change. Priority is based on risk and maintenance cost, not line count.

**Approved decisions (2026-10-22):** See [`ACE_ENGINE_REFACTOR_DECISIONS.md`](ACE_ENGINE_REFACTOR_DECISIONS.md). Ace Blocker is in pre-release. **Compatibility with old formats, data, and systems is not required.** The decision document records approved architecture choices. The queue lists unfinished implementation work. The verification matrix lists required cross-cutting reviews and integration tests.

## Requirements that must not change

- Keep network blocks, schedules, locks (including the uninstall gate), safe search, Resistance Mode, and exception priority unchanged. Do not assume a new behavior for storage or load failures without an explicit decision.
- Keep the engine and watchdog as independent services. Keep recovery and maintenance authenticated. Do not give Tauri control of their lifecycle.
- Keep local control on **typed remoc RPC** over Named Pipes or Unix sockets. Keep engine capability authorization and ACL checks. Do not add local HTTP control. Treat the external server and the HTTP(S) proxy as separate systems.
- Do not decrypt traffic from non-browser processes. Keep certificate handling, streaming, cancellation, retry, and current contracts unchanged. Compatibility with old pre-release formats is not required.
- The `src/ace_server_api/` client is generated. **Do not edit it by hand.** Check actual consumers before you remove a binary tool or a protocol that is still in use. Do not add compatibility shims for old versions.

## Components reviewed

| Area | Evidence / entry point | Focus |
| --- | --- | --- |
| Startup and lifecycle | `src/main.rs`, `src/lib.rs`, `src/platform.rs`, `src/watchdog.rs`, `src/lock_check.rs` | Startup, failover, SCM separation, maintenance/uninstall |
| Rules and domain models | `src/core/common/{domain_types,schedule,lock,injectable_scripts}.rs`, `src/core/network_block/{mod,user_defined_rules,regex_rules}.rs` | Type invariants, parsing, schedules, policy cache |
| Site packs | `src/core/network_block/site_pack/{mod,site_pack_db,importer/*}.rs` | Staging, errors, DB, identifiers, downloads |
| Coordination | `src/manager/network/{mod,site_pack,network_blocks,network_blocks_to_site_pack,resistance_mode,schedule,backup}.rs`, `src/manager/account.rs` | DB and memory, locks, separate responsibilities |
| Persistence | `src/db.rs`, `src/db/{schedule_repo,site_pack_repo,...}.rs`, migrations | Transactions, failures, SQL model |
| Service and control | `src/service/api/{rpc,transport,bridge,security,operations_*}.rs` | Trust boundary, authorization, error mapping |
| Updates and proxy | `src/service/{content_updater,bundled_site_packs,proxy/*}.rs`, `src/core/utils/file_download.rs` | Staging, memory limits, locks, content |
| Utilities, tests, and generation | `src/bin/*`, `build.rs`, `docs/rest-api.md`, `Cargo.toml` | Old artifacts, developer tools, coverage, documentation |

## Priority queue

| ID | Priority | Status | Area | Reason / next action |
| --- | --- | --- | --- | --- |
| AE-11 | P1 | implementation, VM pending | Script bundle activation | Journal and startup recovery are implemented. Five simulated crash points and SQL rollback have tests. Real Windows kill/restart tests remain. |
| AE-13 | P2 | implementation, stress pending | Content updater | Service snapshots and a single-flight gate are separate from the metadata mutex. Manual and background paths share the gate. Stress and cancellation tests remain. |
| AE-14 | P2 | implementation, integration pending | API bridge | 27 calls are simpler. A test compares 63 RPC methods. Tauri, status 425, and Windows reconnect tests remain. |
| AE-15 | P2 | partial | Schedule/minute semantics | Seconds and fractions are rejected. Midnight and merge tests pass. DST and integrated round-trip tests remain. |
| AE-17 | P2 | implementation, stress pending | Site-pack importer | Downloads use a bounded downloader. Progress remains available. Ten importer tests pass. Cancellation and multiple-source tests remain. |
| AE-19 | P2 | partial | Test | `OnceLock<TempDir>` owns fixtures. Recovery tests use injected paths. Global Settings overrides remain. |
| AE-26 | P2 | partial | Typed error propagation | Resistance Mode load and updater history now propagate `Err`. DB corruption tests pass. Audit of other consumers and boundaries remains. |

---

## Work that remains

### AE-11 — Script recovery after a crash [P1, partial verification]

- **Implemented:** A persistent journal is written before file renames. Startup reconciles the SQLite version and restores the last working version. Five interruption-point tests and one SQL failure test after the swap passed.
- **Remaining:** Test actual process termination and SCM restart. Test locked Windows file handles, sudden power loss, and unreadable journals. No final VM verification has passed.
- **Gate:** Test repeated crashes and restarts in a Windows VM. Check version and DB consistency, lock enforcement, and filesystem fault injection.

### AE-13 — Updater mutex duration [P2, partial verification]

- **Implemented:** The metadata mutex is held for a short time. Network I/O uses a service snapshot. Manual updates, automatic updates, and script downloads share a single-flight gate. Cache publication is explicit. Local concurrency tests pass.
- **Remaining:** Run stress tests for concurrent network updates, cancellation, account changes, and retries.
- **Gate:** Test concurrent background and manual updates. Test staging cancellation and version races.

### AE-14 — Repeated remoc bridge code [P2, partial verification]

- **Implemented:** A local macro handles 27 identical remoc calls without arguments. The typed transport and JSON responses did not change. A test confirms that the bridge contains all 63 trait methods, including streams.
- **Remaining:** Test payload schema integration, capability checks, authorization, `confirmUnlock` status 425, streams, cancellation, and Windows reconnect.
- **Invariant:** Do not add a JSON protocol or a dynamic RPC framework. Do not change the server contract.

### AE-15 — Minute-level schedule contract [P2, partial verification]

- **Implemented:** Both constructors reject nonzero seconds and nanoseconds with a typed error (option A). Tests define midnight as the end-of-day limit. Merge tests pass. Version formats are already canonical.
- **Remaining:** Test daylight saving time changes and integrated DB/backup round trips. Do not add second-level time precision.
- **Gate:** Test repeated and nonexistent DST times, midnight, day-based locks, and DB persistence.

### AE-17 — Repeated HTTP downloads in importer [P2, partial verification]

- **Implemented:** Importer URLs use `file_download_with_limit`. The 2 GiB limit and integer-percent progress remain unchanged. An HTTP error test was added. Ten importer tests passed.
- **Remaining:** Test multiple sources, cancellation, client disconnects, HTTP content size, and temporary file cleanup.
- **Invariant:** Keep streaming events and per-source errors. Do not add another download abstraction.

### AE-19 — Global test directory overrides [P2, partial]

- **Implemented:** `OnceLock<TempDir>` owns shared directories without `mem::forget`. Five recovery tests use explicit paths instead of global Settings overrides. The library test suite also passed with four threads.
- **Remaining:** `TEST_SITES_DIR`, `TEST_RESISTANCE_SCRIPTS_DIR`, and `TEST_BUNDLED_PACKS_DIR` still use process-global overrides. Gradually inject paths into the components. Do not replace these overrides with thread-local values in asynchronous paths.
- **Gate:** Repeat parallel test suites. Check fixture isolation, cleanup, and removal of dependencies on global state.

### AE-26 — Failures encoded as valid values [P2, partial]

- **Current state:** Dashboard RPCs, counts, site-pack lookups, restore, and update-check RPCs propagate `Result`. They distinguish errors, missing data, and empty values.
- **Implemented (2026-10-22):** `load_resistance_manager` returns `Result` for invalid settings, intensity, or locks. Updater history now loads through a fallible operation during update checks, not at startup. Tests for invalid stored values and failed reads passed.
- **Remaining:** Audit consumers and boundaries that are not yet covered. Preserve the approved startup availability policy.
- **Next refactor:** Return `Err` or typed errors for real failures. Keep expected absence distinct from failure. Apply explicitly approved fail-open policies only at the enforcement boundary. Do not introduce primitive error sentinels. Keep lock and uninstall guarantees unchanged.
- **Gate:** Test query and parse errors, stale views, update-check behavior, locked state, and RPC error mapping.

## Order for remaining work

1. **Recovery and enforcement consistency:** AE-11. Run crash/restart tests in a Windows VM.
2. **Concurrency and RPC coordination:** AE-13 and AE-14. Test single-flight behavior, concurrent updates, and typed dispatch.
3. **Schedule precision and remaining errors:** AE-15 and AE-26. Make contracts and errors explicit. Keep the approved policy unchanged.
4. **Scoped P2 improvements:** AE-17 and AE-19. Reuse download functions carefully and isolate tests.

## Verification matrix to use *before* implementation

| Domain | Minimum tests |
| --- | --- |
| Policy/lock/DB | `cargo test -p ace-engine --lib -- --test-threads=1`, targeted DB write-failure tests, reboot round trip, locked/unlocked |
| RPC/security | `cargo test -p ace-engine --lib` (remoc), Named Pipe DACL/capability in Windows VM, release debug gates |
| Import/backup | Staging/cancellation, current backup, SQLite fault injection, checksum/zip bomb, path traversal, retry |
| Proxy/sitepack | `service::proxy` tests, HTTP/HTTPS tunnel, DOM/script injection, no decryption of non-browser traffic, cache invalidation |
| Watchdog/installer | `cargo test -p ace-watchdog-protocol` and Windows VM SCM/recovery/maintenance/uninstall gate |
| Rust compile | `cargo fmt --all --check` (read-only), `cargo check -p ace-engine --all-targets`, targeted lint without autofix |
| Audit/documentation | **Independent** architecture, domain/type, and correctness diff reviews. Update paths and test gates. |

**Last recorded gates (2026-10-22):** `cargo test -p ace-engine --lib -- --test-threads=1`: **252/252** passed. The four-thread parallel suite had **251/251** passes before the last recovery test was added. `cargo check -p ace-engine --all-targets --all-features` passed. `cargo test -p ace-watchdog-protocol`: **3/3** passed. `git diff --check` passed. Windows VM tests for services and installer did not finish. Real-crash recovery and independent review remain open. `cargo fmt --all --check` reported pre-existing formatting differences in other, unchanged source files. See `ACE_ENGINE_REFACTOR_DECISIONS.md` for product decisions and `TECH_DEBT_BACKLOG.md` for general technical debt.

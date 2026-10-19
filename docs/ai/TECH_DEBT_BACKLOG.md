# Ace Blocker — Technical Debt Backlog

Use [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md) for the required audit, implementation, and independent-review process.

This file tracks **verified, actionable maintainability work**, not a wishlist of speculative rewrites. Add items only after inspecting the relevant code and identifying concrete costs. Audit findings may also be marked `candidate` if they need verification.

## Queue

| ID | Priority | Status | Area | Problem / evidence | Intended outcome |
| --- | --- | --- | --- | --- | --- |
| [TD-001](#td-001--native-ipc-integration-coverage) | P1 | verified | Native IPC testing | Removed browser E2E suites mocked the retired REST port; unit tests alone do not exercise installed Windows services | Add Windows Tauri/remoc/installer E2E coverage |
| [TD-002](#td-002--safe-and-efficient-local-site-pack-imports) | P1 | review | Local site pack importer | Local resync rebuilt the live SQLite DB; multiple wrapper streams repeated terminal handling; huge single INSERTs exceeded SQLite bind-variable limits | Stage and validate before swap, consolidate import streams, and transactionally chunk inserts |

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

## Item template

Copy this section for each verified or candidate finding. Do not fabricate evidence or completion details.

### TD-001 — Short, actionable title

- **Status:** candidate / verified / ready / in_progress / review / done / blocked / rejected
- **Priority:** P0 (urgent correctness/security) / P1 (high cost) / P2 (routine) / P3 (opportunistic)
- **Area and files:** `path/to/file.rs` — symbol/function and line or search identifier
- **Evidence:** Observed duplication, coupling, unnecessary dependency, obsolete path, or test gap. Cite concrete call sites.
- **Cost:** Why this makes maintenance harder or increases regression risk.
- **Smallest proposed improvement:** Favor simplification or deletion; state why a new abstraction is truly needed, if one is proposed.
- **In scope / out of scope:** Explicit module boundaries; no bundled unrelated rewrites.
- **Behavior and security invariants:** Identify the observable contracts to preserve.
- **Baseline / tests needed:** Commands, currently observed results, and any characterization tests to add before edits.
- **Dependencies / blockers:** None or explicit linked items.
- **Owner / branch or worktree:** Unassigned until work starts.
- **Reviewer and review findings:** Independent review record.
- **Outcome / verification / commit:** Fill in only after implementation and actual checks.

## Operating rules

1. Read-only audits may add or update backlog items; they must not alter product code.
2. Implementation tasks claim a single ID and keep code changes bounded.
3. Reviewers verify changes independently and note unresolved risks.
4. Mark `done` only after passing the relevant checks, recording any checks that could not run, and completing independent review.
5. Do not silently erase or reprioritize someone else's in-progress task.

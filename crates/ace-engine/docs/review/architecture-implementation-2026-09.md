# Ace Engine — Launch-safety and simplification implementation report

> **Status update (2026-09-02, tracking removal):** Engine commit `8e5fca5`
> removed the remaining tracking and usage components. These included the
> SIMP-01 dynamic resistance engine, SIMP-04 tracking-script pipeline,
> SVC-13, and report receivers. The tables below keep the historical record.

**Repository:** `C:\Projects\ace-project\ace-engine` (git root: `ace-engine/`, branch `main`)
**Implementation date:** 2026-09 (session)
**Author:** AI coding agent (DeepSeek Harness)
**Input:** `docs/review/architecture-review-2026-09.md` (the authoritative review)
**Scope:** Launch-safety and simplification only. The work did not redesign the engine.

---

## 1. Baseline (Phase 1) and final state

| Check | Baseline (HEAD `3f7413d`) | After this pass |
|---|---|---|
| `cargo fmt --check` | ❌ many files unformatted | ✅ `cargo fmt --check` clean |
| `cargo check --workspace` | ✅ | ✅ |
| `cargo test --workspace --no-fail-fast` | ❌ lib 119/6 failed; `api_test` 33/15 failed | ✅ **lib 135/135; api_test 48/48; ace-domain-blocklist 54/54; resistance_rules_test 1/1** |
| `cargo clippy --workspace --all-targets -- -D warnings` | ❌ 436 lib warnings (gate red) | ✅ clean (incl. `--features dev-tools`) |
| Baseline log | `docs/review/baseline-2026-09.log` (kept) | — |

All 21 baseline failures came from outdated tests. The causes included renamed
routes (`/sites-lists`→`/site-lists` and `/usage`→`/report/usage`), the 90-day
lock cap, seed migration `0021`, and schedule-only activation. Other causes
were two drain-then-assert tests in `progress.rs` and a harness that forces Pro.
This work did not cause these failures. The sections below describe each fix.

---

## 2. Phase 2 — Security and launch blockers

### 2.1 `aceblocker://` protocol handler restricted (SEC-01/SVC-06)

The release (`ace-redirect`) and debug (`main.rs`) protocol handlers forwarded
**all** paths with the capability token. The product review found one site deep
link: `aceblocker://account/api-key#apiKey=...`
(`ace-blocker-site/src/lib/utils.ts`). The site also has a `get-engine-port`
probe. That probe does not reach the engine.

Changes:

- `src/protocol.rs` — new `resolve_forward_target()`: parses the URI, drops the
  fragment, validates the path against the explicit allowlist
  `ALLOWED_FORWARD_PATHS = ["/account/api-key"]`, and rejects:
  - unknown paths (`/system/shutdown`, `/backup/import`, `/network-blocks/...`,
    `/account/logout`, case variants, …);
  - path traversal and malformed paths (`..`, `.`, backslashes,
    percent-encoding, drive prefixes, double/trailing slashes, duplicate `#`,
    control characters);
  - query strings that are not exactly the legacy `apiKey=<value>` pair
    (re-encoded before forwarding).
- `src/bin/ace_redirect.rs` — uses the resolver; rejected URIs print a clear
  message and exit `2` (forward failures exit `1`, success `0`).
- `src/main.rs` (debug handler) — same resolver; the debug 10-minute sleep was
  reduced to 5 s (MAIN-01).
- Unit tests in `src/protocol.rs` cover every required rejection class
  (state-changing routes, traversal/malformed paths, non-protocol URIs,
  malformed queries) and the two allowed forms (fragment + legacy query).

### 2.2 Outbound HTTP timeouts (SVC-01)

Every outbound request now has explicit connect + total timeouts:

- `src/ace_server_api/client.rs` (generated, manual edit documented) —
  `with_config` builds the reqwest client with connect 10 s / total 60 s.
- `src/core/utils/file_download.rs` — new shared `http_client()` (connect 10 s
  / total 60 s), size caps on downloads (2 GiB default, 256 MiB script
  bundles), and the `download_and_verify` helper (moved here from
  `resistance_mode.rs`) with the same policy.
- `src/core/network_block/site_pack/importer/importer.rs` — the URL-source
  download now uses the timeout'd client and enforces a 2 GiB cap
  (API-009-adjacent).
- `ace_redirect.rs` and the debug handler in `main.rs` — connect 5 s / total
  30 s.

An unavailable server can no longer stop engine startup indefinitely. Startup
content initialization no longer runs synchronously (see Section 2.4).

### 2.3 Safe atomic site-pack updates (MGR-01, CORE-012, MGR-14, SVC-17)

The new **stage → validate → atomic commit** pipeline replaces
`update_site_pack_version`. It keeps the pack UUID. MGR-01 caused "Safe Mode
on restart" when the UUID changed but blocks still referred to the old UUID:

- Staging (`ContentUpdateService::stage_site_pack_backup`) — unique temp zip,
  extraction into a staging directory on the same volume as the live packs,
  archive-entry validation (absolute paths / `..` / backslashes / drive
  prefixes rejected — `stage_backup_archive` in `site_pack/mod.rs`), and
  validation that the staged DB opens and contains entries (fail closed on
  empty/corrupt archives).
- Commit (`NetworkManager::commit_site_pack_update`, under the write lock) —
  re-checks the version (concurrent callers see "already applied" and skip),
  re-validates, swaps live→old / staged→live with rollback on every failure
  path (including reload), persists the new version on the **same** DB row,
  cleans up the superseded dir and all temp files (RAII `Drop` on the staged
  types).
- `SitePack::restore_from_backup` (user-triggered remote resync) got the same
  treatment: stage + validate + atomic swap + rollback, with the live DB
  closed only immediately before the swap and re-opened on every failure
  path.
- `NetworkManager::from` now tolerates dangling pack references at load
  (skip + warn) instead of failing into Safe Mode (MGR-01 defense in depth).

Tests cover failed downloads (size cap and content length). They also cover
invalid archives, including no DB entry, a corrupt DB, and an empty DB. In
these cases, the previous pack remains intact. Other tests cover path
traversal, successful replacement, rollback, and temporary-file cleanup.

### 2.4 No network work under the manager write lock (SVC-02, SVC-15, LIF-03, API-02)

- New pipeline in `content_updater.rs`: `stage_pending_updates()` does all
  metadata fetches, downloads, hash verification, and extraction with **no
  manager lock**; `NetworkManager::apply_staged_content_updates()` commits
  under the write lock (disk swaps + DB/memory transitions only).
- `run_background_update_loop` — immediate first pass (replaces the old
  synchronous startup content init), then sleep; per-pass errors are logged
  and retried on the next tick.
- `POST /system/update/content` and `POST /resistance-mode/download-scripts`
  use the same stage-outside/commit-inside pattern, and **propagate
  failures** (API-02: no more 200-on-partial-failure; staging errors are
  reported).
- Startup no longer runs content init synchronously and
  `ContentUpdateService::from` cannot fail (LIF-03: the double-init `.expect`
  panic is gone).
- The tracking-script pipeline (download/extract/version-persist) was removed
  (SIMP-04 — nothing consumed it; the API field stays and reports `false`).
- Concurrency: version re-check under the lock + `concurrent_commits_have_defined_behavior`
  test (two racing commits → exactly one applies); per-item failures cannot
  leave partial state (rollback tests).

---

## 3. Phase 3 — Lock and uninstall coordination

### 3.1 `--check-locks` (uninstall gate)

Audited and kept (it was already read-only, fail-safe, and tested). Added:

- **`busy_timeout`** (SVC-08): `PRAGMA busy_timeout = 2000` right after the
  read-only open, so a mid-commit engine is waited out instead of turning
  into a spurious "cannot verify". New contention test holds `BEGIN EXCLUSIVE`
  for 400 ms and asserts the gate proceeds (exit 0) instead of failing.
- Documentation of the deliberate behaviors in the module header:
  - exit codes `0` (no locks) / `1` (locks active, expirations printed) /
    `2` (cannot verify — fail safe);
  - the tool never contacts the engine (direct read-only DB access);
  - `active_during_schedule` is treated as **always active** by the gate —
    deliberately more conservative than the engine's window-bound evaluator.

### 3.2 `ActiveDuringSchedule` semantics (API-003 / review Q2)

The repository establishes the intended behavior. The review did not need a
speculative redesign. Evidence includes the state name "Active During
Schedule", the evaluator (`Lock::is_active` →
`schedule.is_active_at(now)`), the app label
(`lock_active_during_schedule`), and engine guards that call
`is_locked(now)`. **Enforcement applies only in the schedule window.** The
uninstall gate uses a stricter rule. It cannot evaluate schedules, and
uninstall cannot be reversed. Documentation in `src/core/common/lock.rs`
(variant and `is_active`) and `lock_check.rs` now states this behavior.

### 3.3 Soft-unlock dead-end fixed (API-001)

Users could not remove the default `soft_unlockable` lock. The confirmation
path called `set_lock(Inactive)`. The strictness guard rejected that call.
The fix adds `NetworkManager::redeem_soft_unlock` as the only permitted path
to weaken a soft lock. The challenge flow (initiate → id → cooldown) must
first verify intent. The function then checks state again under the write
lock. It cannot redeem a lock that became stricter. It consumes the
challenge only after success. Unit tests cover redemption, guard integrity,
and escalation protection.

### 3.4 Installer coordination (LIF-01 / review Q4)

Verified in `ace-installer/installer.wxs`:

- The installer **does** wire `--check-locks` (`CA_CheckLocks`,
  `Return="check"`, before any destructive step, condition `REMOVE`).
- The installer **attempts** to stop the service on uninstall
  (`ServiceControl Stop="both" Wait="yes"`), but the engine **refuses SCM
  STOP by design**, so the engine process keeps running (and keeps its
  database/files) after "uninstall" until `POST /system/shutdown` or reboot.

This repository cannot change installer behavior. Section 7 records the
required change in a sibling repository. The installer should call
`POST /system/shutdown` with the capability token. Otherwise, the engine
process can remain until reboot. Active locks already prevent uninstall
through `--check-locks`. The engine's `--uninstall` command now gives a clear
warning if the service still runs after the stop attempt. It no longer
silently removes the registration.

---

## 4. Phase 4 — Restored verification

### 4.1 Tests repaired (TEST-01)

- `network_blocks` unit tests: seed-aware assertions (migration `0021` seeds
  4 default blocks) instead of absolute counts.
- `schedule` lock-constraints test: aligned with the documented keep-or-extend
  rule (identical/stronger edits allowed while locked; coverage-reducing edits
  rejected).
- `test_is_host_blocked` / `test_exceptions_override_blocklist`: blocks are
  activated with an all-day schedule (schedule-only activation semantics);
  the exceptions test now proves the block is genuinely active.
- Integration tests: `/sites-lists`→`/site-lists`, `/usage`→`/report/usage`,
  >90-day locks capped to 30 days, `account_test` asserts the harness-forced
  Pro, `misc_test` dev-route assertions fixed (the client helper had stale
  `/debug/*` paths — also fixed).
- The two `progress.rs` failures were removed with the dead module (see §5).

### 4.2 Test infra made deterministic (TESTINFRA-01)

- The library's test-dir override (`Settings::set_test_sites_dir`) was
  `#[cfg(test)]`-gated, which is **inactive when the library is compiled for
  integration tests** — integration pack I/O silently used the real
  ProgramData directory. The gate was removed (the override only triggers
  when a test sets it).
- Integration tests now share one process-wide temp sites dir (UUID-named
  packs → no collisions) and a shared empty resistance dir, serialized by
  `RESISTANCE_DIR_LOCK` for fixture tests.
- Vacuous tests replaced with real ones: `resistance_rules_test` builds its
  own catalog fixture; `test_resistance_mode_scripts` uses a deterministic
  fixture server; `test_sites_list_fuzz` asserts the FAILED phase;
  `test_network_list_sites_connection` asserts COMPLETED.
- `tests/api_client.rs` moved into `tests/api_test/` (no more duplicate empty
  test target); `usage_test.rs` imports `common` instead of re-compiling it.

### 4.3 Lint and format (TST-001, CI-01)

- Generated client allow-listed (`client.rs` header, documented as kept
  across regenerations).
- The repo's `.clippy.toml` bans `e`/`err`; the remaining ~240 handwritten
  `|e|`/`|err|` bindings were renamed to `error` (word-boundary rename
  outside the generated client), plus targeted fixes (ptr-arg, needless
  borrows, clone-on-copy, module inception, await-holding-lock in
  test-serialization guards, etc.).
- **`cargo clippy --workspace --all-targets --features dev-tools -- -D warnings`
  is clean.**
- `cargo fmt --all --check` is clean.
- `azure-pipelines.yml` now matches the workspace: `cargo fmt --all --check`,
  `cargo test --workspace --no-fail-fast`, clippy with `--workspace
  --all-targets --features dev-tools -D warnings`, and an OpenAPI drift check
  (regenerate `docs/openapi.json` + `git diff --exit-code`).

---

## 5. Phase 5 — Safe simplification

### Deleted (verified no references outside the deleted code)

- **`UpdateManager`** (`src/manager/update/`, ~600 lines + tests) — zero
  production callers; `/system/update/check` already reported `available:
  false` (SIMP-02, DEAD-01).
- **`ProgressTracker`/`ChildProgress`** (`src/core/common/progress.rs`, ~300
  lines; `into_receiver()` was `unimplemented!()`) (SIMP-05, DEAD-05) — its
  two failing tests went with it.
- **Tracking-script pipeline** (SIMP-04): `download_tracking_scripts`,
  `get_latest_tracking_version`, `update_tracking_script_version`,
  `Settings::get_tracking_scripts_dir/_json_path`. The DB column and the API
  field are kept (persistence/API compatibility); both are documented as
  vestigial.
- **`core/programs/**`** (ProgramPaths/ProgramCertificates — dead since the
  program-blocking feature was removed) and the `ProgramList*` error variants
  + their API mappings.
- **Dead accessors**: `get_all_network_block_models`, `is_network_block_active`,
  `set_user_defined_entries`, `remove_schedule_entry` + `Schedule::remove_entry`,
  `SitePack::resync`/`resync_internal`, `get_usage_tracker`,
  `get_resistance_mode_manager`, `PunishmentLock` (+ `Punishment.lock`/
  `get_lock`/`set_lock`/`is_locked`), `switch_to_dynamic*`, resistance
  `get_lock`, `UnlockChallenge::completed_at`/`is_ready`,
  `lock_repo::add_default_lock_to` (was broken — inserted `'INACTIVE'` against
  the 0005 CHECK), `get_dodo_payments_url` (sandbox landmine), and
  `ace-domain-blocklist::parse_file`/`parse_file_auto` (+ their tests).
- **Unused deps**: `x509-parser`, `tokio-stream`, `tokio-util`, `hosts-parser`,
  runtime `prettyplease` (build-dependency copy kept).
- **Repo litter**: stale `.gitignore` entries (`schemas/`, `tests/api_client`),
  stray `crates/ace-domain-blocklist/Cargo.lock`,
  `docs/libs/rocksdb.*` + `docs/libs/hudsucker.*`, `doc_gen.rs` rocksdb entry.

### Dynamic Resistance Mode (SIMP-01)

The API rejects dynamic mode. At startup, the engine converts saved dynamic
configurations to static mode. Dynamic mode is **not part of the current
product**. However, `PunishmentMode::Dynamic` and the dynamic settings
columns remain part of the public API and persistence contract. Removing
them would require app-SDK and migration changes. The chosen rule was
**isolate, do not delete**. Ladder and probation code remains in
`src/manager/resistance/`. It runs **only in dynamic mode**.
`process_usage_report` and `advance_day` return early in static mode
(MGR-04). This avoids unused ladder work and changes to dynamic fields on
each usage report. MGR-06 is also fixed. The restricted usage history
loader formerly read the *same* day 10 times. It now reads 10 distinct days.

### Correctness/doc fixes folded in

- API-010: `set_api_key` utoipa annotation (POST, no bogus path param);
  `docs/openapi.json` regenerated (diff is exactly this fix).
- API-01: `/system/update/check` now reports real local pack versions
  (new `db_get_site_pack_version`).
- GEN-01: `gen-api-client` fails loudly instead of silently skipping.
- BIN-01: `install-cert` no longer advertises a nonexistent `--force`.
- TST-015: stale comment pointing at a nonexistent `security_test.rs`.
- Dev-tool bins (clear-cache, doc-gen, gen-site-packs, gen-api-client,
  gen-openapi-spec, stress-netblocks) are gated behind `--features dev-tools`
  (SIMP-07) — they no longer ship in the release artifact; CI still checks
  them.
- Stale docs corrected: `AGENTS.md` (engine + monorepo root — RocksDB,
  engine-core, ace-hide, generated-client claims), `docs/rest-api.md`
  (RocksDB + `/site-packs` naming), `docs/update-policy.md` (RocksDB,
  tracking pipeline retired, self-update marked not-implemented).

---

## 6. Skills used and how they shaped the work

| Skill | Where it influenced the work |
|---|---|
| `rust-simplification-review` (project-local) | Phase 5 inventory discipline: count uses before deleting; DELETE over refactor; the tracking-pipeline and `PunishmentLock` deletions followed its single-call-site rule |
| `windows-network-lifecycle-analysis` (project-local) | Phase 3 lifecycle matrix: uninstall gate scenarios, service-stop refusal vs installer `ServiceControl`, shutdown ordering |
| `evidence-based-refactoring` (project-local) | Every claim re-verified against source (e.g. the `#[cfg(test)]` test-dir bug, the installer `Stop="both"` finding, `zip`'s sanitize-vs-reject behavior); risk bucketing kept the dynamic-resistance change conservative |
| `rust-best-practices` / `rust-patterns` / `rust-testing` / `rust-async-patterns` (session) | `#[expect]` over `#[allow]`, error propagation (API-02), atomic-swap rollback paths, `try_stream!`/pin discipline, test quality (no vacuous tests) |

The three project-local skills from the review were sufficient. The work
did not require a new skill.

---

## 7. Cross-repository change required (documented, not executed)

**`ace-installer/installer.wxs`** — Before `CA_CheckLocks` on `REMOVE`, the
installer should stop the engine through its API. It must call
`POST /system/shutdown` with `X-Ace-Capability` from the capability token
file. Alternatively, it must warn that the engine process remains until
reboot. The current `ServiceControl Stop="both"` cannot stop the engine.
The engine rejects SCM STOP by design. This is the remaining LIF-01 task.

---

## 8. Open questions carried forward

- **Q1 (dynamic resistance v2 intent):** unresolved; the machinery is now
  isolated and inactive in static mode, so the decision can be made later
  without further cleanup.
- **Q3 (server version format):** the comparator is now dotted-numeric with a
  string fallback; if the server uses a different format (e.g. dates), revisit.
- **Q6 (seed regex over-match):** unchanged (curation is a product decision).
- **Deferred per the review's Plan C:** schema-layer unification, build-time
  client generation in CI, report-route rate limiting, token-gating GETs.

## 9. Acceptance criteria — verified

1. `cargo fmt --all --check` ✅
2. `cargo test --workspace --no-fail-fast` ✅ (135 lib + 48 integration + 54
   domain-blocklist + 1 rules, 0 failures)
3. `cargo clippy --workspace --all-targets --features dev-tools -- -D warnings` ✅
4. `cargo build --release` ✅ (product binaries; dev tools gated)
5. OpenAPI snapshot regenerated and consistent with the router ✅
6. Protocol-handler allowlist unit-tested for all required routes ✅
7. `--check-locks` contention test green ✅
8. Atomic update/concurrency/rollback tests green ✅

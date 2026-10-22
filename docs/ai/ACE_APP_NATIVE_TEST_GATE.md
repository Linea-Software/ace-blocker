# Ace App P1 native IPC and security verification gate (AA-07 / AA-10)

Status (2026-10-22): **native verification blocked** because CodexPro CAP
has no registered disposable Windows VM image. The Tauri adapter now denies
WebView `shutdown` calls before reading the engine capability; Rust unit
tests cover the denial, but installed-system security checks have not run.

## Scope and safety

Execute **only on an isolated, disposable Windows VM** with an Ace Blocker
test installer and synthetic data. Snapshot the VM before every destructive
service/installer scenario. Do not use an active developer workstation or
production configuration. Preserve the LocalSystem engine/watchdog services,
capability credential privacy, named-pipe ACLs and lock-aware uninstall gate.
See `docs/IPC_ARCHITECTURE.md` and `TECH_DEBT_BACKLOG.md#td-001`.

## Required native cases

| Case | Setup / invocation | Assertion |
| --- | --- | --- |
| Clean install | Install MSI as administrator, launch Ace App as standard user | Engine/watchdog are separate LocalSystem services; app connects to remoc |
| Capability | Run normal WebView write and a write without a valid capability | Valid write succeeds when allowed; invalid write fails with typed 401/403; capability never reaches JS |
| Pipe ACL | Inspect `\\.\pipe\AceBlockerEngineControlV1`; test standard and unauthorized/remote access | DACL and engine auth independently reject unauthorized callers |
| Partial save | Inject failed rename, icon, rule, schedule, pack and safe-search RPCs | No false save success; editor reports uncertainty, refreshes authoritative state and refuses stale retry when refresh fails |
| Stream | Import and resync normal, terminal FAILED, premature close and dropped connection | COMPLETED alone signals success; EOF/error never does |
| Reconnect | Restart only engine test service during reads, writes and progress streams | Typed transport error followed by successful reconnect; watchdog still operates independently |
| Lock | Lock a block; attempt deactivation, schedule loosening, delete and shutdown via WebView | Every prohibited mutation is rejected by engine even if JavaScript is compromised |
| Maintenance | Use documented installer `ace-engine --request-shutdown` under allowed and locked conditions | Authorized maintenance succeeds only when policy permits; uninstall remains refused while locked |
| WebView surface | Try an arbitrary `invoke("engine_call", {op:"shutdown",args:{}})` | **After implementation:** a normal WebView request must fail with `status: 403` and `code: desktop_operation_forbidden` before engine connection; confirm in installed Tauri runtime |
| Browser enforcement | Start browser before engine and after engine; lock and restart services | Enforcement and proxy safeguards remain active independently from UI |

For every case: log VM image/snapshot ID, test commit, executable hashes,
command/steps, expected and observed status/code, and redacted console output.
Never log capability bytes. Keep independently executable service/watchdog/
installer checks in their owning test suites rather than copying privileged
logic into the WebView.

## Approved AA-10 restriction and pending native validation

The owner approved option A on 2026-10-22. The Tauri adapter now
rejects `shutdown` in both `engine_call` and `engine_stream`, before reading
the installation capability or connecting to the engine. The public Svelte
SDK and operation map no longer expose it. The privileged engine bridge and
independently authenticated `--request-shutdown` installer path remain intact.
Two focused Rust unit tests passed for WebView denial and ordinary calls.
**Native locked/unlocked VM verification is still required before AA-10 can
be described as security-verified.**

# Ace App P1 native IPC and security verification gate (AA-07 / AA-10)

Status (2026-10-22): **native verification blocked**. CodexPro CAP has no registered disposable Windows VM image. The Tauri adapter rejects WebView `shutdown` calls before it reads the engine capability. Rust unit tests verify this rejection. Security tests on an installed system have not run.

## Scope and safety

Run these tests **only on an isolated, disposable Windows VM**. Use an Ace Blocker test installer and synthetic data. Take a VM snapshot before each destructive service or installer test. Do not use an active developer workstation. Do not use a production configuration. Keep the engine and watchdog as LocalSystem services. Protect the capability credential. Keep the named-pipe ACLs and the lock-aware uninstall gate.
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

For each case, record the VM image and snapshot IDs. Record the test commit and executable hashes. Record the commands and steps. Record the expected and observed status and code. Keep a redacted copy of the console output. Never record capability bytes. Keep service, watchdog, and installer tests in their own test suites. Do not copy privileged logic into the WebView.

## Approved AA-10 restriction and pending native validation

The owner approved option A on 2026-10-22. The Tauri adapter rejects `shutdown` in `engine_call` and `engine_stream`. It rejects the operation before it reads the installation capability or connects to the engine. The public Svelte SDK and operation map do not expose `shutdown`. The privileged engine bridge remains intact. The independently authenticated installer path, `--request-shutdown`, also remains intact. Two focused Rust unit tests passed. They cover WebView denial and ordinary calls. **AA-10 is not security-verified until locked and unlocked tests pass on a native Windows VM.**

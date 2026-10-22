# Ace App P1 native IPC and security verification gate (AA-07 / AA-10)

Status (2026-10-22): **blocked: no disposable Windows VM image is registered
in CodexPro CAP** (`vm images` returned an empty inventory). Nothing in this
document claims that tests were run or the desktop bridge was hardened.

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
| WebView surface | Try an arbitrary `invoke("engine_call", {op:"shutdown",args:{}})` | **Before restriction:** record actual observed authorization result; **after approval/hardening:** normal WebView shutdown must be rejected at Tauri boundary |
| Browser enforcement | Start browser before engine and after engine; lock and restart services | Enforcement and proxy safeguards remain active independently from UI |

For every case: log VM image/snapshot ID, test commit, executable hashes,
command/steps, expected and observed status/code, and redacted console output.
Never log capability bytes. Keep independently executable service/watchdog/
installer checks in their owning test suites rather than copying privileged
logic into the WebView.

## Decision before AA-10 change

The ordinary UI has no discovered source call site of `shutdown`, but the
generic Tauri command `engine_call(op,args)` currently forwards the
`shutdown` operation exposed by the bridge. Removing shutdown from the
*engine* would break the installer maintenance path. Recommended restriction:
deny desktop-originated `shutdown` in the Tauri adapter and stop exposing it
in the Svelte SDK, leaving the engine's independently authenticated installer
request intact. **Do not mark AA-10 done until the user approves the boundary
choice and native locked/unlocked verification succeeds.**

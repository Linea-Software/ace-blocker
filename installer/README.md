# Ace Installer

Packages **ace-engine**, the dedicated **ace-watchdog**, and **ace-app** into a
Windows MSI installer using WiX Toolset v4.

## Prerequisites

- [Rust toolchain](https://rustup.rs/) (for `ace-engine` and `ace-watchdog`)
- [Bun](https://bun.sh/) (for `ace-app`)
- [WiX Toolset v4](https://docs.firegiant.com/wix/), including the Util extension:
  ```powershell
  dotnet tool install -g wix --version 4.0.5
  & "$env:USERPROFILE\.dotnet\tools\wix.exe" extension add -g WixToolset.Util.wixext/4.0.5
  ```

  The major version is pinned intentionally. WiX v7 has different licensing
  and EULA requirements and is not the CLI targeted by this installer source.

## Build

From this unified repository, run the convenience wrapper:

```powershell
.\build.ps1
.\build.ps1 -Version "0.2.0"
```

It builds `crates/ace-engine` and `crates/ace-watchdog` in the root
Cargo workspace and `apps/ace-app` with Tauri. Artifacts come from the
workspace's shared `target/release` directory and are staged for the
canonical packager.

Both Rust projects enable Windows x64 MSVC `+crt-static` in checked-in Cargo
configuration, so standard `cargo build --release` and `bun tauri build` (in
local and CI workflows) require no manual `RUSTFLAGS`. This target-level setting
also applies to Windows debug builds; non-Windows targets are unchanged. Native
libraries must be compatible with static CRT linkage.

CI and other reproducible release environments should call `package.ps1`
directly with explicit inputs:

```powershell
.\package.ps1 `
    -Version "0.2.0" `
    -EngineArtifacts ".\release-input\engine" `
    -AppArtifacts ".\release-input\app"
```

`package.ps1` never reaches into sibling source repositories. It validates and
stages the supplied binaries, verifies the Wintun Authenticode signature,
includes the selected offline site-pack bundle, stamps a generated WiX source,
and writes `dist\AceBlocker-<version>.msi` without modifying
`installer.wxs`.

The packager runs `verify-crt.ps1` over the PE import tables of all five
executables and the signed Wintun DLL and rejects dependencies on MSVC
Redistributable DLLs (for example `VCRUNTIME140.dll`, `MSVCP140.dll`). LLVM's
`llvm-readobj.exe` must be available on the Windows release builder. Windows
10/11-inbox Universal CRT imports such as `api-ms-win-crt-*.dll` can remain;
these do not require a separately installed Visual C++ Redistributable.

## GitHub release candidates

`.github/workflows/release.yml` is manually dispatched from `main` with a
three-part version. It builds a **Windows x64 MSI** and a **Linux x64 tarball**
(including engine, watchdog, Tauri app, and offline site packs), verifies
their SHA-256/provenance, and attaches both to a **draft prerelease** on GitHub.
No macOS builds or automatic public publishing are configured.

The optional `stage_on_site` input stages **only the Windows MSI** with the
Ace website; the site requires an administrator to publish it. It now requires
a valid Authenticode signature matching `ACE_RELEASE_PUBLISHER_CERT_SHA256`.
The workflow does not yet sign the MSI: an approved signing service must be
configured externally before staging can succeed. See `docs/PRODUCT_UPDATES.md`
for the pre-release signed-feed contract and disabled auto-install boundary. Configure
`ACE_RELEASE_INGEST_URL` (repository variable) and
`ACE_RELEASE_INGEST_TOKEN` (repository secret) to enable that step.

See [Desktop CI and release candidates](../docs/RELEASING.md) for the
build matrix, Linux manual installation caveats, and security verification
gates that must pass before publishing a release.

## What gets installed

| File | Purpose |
|------|---------|
| `ace-engine.exe` | `AceEngine` service: blocking, proxy, local API, lock gate, watchdog heartbeat client |
| `ace-watchdog.exe` | `AceEngineWatchdog` LocalSystem service: engine recovery and hang detection |
| `ace-app.exe` | Desktop user interface (Tauri + SvelteKit) |
| `ace-redirect.exe` | URL scheme handler (`aceblocker://`) |
| `install-cert.exe` | MITM root CA certificate installer |
| `wintun.dll` | WireGuard's signed TUN library (loads its kernel component on demand) |
| `bundled-packs/*` | Offline site-pack seed data |
| `shutdown-engine.ps1` | Fail-safe uninstall/upgrade maintenance and graceful-shutdown helper |
| `verify-product-signature.ps1` | Authenticode + pinned publisher-certificate SHA-256 verifier for staged MSI packages |

The obsolete `ace-sentinel` kernel driver is not packaged or modified by this
installer. The userspace watchdog is intentionally not a kernel/PPL substitute.

## Service architecture

The MSI is the canonical owner of production service configuration:

- `AceEngine` -> `ace-engine.exe`, `LocalSystem`, auto-start.
- `AceEngineWatchdog` -> `ace-watchdog.exe`, `LocalSystem`, auto-start.
- Both services receive three SCM restart actions (2-second delay each).
  SCM repeats the third restart indefinitely on further failures. The failure
  counter resets after a failure-free day; watchdog/engine mutual recovery
  retains its existing bounded backoff.
- Failure actions are configured to include clean-but-unexpected service exits.
  Planned engine shutdown reports `SERVICE_STOPPED` before process exit, so it
  is not treated as a failure.
- Both services are configured for unrestricted service SIDs. Their service
  DACLs leave full control to SYSTEM/Administrators and query/status rights to
  ordinary users; ordinary users cannot stop/reconfigure them through the ACL.
- The engine deliberately does not accept ordinary SCM STOP. The watchdog does
  accept STOP so the MSI can remove/update it after the engine has entered
  maintenance and stopped.

`ace-engine.exe --install` remains available for legacy/developer use and keeps
its engine recovery policy aligned with the MSI, but it is not the production
installation path and does not install the watchdog service.

## Watchdog health and recovery

The watchdog treats SCM as authoritative. It combines the `AceEngine` service
state, the SCM-owned service PID, a per-start engine instance ID, and a local
challenge/response heartbeat on:

```text
\\.\pipe\AceBlocker\WatchdogV1
```

The pipe rejects remote clients, is restricted to SYSTEM, and the watchdog
verifies the actual pipe client PID against the current SCM-owned `AceEngine`
PID before accepting it. No API keys, capability tokens, URLs, blocked domains,
user activity, or certificate material are sent over this protocol.

Expected recovery behavior:

- engine crash/kill -> SCM gets the first recovery opportunity; watchdog
  requests `StartService` as a bounded fallback if the service remains stopped;
- watchdog crash/kill -> its own SCM recovery applies, and a live engine also
  asks SCM to start `AceEngineWatchdog` if its pipe disappears;
- engine hang -> startup grace plus multiple missed heartbeat challenges are
  required; the watchdog re-queries the current SCM PID immediately before
  terminating that process, then lets SCM restart it;
- repeated failures -> SCM continues restarting failed services after 2 seconds;
  the watchdog's independent recovery path still caps its own backoff at one minute.

For debug builds only, create
`{ProgramData}\LineaSoftware\AceBlocker\watchdog-heartbeat.pause` (or set
`ACE_WATCHDOG_DEV_DROP_HEARTBEAT`) to suppress heartbeat replies while the
engine process stays alive. Release builds ignore both mechanisms.

## Uninstall and upgrade lifecycle

The lock checks remain authoritative and fail safe. The removal half of an
uninstall/major upgrade is ordered as follows:

1. `ace-engine.exe --check-locks` reads the engine database without booting the
   engine. Exit 0 means no lock; exit 1 means an active network-block or
   Resistance Mode lock; exit 2 means state cannot be verified. Non-zero aborts
   before maintenance, service stop, or file removal.
2. `shutdown-engine.ps1` calls authenticated `POST /system/shutdown`. The engine
   re-checks both lock classes. If locked it returns HTTP 423 and does not notify
   the watchdog.
3. If unlocked, the engine requests and waits for a bounded in-memory watchdog
   maintenance lease (normally three minutes). Failure to get the acknowledgement
   aborts shutdown.
4. The engine performs its existing graceful shutdown and reports
   `SERVICE_STOPPED`. The helper waits up to 30 seconds for SCM to observe the
   stopped state; inability to verify it aborts the MSI action.
5. The MSI stops `AceEngineWatchdog`, then the normal MSI service-removal and
   file-replacement/removal actions can proceed without a resurrection race.
6. On major upgrade the new MSI installs/reconfigures and starts both services.

The helper never logs the capability-token value. Its diagnostic log is
`%TEMP%\AceBlocker-uninstall-shutdown.log`.

## ACL and data policy

The MSI applies conservative allow-only ACLs (no deny ACEs):

- `ace-engine.exe`, `ace-watchdog.exe`, and `wintun.dll`: ordinary users have
  read/execute but not write/replace rights; SYSTEM/Administrators have full
  control.
- `{ProgramData}\LineaSoftware\AceBlocker`: preserved across uninstall;
  SYSTEM/Administrators have full control and ordinary users have inherited
  read/execute but not modify rights.
- service objects: ordinary users can query service state but do not receive
  stop/change-config rights.

These controls target the project's weak-moment model. They do **not** make the
installation resistant to a determined elevated administrator with ownership,
ACL, service-control, debugging, or offline-disk capabilities.

## Offline site packs (`bundled-packs/`)

`bundled-packs/` contains the site packs that ship with the installer so a fresh
offline install works before the engine reaches the internet. The versioned
manifest is the source of truth; `package.ps1` generates `bundled-packs.wxs` from
it during packaging. User-downloaded content under ProgramData is not treated as
immutable and is preserved across uninstall.

Regenerate the bundle from `ace-engine/` with the existing exporter, for example:

```powershell
cargo run --features dev-tools --bin export-site-pack-bundle -- `
    --catalog ..\ace-app\src\lib\data\site-packs.json `
    --out-dir ..\ace-installer\bundled-packs `
    --packs pack-adult-free,pack-social-free,pack-gaming-free `
    --build-date YYYY-MM-DD
```

## Integrity monitoring status

Binary/service integrity monitoring is intentionally **deferred**. The current
binary update/signing path does not yet provide a stable trusted signer/manifest
contract for `ace-engine.exe` and `ace-watchdog.exe`. Enforcing hashes or signer
identity now could strand legitimate upgrades. The safe prerequisite layers
(SCM recovery, separate watchdog, heartbeat, maintenance, and ACL hardening) are
implemented first; no new updater was added.

## Windows validation

Run `verify-watchdog.ps1` from an elevated PowerShell after installing a freshly
built MSI. The default mode inspects service paths/start mode/PIDs, failure
actions, non-crash failure flags, service SID type, service DACLs, and relevant
file/data ACLs. `-ExerciseRecovery` additionally kills each service process once
and verifies SCM/watchdog recovery. See the script output for the remaining
manual lock, hang, reboot, update, uninstall, and elevated-admin checks.

Useful direct inspection commands are also:

```powershell
sc.exe qc AceEngine
sc.exe qfailure AceEngine
sc.exe qfailureflag AceEngine
sc.exe qsidtype AceEngine
sc.exe sdshow AceEngine
sc.exe qc AceEngineWatchdog
sc.exe qfailure AceEngineWatchdog
sc.exe qfailureflag AceEngineWatchdog
sc.exe qsidtype AceEngineWatchdog
sc.exe sdshow AceEngineWatchdog
```

Do not claim end-to-end watchdog correctness until these behaviors have been
exercised on a clean Windows VM, including reboot and major-upgrade scenarios.

## Project structure

```text
ace-installer/
├── build.ps1
├── installer.wxs
├── shutdown-engine.ps1
├── verify-watchdog.ps1
├── bundled-packs/
├── artifacts/          # build staging (gitignored)
├── dist/               # MSI output (gitignored)
└── README.md
```

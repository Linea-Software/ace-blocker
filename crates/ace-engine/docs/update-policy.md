# Ace Engine Update Policy

> **Historical design note (pre-remoc):** The local REST endpoints, port routing, and restart scripts below describe a former design. They are not the current engine control contract. For current remoc RPC over Named Pipes/Unix sockets and the running updater, see `docs/IPC_ARCHITECTURE.md`, `src/service/api/`, and `src/service/content_updater.rs`.

This document defines the update strategy for four Ace Engine assets: **site packs**, **resistance scripts**, **ace-engine** (engine binary), and **ace-app** (client UI).

---

## 📋 Table of Contents

1. [Overview & Scope](#overview--scope)
2. [Content Updates (Hot / In-Place)](#content-updates-hot--in-place)
   - [Update Cadence](#update-cadence)
   - [Site Packs](#site-packs)
   - [Resistance Scripts](#resistance-scripts)
   - [Atomicity & Rollback](#atomicity--rollback)
3. [Binary Updates (Require Restart)](#binary-updates-require-restart)
   - [Engine Self-Update](#engine-self-update)
   - [Certificate Forward-Compatibility](#certificate-forward-compatibility)
   - [Engine Update Flow](#engine-update-flow)
   - [Restart Script](#restart-script)
4. [Ace App Update Notification](#ace-app-update-notification)
5. [Update Lifecycle State Machine](#update-lifecycle-state-machine)
6. [Recovery & Rollback](#recovery--rollback)
7. [API Endpoints](#api-endpoints)
8. [Implementation Plan](#implementation-plan)
9. [Data Structures](#data-structures)

---

## Overview & Scope

Ace Engine manages four asset types in two groups. The groups differ by whether an update interrupts the service:

```
+----------------------------------------------+
|              UPDATE POLICY                   |
+-------------------+--------------------------+
|  CONTENT UPDATES   |    BINARY UPDATES       |
|  (Hot / In-Place)  |   (Requires Restart)    |
+-------------------+--------------------------+
| * Site Lists       | * Ace Engine binary     |
| * Resistance Scripts | * Ace App binary      |
+-------------------+--------------------------+
```

- **Content Updates**: Apply during runtime without stopping the proxy or API service. Swap data atomically in memory.
- **Binary Updates**: Shut down the engine gracefully. Replace the executable on disk. Restart the engine and re-enable the system proxy.

### Server as Source of Truth

The **Ace Blocker server** is the canonical source of version metadata and download URLs. Release uses `aceblocker.com`; debug uses `localhost:5173`. The engine uses generated `ace_server_api::HttpClient` (`src/ace_server_api/`). `AceServerClient` (`src/service/clients/ace_server_client.rs`) wraps this client.

These endpoints support the update flow:

| Endpoint | Used For |
|----------|----------|
| `GET /api/v1/releases/latest` | Bulk version metadata for **all** assets: engine installer, resistance scripts, and site lists. Returns `GetLatestReleasesVersionDataResponse`. |
| `GET /api/v1/scripts/resistance-mode` | Download URL + version for the latest resistance scripts zip. Returns `GetResistanceModeScriptsResponse { url, version }`. |

`ContentUpdateService` in `src/service/updater.rs` uses `GET /api/v1/releases/latest` to compare versions. `NetworkManager::download_resistance_scripts()` uses the dedicated scripts endpoint to download the ZIP bundle.

---

## Content Updates (Hot / In-Place)

### Update Cadence

| Trigger | Cadence | Description |
|---------|---------|-------------|
| **Startup** | Every engine start, if `last_check > 24h` | Implemented in `src/service/content_updater.rs` via `UPDATE_FETCH_THRESHOLD`. Ensures freshness without aggressive polling. |
| **Scheduled (Background)** | Every 24 hours while running | Background `tokio` task spawned in `run()` calls `ContentUpdateService::check_for_updates()` then `NetworkManager::apply_pending_content_updates()`. Uses `tokio::time::sleep` (timer wheel) — **no sleeping thread**. |
| **Manual (API)** | User-initiated | Already exposed via `POST /resistance-mode/download-scripts` and `POST /site-lists/{id}/resync`. |

> [!NOTE]
> In **debug** builds the background check interval can be changed at runtime via  
> `POST /dev/update-check-interval/{seconds}` (see `route_dev.rs`).  
> In **release** builds the interval is hardcoded at 24 hours to avoid hitting the server too aggressively. |

### Site Packs

**Update Mechanism**: Compare each pack version with server metadata.
Use `ContentUpdateService::version_is_newer` for semver-style dotted comparison.

**Flow** (staging + atomic commit; see `src/service/content_updater.rs` and
`docs/review/architecture-implementation-2026-09.md`):

1. `ContentUpdateService::get_site_packs_to_update()` compares local `site_packs` versions with `GetLatestReleasesVersionDataResponse.site_packs`.
2. **Staging (no manager lock)**: For each newer remote pack, download its backup archive to a unique temporary file.
   Enforce download size and time limits. Extract the SQLite database into a staging directory with traversal protection.
   Verify that the database opens and contains entries.
3. **Commit (manager write lock)**: Swap the pack directory atomically (live → old, staged → live).
   Roll back the swap if it fails. Save the new version in the **same** `site_packs` row.
   Keep the pack UUID unchanged to preserve network-block references and join rows.
4. If any step fails, keep the previous valid pack available for use.

**Versioning**: Server versions should follow semver.
Compare dotted numeric segments (`1.9` < `1.10`).
Use a string fallback for nonnumeric segments.

**Offline bundled packs (installer seeds)**: the Windows installer ships a
small set of Free site packs under `[INSTALLFOLDER]\bundled-packs\` together
with a versioned `manifest.json` (produced by the `export-site-pack-bundle`
dev bin — see the monorepo report
`ace-blocker-offline-site-pack-bundling-report.md`). At every startup, before
the network manager is constructed, `service::bundled_site_packs::apply_bundled_packs`
imports them so a fresh **offline** install has no "ghost" packs:

- Verify the manifest and each artifact's SHA-256.
  Reject corrupt or schema-incompatible bundles without registering or importing data.
  Do not fail startup because of these bundles.
- Precedence: `valid newer downloaded pack → valid bundled pack →
  unavailable`. A row already backed by usable local rules is kept unless the
  bundled content version is a strictly-newer bundle-style (`YYYY.MM.DD`)
  version; versions from the server that are not bundle-style are never
  overwritten (the bundle only fills gaps). Packs the user ignored are never
  resurrected.
- Mark packs installed from the bundle with a `.bundled` file beside `pack.db`.
  Online updates and user resync replace the whole directory and clear the marker.
- Online, the normal per-pack version flow then updates bundled rows like any
  other pack (a bundled row records its content version, so a newer server
  release is picked up by `version_is_newer`).

### Resistance Scripts

**Update Mechanism**: Compare version strings and check whether the directory exists.

**Flow**:
1. `ContentUpdateService::resistance_scripts_require_update()` compares `resistance_mode_settings.scripts_version` with `GetLatestReleasesVersionDataResponse.resistance.version`.
2. **Staging (no manager lock)**: Download the bundle with SHA-256 verification, size limits, and retries.
   Extract it to a staging directory with traversal protection.
   Require the `scripts-data.json` catalog.
3. **Commit (manager write lock)**: Swap the scripts directory atomically (live → old, staged → live).
   Run `reload_scripts()` to rebuild the in-memory tree. Restore the previous tree if parsing fails.
   Save the new version to memory **and** SQLite only after all steps succeed.

### Tracking Scripts

**Removed (2026-09-02)**: Usage and tracking scripts are no longer updatable assets.
The tracking-script bundle and `GET /api/v1/scripts/tracking` endpoint were removed.
The latest-release `tracking` field and `tracking_scripts_need_update` response field were removed.
The `tracking_script_version` settings column was also removed.
The engine does not download or apply tracking bundles.
See the tracking-removal implementation report for the complete deletion map.

### Atomicity & Rollback

| Asset | Atomicity Strategy | Rollback on Failure |
|-------|-------------------|---------------------|
| **Site Packs** | Temp download/extract → validate → directory swap (same UUID) | Previous pack untouched until the swap; rollback renames it back |
| **Resistance Scripts** | Temp download/verify/extract → directory swap → reload | Fall back to old `InjectableScriptTree` and old directory |

---

## Binary Updates (Require Restart)

### Engine Self-Update

**Not implemented.** The `UpdateManager` state machine was removed as unused code (SIMP-02).
No production caller existed. `GET /system/update/check` reports `available: false`.
Engine self-update remains future work.
The following sections describe the intended design, which was never implemented.

The running engine cannot replace itself in place for these reasons:

1. **Proxy shutdown**: Disable the system proxy (`sysproxy`). Release ports and drain active connections gracefully.
2. **Certificate continuity**: Keep the Root CA certificate valid and trusted across versions.
3. **Database consistency**: Write all in-memory state to SQLite before process exit.

### Certificate Forward-Compatibility

`install_certificate()` in `src/service/proxy.rs` generates the Root CA certificate on first run.
The engine installs it in the machine trust store.
For update compatibility, preserve these requirements:

- **Validity Period**: The CA certificate stays valid for **100 years** after generation. `install_certificate()` in `src/service/proxy.rs` uses `chrono::Utc::now()` to calculate this period. It sets `not_before` and `not_after` relative to the system time at generation. This avoids certificate regeneration during the expected product lifetime.
- **Persistence**: Store the certificate (`ace_ca.pem`) and private key (`ace_ca.key`) in `{app_dir}/certs/`. Never delete them during updates.
- **Compatibility Gating**: Server release metadata includes `cert_generation_version`. If a new engine version requires a new certificate, `ContentUpdateService` flags it. Require explicit user confirmation.
- **If certificate regeneration becomes necessary**, the engine will:
  1. Generate the new CA **alongside** the old one (never delete the old cert).
  2. Expose a `requires_cert_reinstall: true` flag in the update check response.
  3. The ace-app must display a dedicated UI guiding the user to re-trust the new certificate.

### Engine Update Flow

```
+------------+       +----------------+       +------------------------+
|  ace-app   |------>|  REST API      |------>|  ContentUpdateService  |
|  (UI)      |       |  /system/update|       |  / UpdateManager       |
+------------+       +----------------+       +-------+----------------+
                                                       |
                          +----------------------------+
                          v
+----------------------------------------------------+
|  Update Orchestrator                               |
|  +----------------------------------------------+  |
|  | 1. Check cert compatibility                  |  |
|  | 2. Download new binary from CDN              |  |
|  | 3. Verify checksum/signature                 |  |
|  | 4. Stage binary (ace-engine.new.exe)         |  |
|  | 5. Write restart script                      |  |
|  | 6. Notify ace-app: update ready              |  |
|  | 7. Wait for shutdown signal                  |  |
|  | 8. Graceful shutdown                         |  |
|  | 9. Restart script replaces + launches        |  |
|  +----------------------------------------------+  |
+----------------------------------------------------+
```

**Detailed Steps**:

1. **Check**: `ContentUpdateService::get_latest_installer_version()` compares `env!("CARGO_PKG_VERSION")` against the server's `GetLatestReleasesVersionDataResponse.installer.version`. Semver comparison.

2. **Certificate Compatibility Check**: Before download, check `cert_generation_version` in release metadata. Determine whether the new version requires certificate regeneration.

3. **Download**: Get the new binary from the CDN URL in the server release response. Stream it to a temporary file.

4. **Verify**: Compare the SHA-256 checksum with release metadata. On Windows, also verify the code signature.

5. **Stage**: Write the verified binary as `ace-engine.new.exe` beside the running executable. Windows cannot delete a running `.exe`, but it can rename it.

6. **Prepare Restart Script**: Write a platform-specific script that:
   - Waits for the old process to fully exit (polling loop).
   - Renames `ace-engine.exe` → `ace-engine.old.exe` (backup).
   - Renames `ace-engine.new.exe` → `ace-engine.exe`.
   - Launches the new `ace-engine.exe` with the same arguments.

7. **Notify ace-app**: Return `{ state: "ready", requires_restart: true }` from the API. The app can then show a "Restart Now" button.

8. **Graceful Shutdown** (triggered by `POST /system/shutdown`):
   - Disable system proxy via `sysproxy::Sysproxy::set_system_proxy()`.
   - Flush all database writes (SQLite WAL checkpoint, RocksDB flush).
   - Exit with code 0.

9. **Restart**: The restart script takes over, swaps binaries, and launches the new version.

### Restart Script

**Windows** (`restart.bat`, written to a temp directory):

```batch
@echo off
setlocal
set ENGINE_DIR=%~dp0
set PID=%1

:wait
timeout /t 1 /nobreak >nul
tasklist /fi "PID eq %PID%" 2>nul | find "%PID%" >nul
if %errorlevel% equ 0 goto wait

move /Y "%ENGINE_DIR%\ace-engine.exe" "%ENGINE_DIR%\ace-engine.old.exe"
move /Y "%ENGINE_DIR%\ace-engine.new.exe" "%ENGINE_DIR%\ace-engine.exe"
start "" "%ENGINE_DIR%\ace-engine.exe"
del "%~f0"
```

Before exit, the engine starts this script as a detached process. It passes its PID so the script can detect engine exit.

---

## Ace App Update Notification

The engine does **not** update ace-app. The app updates its own binary. The engine provides version information through the API.

- The engine polls the server for the latest ace-app version and includes it in `GET /system/update/check`.
- The response provides the `download_url` and `checksum` for the latest ace-app installer.
- The ace-app consumes this information and implements its own download-and-replace flow.

---

## Update Lifecycle State Machine

```
                         +--------+
                         |  IDLE  |
                         +---+----+
                             | check_for_updates()
                             | (startup + every 6h)
                             v
                       +------------+
                       | CHECKING   |
                       +-----+------+
                             |
              +--------------+--------------+
              v              v              v
        +---------+   +---------+   +-------------+
        | CONTENT |   | ENGINE  |   |   UP TO     |
        | AVAILABLE|  | AVAILABLE|  |   DATE      |
        +----+----+   +----+----+   +-------------+
             |              |
             v              v
        +---------+   +-------------+
        |DOWNLOAD |   |  DOWNLOAD   |
        | CONTENT |   |  ENGINE     |
        +----+----+   +------+------+
             |               |
             v               v
        +---------+   +-------------+
        | APPLIED |   |   STAGED    |---> Notify ace-app
        | (atomic)|   | (ready to   |     via API response
        +---------+   |  restart)   |
                      +------+------+
                             |
                             v
                      +-------------+
                      |  AWAITING   |
                      |  RESTART    |---> POST /system/shutdown
                      +------+------+
                             |
                             v
                      +-------------+
                      |  SHUTDOWN   |---> Restart script takes over
                      | (graceful)  |
                      +-------------+
```

---

## Recovery & Rollback

| Scenario | Strategy |
|----------|----------|
| **Content update fails (any type)** | Keep old data in memory. Log a warning. Retry on the next check cycle (6h or next startup). No user-visible impact. |
| **Engine binary download fails** | Partial download is discarded. Retry with exponential backoff: 1 min, 5 min, 15 min, then daily. |
| **Engine binary checksum mismatch** | Delete the staged binary. Log an error. Do NOT proceed to restart. Retry download on next check. |
| **New engine binary fails to start** | The old binary `ace-engine.old.exe` is kept. If the new one fails to start 3 times within 60 seconds, the watchdog restores the old binary and launches it. |
| **Certificate broken by update** | The old `ace_ca.pem` and `ace_ca.key` are **never deleted** during an update. The new binary loads the existing certs from `{app_dir}/certs/`. |
| **Database migration failure on new version** | If the new engine binary runs pending migrations that fail, it will not start successfully. The old binary can be restored (see above). |

---

## API Endpoints

All new endpoints are under the `/system` route group (`src/service/api/route_system.rs`).

### `GET /system/update/check`

Return the current update status for all assets.

**Response**:
```json
{
  "engine": {
    "currentVersion": "0.1.0",
    "latestVersion": "0.2.0",
    "available": true,
    "downloadSize": 5242880,
    "requiresCertReinstall": false
  },
  "app": {
    "currentVersion": "1.0.0",
    "latestVersion": "1.1.0",
    "available": true,
    "downloadUrl": "https://cdn.aceblocker.com/releases/ace-app-1.1.0.exe",
    "downloadSize": 10485760,
    "checksum": "sha256:abc123..."
  },
  "content": {
    "siteListsNeedingUpdate": [
      { "id": "al-123", "name": "Adult Content", "currentVersion": "1.0", "latestVersion": "1.1" }
    ],
    "resistanceScriptsNeedUpdate": false
  }
}
```

### `POST /system/update/engine`

Start the engine binary download and staging process.

**Response (202 Accepted)**:
```json
{
  "state": "downloading",
  "progress": 0.0
}
```

### `GET /system/update/engine/status`

Read the current engine update state.

**Response**:
```json
{
  "state": "downloading|staging|ready|error",
  "progress": 0.0,
  "error": null
}
```

When `state` is `"ready"`, ace-app should prompt the user to restart the engine.

### `POST /system/shutdown`

Shut down the engine gracefully.

- Disables the system proxy.
- Flushes all pending database writes.
- If an engine update is staged, writes the restart script and spawns it as a detached process.
- Returns `200 OK` before exiting.

### `POST /system/update/content`

Check for content updates immediately and apply all pending updates. Return the `content` part of the update check response.

---

## Implementation Plan

Follow this implementation order. It uses `ContentUpdateService` in `src/service/updater.rs` and `UpdateManager` in `src/manager/update/`:

### Phase 1: Refactor ContentUpdateService (DONE)

- Renamed `Updater` to `ContentUpdateService` to clarify its scope (content only).
- Created `UpdateManager` in `src/manager/update/` for the binary update lifecycle.
- Updated all call sites: `lib.rs`, `NetworkManager::initialize_content_updates()`, `re_enable_all_remote_site_lists()`.

### Phase 2: Add Update Check API Endpoints

- `GET /system/update/check` — aggregates all version info from `ContentUpdateService` and `UpdateManager`.
- `GET /system/update/engine/status` — exposes the engine update state machine.

### Phase 3: Add Periodic Background Checks (DONE)

- Spawn a `tokio::spawn` loop in `run()` that calls `ContentUpdateService::check_for_updates()` every 24 hours, then delegates to `NetworkManager::apply_pending_content_updates()`.
- Uses `tokio::time::sleep` (timer wheel) — no sleeping OS thread.
- In **debug** builds the interval is configurable via `POST /dev/update-check-interval/{seconds}`.
- On content available, applies automatically.
- On engine update available, just sets the state (no automatic download).

### Phase 4: Engine Binary Download & Staging

- `POST /system/update/engine` — downloads, verifies, and stages the new binary.
- Certificate compatibility check before download.
- Restart script generation.

### Phase 5: Graceful Shutdown

- `POST /system/shutdown` — clean proxy disable, DB flush, restart script spawn, exit.

### Phase 6: Recovery & Watchdog

- The watchdog monitors engine health and can restore the old binary on repeated failures.

---

## Data Structures

These types define the update subsystem.
Binary update implementations are in [`src/manager/update/`].
Content update implementations are in [`src/service/updater.rs`].

### `UpdateState` (in `src/manager/update/state.rs`)

```rust
/// Represents the current state of the engine binary update lifecycle.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    /// No update in progress.
    Idle,
    /// Currently checking the server for new versions.
    Checking,
    /// A new engine binary is being downloaded.
    Downloading {
        version: String,
        progress: Percentage,  // 0.0 to 100.0, clamped
    },
    /// The download is complete; binary is being written to staging.
    Staging {
        version: String,
    },
    /// The binary has been staged on disk; ready to restart.
    Staged {
        version: String,
    },
    /// An error occurred during the update process.
    Error {
        message: String,
        retryable: bool,
        retry_count: u32,
    },
}
```

### `ContentUpdateService` (in `src/service/updater.rs`)

The renamed `Updater` — owns the version metadata cache (`GetLatestReleasesVersionDataResponse`)
and exposes comparison methods:
- `resistance_scripts_require_update()`
- `get_site_lists_to_update()`
- `get_latest_installer_version()` (shared with `UpdateManager`)

### `UpdateManager` (in `src/manager/update/mod.rs`)

Owns the binary update state machine, download staging paths, and restart script generation.
See [`UpdateManager`](src/manager/update/mod.rs) for the full API.

### `UpdateCheckResponse` (API schema — future)

```rust
pub struct UpdateCheckResponse {
    pub engine: Option<BinaryUpdateInfo>,
    pub app: Option<BinaryUpdateInfo>,
    pub content: ContentUpdateInfo,
}

pub struct BinaryUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub download_url: Option<String>,    // None for engine (handled internally)
    pub download_size: u64,
    pub checksum: Option<String>,
    pub requires_cert_reinstall: bool,   // Engine only
}

pub struct ContentUpdateInfo {
    pub site_lists_needing_update: Vec<SiteListUpdateEntry>,
    pub resistance_scripts_need_update: bool,
}

pub struct SiteListUpdateEntry {
    pub id: String,
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
}

pub struct EngineUpdateStatusResponse {
    pub state: String,         // "idle" | "downloading" | "staging" | "ready" | "error"
    pub progress: f32,         // 0.0 to 1.0
    pub error: Option<String>, // Only set when state is "error"
}
```

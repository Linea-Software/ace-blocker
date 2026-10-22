# Ace Engine Update Policy

> **Historical design note (pre-remoc):** The local REST endpoints, port routing and restart scripts below describe a former design, not the current engine control contract. For current typed remoc RPC on Named Pipes/Unix sockets and the running updater, see `docs/IPC_ARCHITECTURE.md`, `src/service/api/` and `src/service/content_updater.rs`.

This document defines the comprehensive update strategy for all updatable assets in the Ace Engine ecosystem: **site packs**, **resistance scripts**, **ace-engine** (the engine binary), and **ace-app** (the client UI).

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

The Ace Engine manages four distinct types of updatable assets, divided into two scopes based on whether they require a service interruption:

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

- **Content Updates**: Can be applied at runtime without interrupting the proxy or API service. Data is swapped atomically in memory.
- **Binary Updates**: Require a graceful shutdown of the engine process, replacement of the executable on disk, and a clean restart that re-enables the system proxy.

### Server as Source of Truth

The **Ace Blocker server** (`aceblocker.com` in release, `localhost:5173` in debug) is the canonical source for all version metadata and download URLs. The engine communicates with the server through the generated `ace_server_api::HttpClient` (located in `src/ace_server_api/`), wrapped by `AceServerClient` (`src/service/clients/ace_server_client.rs`).

Three key endpoints serve the update flow:

| Endpoint | Used For |
|----------|----------|
| `GET /api/v1/releases/latest` | Bulk version metadata for **all** assets: engine installer, resistance scripts, and site lists. Returns `GetLatestReleasesVersionDataResponse`. |
| `GET /api/v1/scripts/resistance-mode` | Download URL + version for the latest resistance scripts zip. Returns `GetResistanceModeScriptsResponse { url, version }`. |

The `ContentUpdateService` in `src/service/updater.rs` already consumes `GET /api/v1/releases/latest` to drive all version comparisons. The `download_resistance_scripts()` method in `NetworkManager` uses the dedicated script endpoint to fetch the actual zip bundle.

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

**Update Mechanism**: Per-pack version comparison against the server's metadata
(semver-style dotted comparison, `ContentUpdateService::version_is_newer`).

**Flow** (staging + atomic commit; see `src/service/content_updater.rs` and
`docs/review/architecture-implementation-2026-09.md`):

1. `ContentUpdateService::get_site_packs_to_update()` compares local versions
   (from the `site_packs` table) against `GetLatestReleasesVersionDataResponse.site_packs`.
2. **Staging (no manager lock)**: for each pack where the remote version is
   newer, the backup archive is downloaded to a unique temp file (bounded
   size, timeouts), the SQLite database entry is extracted into a staging
   directory with traversal protection, and the staged database is validated
   (must open and contain entries).
3. **Commit (manager write lock)**: the pack directory is swapped atomically
   (live → old, staged → live, with rollback on failure) and the new version
   is persisted on the **same** `site_packs` row. The pack UUID never
   changes, so network-block references and join rows stay valid.
4. On failure at any point, the previous valid pack remains in place and
   fully usable.

**Versioning**: Server versions should follow semver. Comparison uses a
dotted numeric comparator (`1.9` < `1.10`) with a string fallback for
non-numeric segments.

**Offline bundled packs (installer seeds)**: the Windows installer ships a
small set of Free site packs under `[INSTALLFOLDER]\bundled-packs\` together
with a versioned `manifest.json` (produced by the `export-site-pack-bundle`
dev bin — see the monorepo report
`ace-blocker-offline-site-pack-bundling-report.md`). At every startup, before
the network manager is constructed, `service::bundled_site_packs::apply_bundled_packs`
imports them so a fresh **offline** install has no "ghost" packs:

- The manifest and each artifact's SHA-256 are validated; corrupt or
  schema-incompatible bundles are rejected safely (nothing is registered or
  imported, startup never fails).
- Precedence: `valid newer downloaded pack → valid bundled pack →
  unavailable`. A row already backed by usable local rules is kept unless the
  bundled content version is a strictly-newer bundle-style (`YYYY.MM.DD`)
  version; versions from the server that are not bundle-style are never
  overwritten (the bundle only fills gaps). Packs the user ignored are never
  resurrected.
- Packs installed from the bundle are marked with a `.bundled` marker file
  next to `pack.db`; any engine directory swap (online update / user resync)
  replaces the whole directory and clears the marker.
- Online, the normal per-pack version flow then updates bundled rows like any
  other pack (a bundled row records its content version, so a newer server
  release is picked up by `version_is_newer`).

### Resistance Scripts

**Update Mechanism**: Version string comparison + directory existence check.

**Flow**:
1. `ContentUpdateService::resistance_scripts_require_update()` compares `resistance_mode_settings.scripts_version` against `GetLatestReleasesVersionDataResponse.resistance.version`.
2. **Staging (no manager lock)**: the bundle is downloaded (SHA-256 verified,
   bounded size, retries) and extracted into a staging directory with
   traversal protection; the `scripts-data.json` catalog must be present.
3. **Commit (manager write lock)**: the scripts directory is swapped
   atomically (live → old, staged → live), `reload_scripts()` rebuilds the
   in-memory tree (falling back to the previous tree on parse failure), and
   the new version is persisted to memory **and** SQLite only after
   everything succeeded.

### Tracking Scripts

**Removed (2026-09-02)**: usage/tracking scripts are not an updatable
asset. The tracking-script bundle, the `GET /api/v1/scripts/tracking`
endpoint, the `tracking` field in the latest-release payload, the
`tracking_scripts_need_update` response field, and the
`tracking_script_version` settings column were all removed; nothing in the
engine downloads or applies a tracking bundle. See the tracking-removal
implementation report for the full deletion map.

### Atomicity & Rollback

| Asset | Atomicity Strategy | Rollback on Failure |
|-------|-------------------|---------------------|
| **Site Packs** | Temp download/extract → validate → directory swap (same UUID) | Previous pack untouched until the swap; rollback renames it back |
| **Resistance Scripts** | Temp download/verify/extract → directory swap → reload | Fall back to old `InjectableScriptTree` and old directory |

---

## Binary Updates (Require Restart)

### Engine Self-Update

**Not implemented.** The `UpdateManager` state machine was removed as unwired
dead code (SIMP-02) — no production caller existed, and
`GET /system/update/check` reports `available: false` honestly. Engine
self-update is future work; the sections below describe the intended (never
implemented) design and are kept only as a reference.

Reasons the engine cannot update itself in-place while running:

1. **Proxy must be shut down cleanly**: The system proxy must be disabled (`sysproxy`), ports released, and active connections drained gracefully.
2. **Certificate continuity**: The Root CA certificate must remain valid and trusted across versions.
3. **Database consistency**: All in-memory state must be flushed to SQLite before the process exits.

### Certificate Forward-Compatibility

The Root CA certificate is generated on first run (in `install_certificate()` within `src/service/proxy.rs`) and installed into the machine's trust store. To ensure seamless updates:

- **Validity Period**: The CA certificate is valid for **100 years** from the generation date (computed dynamically via `chrono::Utc::now()` in `install_certificate()` within `src/service/proxy.rs`). The `not_before` and `not_after` dates are calculated relative to the current system time at generation, ensuring no certificate regeneration is needed for the foreseeable lifetime of the product.
- **Persistence**: The certificate (`ace_ca.pem`) and private key (`ace_ca.key`) are stored in `{app_dir}/certs/` and are never deleted during updates.
- **Compatibility Gating**: The server's release metadata includes a `cert_generation_version` field. If a future engine version requires a new certificate (e.g., algorithm change), the `ContentUpdateService` will flag this and require explicit user confirmation.
- **If cert regeneration is ever required**, the engine will:
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

2. **Certificate Compatibility Check**: Before downloading, verify that the new version does not require a certificate regeneration (via `cert_generation_version` in release metadata).

3. **Download**: Fetch the new binary from the CDN URL provided by the server's release endpoint. Stream the download to a temporary file.

4. **Verify**: Check the SHA-256 checksum (and code signature, on Windows) against the values provided in the release metadata.

5. **Stage**: Write the verified binary to `ace-engine.new.exe` in the same directory as the running executable. On Windows, the running `.exe` cannot be deleted but CAN be renamed, so this staging approach works.

6. **Prepare Restart Script**: Write a platform-specific script that:
   - Waits for the old process to fully exit (polling loop).
   - Renames `ace-engine.exe` → `ace-engine.old.exe` (backup).
   - Renames `ace-engine.new.exe` → `ace-engine.exe`.
   - Launches the new `ace-engine.exe` with the same arguments.

7. **Notify ace-app**: The API returns `{ state: "ready", requires_restart: true }`. The ace-app can then offer the user a "Restart Now" button.

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

The engine spawns this script as a detached process just before exiting, passing its own PID so the script can wait for the exit to complete.

---

## Ace App Update Notification

The engine itself does **not** update ace-app — the app is responsible for its own binary update. However, the engine **facilitates** the update by exposing version information through the API.

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

Returns the current update status for all assets.

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

Initiates the engine binary download and staging process.

**Response (202 Accepted)**:
```json
{
  "state": "downloading",
  "progress": 0.0
}
```

### `GET /system/update/engine/status`

Polls the current state of the engine update process.

**Response**:
```json
{
  "state": "downloading|staging|ready|error",
  "progress": 0.0,
  "error": null
}
```

When `state` is `"ready"`, the ace-app should prompt the user to restart.

### `POST /system/shutdown`

Performs a graceful shutdown of the engine.

- Disables the system proxy.
- Flushes all pending database writes.
- If an engine update is staged, writes the restart script and spawns it as a detached process.
- Returns `200 OK` before exiting.

### `POST /system/update/content`

Triggers an immediate content update check and applies all pending content updates. Returns the same `content` portion of the update check response.

---

## Implementation Plan

The following is the recommended implementation order, building on the existing `ContentUpdateService` in `src/service/updater.rs` and the new `UpdateManager` in `src/manager/update/`:

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

These are the key types used by the update subsystem.  The implementations
live in [`src/manager/update/`] (binary updates) and
[`src/service/updater.rs`] (content updates).

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

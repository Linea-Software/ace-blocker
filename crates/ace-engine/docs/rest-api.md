# Ace Engine REST API Documentation

The Ace Engine exposes a REST API for managing network blocks, sites lists, schedules, and more. The API is Axum-based and uses camelCase for JSON keys.

**Swagger UI:** Available at `/swagger-ui` (when running the service).
**OpenAPI Specification:** Available at `/api-docs/openapi.json`.

---

## 📋 Table of Contents

1. [Network Blocks](#network-blocks)
2. [Site Packs](#site-packs)
3. [User Rules](#user-rules)
4. [Schedule](#schedule)
5. [Locks](#locks)
6. [Scripts](#scripts)
7. [Connections (Network Block to Site Pack)](#connections)

---

## 🛠️ Network Blocks

Manage the primary containers for blocking rules.

### GET `/network-blocks`

Returns a list of all network blocks.

- **Response:** `GetAllNetworkBlocksResponse`
  - `networkBlocks`: Array of `NetworkBlock`

### POST `/network-blocks`

Creates a new network block.

- **Request Body:** `CreateNetworkBlockRequest`
  - `blockName`: string
- **Response:** `CreateNetworkBlockResponse`
  - `networkBlock`: `NetworkBlock`

### DELETE `/network-blocks/{id}`

Deletes a network block.

---

## 🌐 Site Packs

> **Product terminology:** the desktop UI presents Ace-provided curated site packs as **Categories** and user-imported site packs as **Custom Lists**. The engine/API intentionally keeps the single **site pack** model for both; endpoint paths, schema/property names, persistence, and synchronization semantics remain unchanged.

Manage large databases of sites (per-pack SQLite databases of XXH3-64 host
hashes; the RocksDB era ended 2026-08-22).

### GET `/site-lists`

Returns all registered site packs.

- **Response:** `GetAllSitePacksResponse`
  - `sitePacks`: Array of `SitePack`

### POST `/site-lists`

Imports/Creates a new site pack. Returns an SSE (Server-Sent Events) stream tracking the import progress.

- **Request Body:** `CreateSitePackRequest`
  - `name`: string
  - `description`: string (optional)
  - `customId`: string (optional)
  - `customFolder`: string (optional)
  - `sources`: Array of `BaseSitePackSource`
- **Response:** SSE Stream of `SitePackCreationStatus`

### DELETE `/site-lists/{site_pack_id}`

Deletes a site pack and its associated SQLite database.

### GET `/site-lists/{site_pack_id}/sources`

Returns the sources (URLs or Files) for a specific site pack.

- **Response:** `GetSitePackSourcesResponse`
  - `sources`: Array of `SitePackSource`

### POST `/site-lists/{site_pack_id}/sources`

Adds a new source to a site pack (Requires Pro).

- **Request Body:** `AddSitePackSourceRequest`
- **Response:** `AddSitePackSourceResponse`

### DELETE `/site-packs/sources/{source_id}`

Deletes a specific source from a site pack.

---

## ✍️ User Rules

Regex-based rules for fine-grained control within a network block.

### GET `/network-blocks/{network_block_id}/user-rules`

Returns all user-defined rules for a block.

- **Response:** `GetUserDefinedRulesResponse`
  - `userDefinedRules`: Array of `UserDefinedRule`

### POST `/network-blocks/{network_block_id}/user-rules`

Adds multiple user-defined rules to a block.

- **Request Body:** `AddUserDefinedRulesRequest`
  - `rules`: Array of strings (regex patterns)
- **Response:** `AddUserDefinedRulesResponse`

### DELETE `/user-rules/{user_rule_id}`

Deletes a user-defined rule.

---

## 📅 Schedule

Define when a network block is active.

### GET `/network-blocks/{network_block_id}/schedule`

Returns all schedule entries for a block.

- **Response:** `GetScheduleEntriesResponse`
  - `scheduleEntries`: Array of `ScheduleEntry`

### POST `/network-blocks/{network_block_id}/schedule`

Adds multiple schedule entries to a block.

- **Request Body:** `AddScheduleEntriesRequest`
  - `scheduleEntries`: Array of `BaseScheduleEntry`

### PUT `/schedule/{schedule_entry_id}`

Updates an existing schedule entry.

- **Request Body:** `BaseScheduleEntry`

### DELETE `/schedule/{schedule_entry_id}`

Deletes a schedule entry.

---

## 🔒 Locks

Control the strictness and modifiability of a network block.

### GET `/network-blocks/{network_block_id}/lock`

Returns the current lock status.

- **Response:** `GetLockResponse`
  - `lock`: `NetworkListLock` (optional)

### POST `/network-blocks/{network_block_id}/lock`

Sets or updates the lock of a network block. Note that locks can generally only be "strengthened".

- **Request Body:** `SetLockRequest`
  - `active`: boolean
  - `lockType`: `LockType` ("ACTIVE_DURING_SCHEDULE", "ACTIVE_UNTIL_LOCK_END")
  - `lockEnd`: string (ISO date/time, optional)
- **Response:** `SetLockResponse`

---

## 📜 Resistance Mode (v1 beta)

> **v1 beta scope:** static intensity only. Dynamic escalation is not a
> public feature (no data source); requests to enable it return
> `400 unsupported_mode`. Old persisted dynamic configurations normalize to
> static at startup. Resistance Mode supports only two lock states:
> `inactive` and `active_until_lock_end` (anything else is rejected with
> `400 unsupported_lock_state`). Every error response carries a stable
> machine-readable `code` (see `error-policies.md`).
>
> State-changing requests require the `X-Ace-Capability` header (see
> `local-api-protection-design.md`).

### GET `/resistance-mode/settings`

Returns settings plus the truthful enforcement status:
`enforcementStatus` (`operational | disabled | not_pro | scripts_missing |
scripts_invalid | no_scripts_enabled | intensity_none`),
`proEligible`, `scriptsLoaded`, `scriptsTotal`, `scriptsEnabled`,
`scriptsVersion`, lock state and exact `lockEnd`.

- **Response:** `ResistanceModeSettings`

### GET `/resistance-mode/scripts`

Returns all available resistance scripts.

- **Response:** `GetAllScriptsResponse`
  - `scripts`: Array of `ScriptInfo`

### GET `/resistance-mode/scripts/{id}`

Returns a specific script by its ID.

- **Response:** `ScriptInfo`

### POST `/resistance-mode/set-enabled`

Body: `{ "enabled": boolean }`. 403 `resistance_locked` while locked.

### POST `/resistance-mode/type`

Body: `{ "type": "static", "intensity": "low|medium|high|extreme" }`.
Dynamic requests are rejected with `400 unsupported_mode`.

### POST `/resistance-mode/set-lock`

Body: `{ "lockState": "active_until_lock_end", "lockEnd": "YYYY-MM-DDTHH:MM:SS" }`.
Only `inactive` and `active_until_lock_end` are accepted; unsupported states
are rejected with `400 unsupported_lock_state` before any state changes.

### POST `/resistance-mode/scripts/{id}/set-enabled`

Body: `{ "enabled": boolean }`. 403 `resistance_locked` while locked.

### POST `/resistance-mode/download-scripts`

Downloads, SHA-256-verifies, extracts and reloads the script catalog. The
installed version is committed to memory and SQLite only after all steps
succeed (no more re-download loops). Failures return
`502 script_download_failed` / `503 scripts_unavailable` / `403
pro_status_check_failed`.

> **Naming note:** the manual route docs below still use the older
> `site-packs` naming for some endpoints; the code registers `/site-lists`
> (singular). The committed `docs/openapi.json` is authoritative.

---

## 🔗 Connections (Network Block to Site Pack)

Manage which site packs are assigned to which network blocks.

### GET `/network-blocks/{network_block_id}/site-packs`

Returns connections for a block.

- **Response:** `GetNetListToSitePacksResponse`
  - `connections`: Array of `NetworkBlockToSitePack`

### POST `/network-blocks/{network_block_id}/site-packs`

Adds a site pack to a network block.

- **Request Body:** `AddNetListToSitePackRequest`
  - `sitePackId`: string
- **Response:** `AddNetListToSitePackResponse`

### DELETE `/network-blocks/{network_block_id}/site-packs/{site_pack_id}`

Removes a site pack from a network block.

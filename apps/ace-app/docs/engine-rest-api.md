# Ace Engine REST API Documentation

The Ace Engine exposes a REST API for managing network blocks, sites lists, schedules, and more. The API is Axum-based and uses camelCase for JSON keys.

**Swagger UI:** Available at `/swagger-ui` (when running the service).
**OpenAPI Specification:** Available at `/api-docs/openapi.json`.

---

## 📋 Table of Contents

1. [Network Blocks](#network-blocks)
2. [Sites Lists](#sites-lists)
3. [User Rules](#user-rules)
4. [Schedule](#schedule)
5. [Locks](#locks)
6. [Scripts](#scripts)
7. [Connections (Network Block to Sites List)](#connections)

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

## 🌐 Sites Lists

Manage large databases of sites (RocksDB backed).

### GET `/sites-lists`

Returns all registered sites lists.

- **Response:** `GetAllSitesListsResponse`
    - `sitesLists`: Array of `SitesList`

### POST `/sites-lists`

Imports/Creates a new sites list. Returns an SSE (Server-Sent Events) stream tracking the import progress.

- **Request Body:** `CreateSitesListRequest`
    - `name`: string
    - `description`: string (optional)
    - `customId`: string (optional)
    - `customFolder`: string (optional)
    - `sources`: Array of `BaseSitesListSource`
- **Response:** SSE Stream of `SitesListCreationStatus`

### DELETE `/sites-lists/{sites_list_id}`

Deletes a sites list and its associated RocksDB database.

### GET `/sites-lists/{sites_list_id}/sources`

Returns the sources (URLs or Files) for a specific sites list.

- **Response:** `GetSitesListSourcesResponse`
    - `sources`: Array of `SitesListSource`

### POST `/sites-lists/{sites_list_id}/sources`

Adds a new source to a sites list (Requires Pro).

- **Request Body:** `AddSitesListSourceRequest`
- **Response:** `AddSitesListSourceResponse`

### DELETE `/sites-lists/sources/{source_id}`

Deletes a specific source from a sites list.

---

## ✍️ User Rules

Domain and regex rules for fine-grained control within a network block.

### GET `/network-blocks/{network_block_id}/user-rules`

Returns all user-defined rules for a block.

- **Response:** `GetUserDefinedRulesResponse`
    - `userDefinedRules`: Array of `UserDefinedRule`

### POST `/network-blocks/{network_block_id}/user-rules`

Adds multiple user-defined rules to a block.

- **Request Body:** `AddUserDefinedRulesRequest`
    - `rules`: Array of `{ rule: string, ruleType: "domain" | "regex" }` objects
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

## Resistance Mode (v1 beta)

Manage the Resistance Mode feature (static friction scripts injected into
covered sites).

> **v1 beta:** static intensity only; dynamic escalation is not a public
> feature. Lock states are limited to `inactive` and `active_until_lock_end`.
> State-changing requests require the engine's `X-Ace-Capability` header
> (the app attaches it automatically; see
> `ace-engine/docs/local-api-protection-design.md`).

### GET `/resistance-mode/settings`

Returns settings plus the truthful enforcement status
(`enforcementStatus`, `proEligible`, `scriptsLoaded`, `scriptsTotal`,
`scriptsEnabled`, `scriptsVersion`, lock state and exact `lockEnd`).

- **Response:** `ResistanceModeSettings`

### GET `/resistance-mode/scripts`

Returns all available resistance scripts.

- **Response:** `GetAllScriptsResponse`
    - `scripts`: Array of `ScriptInfo`

### GET `/resistance-mode/scripts/{id}`

Returns a specific script by its ID.

- **Response:** `ScriptInfo`

### POST `/resistance-mode/set-enabled`

Body: `{ "enabled": boolean }`.

### POST `/resistance-mode/type`

Body: `{ "type": "static", "intensity": "low|medium|high|extreme" }`.

### POST `/resistance-mode/set-lock`

Body: `{ "lockState": "active_until_lock_end", "lockEnd": "YYYY-MM-DDTHH:MM:SS" }`.

### POST `/resistance-mode/scripts/{id}/set-enabled`

Body: `{ "enabled": boolean }`.

### POST `/resistance-mode/download-scripts`

Downloads and reloads the script catalog.

---

## 🔗 Connections (Network Block to Sites List)

Manage which sites lists are assigned to which network blocks.

### GET `/network-blocks/{network_block_id}/sites-lists`

Returns connections for a block.

- **Response:** `GetNetListToSitesListsResponse`
    - `connections`: Array of `NetworkBlockToSitesList`

### POST `/network-blocks/{network_block_id}/sites-lists`

Adds a sites list to a network block.

- **Request Body:** `AddNetListToSitesListRequest`
    - `sitesListId`: string
- **Response:** `AddNetListToSitesListResponse`

### DELETE `/network-blocks/{network_block_id}/sites-lists/{sites_list_id}`

Removes a sites list from a network block.

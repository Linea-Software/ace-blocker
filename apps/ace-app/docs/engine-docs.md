# Ace Engine Project Context

This file provides the necessary context for AI models to understand the project structure, core concepts, and development policies of the **ace-engine**.

## 🚀 Project Overview

Ace Engine is a high-performance network and program blocker. It is built as a **service engine** that does not manage a UI directly but exposes a robust **REST API** (Axum-based, located in `src/service/api/`) for client applications.

The core logic resides in the `engine-core` crate, while the `src/` directory contains the service layer, database management, and the **system-wide MITM proxy** (`src/service/proxy.rs`) which coordinates URL evaluation and script injection.

---

## 🏗️ Core Concepts

### 1. Network Blocks

A **Network Block** is a bundle of rules used to decide what to block and when. It consists of:

- **User Defined Rules**: Domain matches and regex rules (`src/core/network_block/user_defined_rules.rs`).
- **Exceptions**: Rules that override blocks (`engine-core/src/network_block/mod.rs`).
- **Sites Lists**: Collections of millions of sites (see below).
- **Schedule**: Weekday/time-based activation rules.
- **Lock**: Logic that prevents weakening the blocking rules.

- **Location:** `engine-core/src/network_block/mod.rs`
- **Storage:** Metadata stored in **SQLite** via **Diesel**, entries in memory/RocksDB.

### 2. Sites Lists

Static or dynamic databases of sites/hosts to be blocked.

- **Implementation:** Backed by **RocksDB** for high-performance lookups of millions of entries.
- **Location:** `engine-core/src/network_block/sites_list.rs`
- **Syncing:** Can be imported from URLs or local files using the **Importer** (`engine-core/src/network_block/importer/`).

### 3. Schedule & Locks

- **Schedule**: Defines active time windows. Uses bitmasks for weekdays.
- **Lock**: Implements a "strictness" policy. A lock can only be changed to a stricter state (e.g., from Unlocked to LockedUntilEnd).
- **Active Lock Constraints**: When a lock is active, it prevents weakening the blocking rules. Specifically:
    - **No Deletion**: The Network Block cannot be deleted.
    - **No Exceptions**: Adding new exceptions to the block is prohibited.
    - **No Disconnecting Site Lists**: Removing connections to Sites Lists is prohibited.
    - **No Removing User Rules**: Deleting existing user-defined rules is prohibited.
    - **No Reducing Schedule**: The schedule cannot be reduced (only expanded/made stricter).
    - **No Removing Lock**: The lock itself cannot be removed or downgraded.
- **Location:** `engine-core/src/common/schedule.rs` and `engine-core/src/common/lock.rs`.

### 4. Resistance Scripts

Custom JavaScript injected into specific websites to reduce their addictive qualities (e.g., removing infinite scroll).

- **Location:** `engine-core/src/common/resistance_scripts.rs`
- **Management:** Managed via a `DomainScriptTree` for fast domain-based script lookup.

---

## 📂 Architecture & Data Management

### 🗄️ Database (SQLite + Diesel)

- **ORM:** Diesel RS.
- **Migrations:** Located in `/migrations`.
- **Schema:** Defined in `src/schema.rs`.
- **Repositories:** Modularized in `src/db/` (e.g., `sites_list_repo.rs`, `network_block_repo.rs`).
- **Policy:** Repository functions should return Diesel models or specific requested values. Use basic models when appropriate.

### 🧠 Network Manager (`src/manager/network/`)

The main orchestrator of the system.

- **Policy:** **ALWAYS use in-memory data.** The manager loads all rules into memory on startup. Do not query the database during URI evaluation.
- **Thread Safety:** Wrapped in `Arc<RwLock<NetworkManager>>` within the AppState.
- **Refactoring Note:** Terminology has been updated from "Hosts List" to "Sites List" and "Net List" to "Network Block".

### 🛡️ Proxy Service (`src/service/proxy.rs`)

The MITM proxy that intercepts traffic. It consults the `NetworkManager` for every request to determine if it should be blocked, allowed, or modified (script injection).

---

## 🧪 Testing & Documentation

- **REST API Docs:** Available via Swagger UI at `/swagger-ui` (OpenAPI generated).
- **Integration Tests:** Located in `/tests`, focusing on API and proxy correctness.
- **Unit Tests:** Found within the respective source files.

---

## 📖 Further Documentation

- `docs/rest-api.md`: Detailed REST API endpoint documentation.
- `docs/error-policies.md`: Error handling conventions.
- `docs/ROADMAP.md`: Project future goals.

# Ace Engine Project Context

This file provides the necessary context for AI models to understand the project structure, core concepts, and development policies of the **ace-engine**.

## 🚀 Project Overview

Ace Engine is a high-performance network and program blocker. It is built as a **service engine** that does not manage a UI directly but exposes **typed remoc RPC** via Named Pipes / Unix sockets to local clients.

The core logic resides in the `src/core/` module, while the `src/` directory contains the service layer, database management, and the **system-wide MITM proxy** (`src/service/proxy/mod.rs`) which coordinates URL evaluation and script injection.

Traffic redirection is handled by the pure-Rust **[`ace-tun`](../ace-tun/)**
crate — a cross-platform transparent redirect (WinTun on Windows, TUN on
Linux, utun on macOS) that replaces the earlier WinDivert-based
`proxy-redirect`.
See [`ace-tun/README.md`](../ace-tun/README.md) for architecture. The `ace-tun` and `mitm-proxy` crates are workspace members under `crates/`, so engine builds always use the in-repo sources rather than remote Git revisions.
Only browser processes are routed through the MITM proxy; other applications'
connections are passed through or blocked at the kernel level without decryption.
For HTTP/3, the TUN recognizes browser QUIC Initials even on alternative UDP
ports, and forwards selected flows only to the local H3 endpoint. Unattributed
or migrated QUIC is rejected. The engine's H3 SNI policy permits ONLY hosts
currently blocked at hostname level; hosts needing URL/path/Resistance inspection
must use the TCP/CONNECT fallback, which retains the TLS client-certificate
preflight. Do not relax this without real native/browser/Windows tests.

---

## Core Concepts

### 1. Network Blocks

A **Network Block** is a bundle of rules used to decide what to block and when. It consists of:

- **User Defined Rules**: Regex-based rules (`src/core/network_block/regex_rules.rs`).
- **Exceptions**: Rules that override blocks (`src/core/network_block/mod.rs`).
- **Site Packs**: Collections of millions of sites (see below).
- **Schedule**: Weekday/time-based activation rules.
- **Lock**: Logic that prevents weakening the blocking rules.

- **Location:** `src/core/network_block/mod.rs`
- **Storage:** Metadata stored in **SQLite** via **Diesel**; site entries live in a
  per-pack **SQLite** database (XXH3-64 host hashes), not RocksDB.

### 2. Domain identifiers

- `NetworkBlockId` and other internally generated UUID identities use UUID v7
  where their actual contract requires it.
- `SitePackId` is an opaque, validated, non-empty **single filesystem
  component**, not necessarily a UUID. Canonical catalog IDs such as
  `pack-adult-free` are legitimate. Incoming/persisted IDs must be validated.
- `RemoteSitePackId` obeys the same safe-component constraints; `ScriptId`
  is a non-empty string identity validated at the catalog, DB and RPC boundaries.
- Do not assume every domain identity is a UUID or admit empty IDs through
  `Default`/`From` constructors. Keep typed, fallible parsing at trust boundaries.

**Pre-release policy:** Ace Blocker has not yet been publicly released. No
backward compatibility for legacy program data, old databases, historical
backups or old local protocols is required. Remove rather than perpetuate
compatibility-only code; preserve security, locks, service isolation and current
typed IPC. The binding decisions are in
`../../docs/ai/ACE_ENGINE_REFACTOR_DECISIONS.md`.

### 3. Site Packs

Static or dynamic databases of sites/hosts to be blocked.

- **Implementation:** One **SQLite** database per pack (`pack.db`, XXH3-64 host
  hashes). Membership is queried lazily with a bounded per-pack cache of
  positive/negative results (500 entries), invalidated on close/reload.
  Never load the full pack into RAM at startup.
- **Location:** `src/core/network_block/site_pack/`
- **Syncing:** Can be imported from URLs or local files using the **Importer**
  (`src/core/network_block/site_pack/importer/`) when building packs. Runtime
  remote updates consume validated SQLite backup ZIPs, stage/extract them outside
  the manager lock, and commit with a temp-dir swap (see
  `docs/review/architecture-implementation-2026-09.md`).

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
- **Location:** `src/core/common/schedule.rs` and `src/core/common/lock.rs`.
- **Time Format:** Schedule entries use `chrono::NaiveTime` for high-precision time window matching.
- **`ActiveDuringSchedule` locks** are enforced only while the block's schedule
  matches (window-bound; see the doc comment in `src/core/common/lock.rs`). The
  installer's uninstall gate treats them as always active on purpose.

### 4. Resistance Scripts

Custom JavaScript injected into specific websites to reduce their addictive qualities (e.g., removing infinite scroll).

- **Location:** `src/core/common/injectable_scripts.rs`
- **Management:** Managed via an `InjectableScriptTree` for fast domain-based script lookup. Script entries are identified by a `ScriptId` (UUID).

---

## 💻 Coding Standards

### Descriptive Variable Naming

- **No Single-Letter Variables**: Avoid using single letters for variables (e.g., `i`, `j`, `k`, `x`, `y`). They make the code unreadable and harder to maintain.
- **Be Descriptive**: Always use descriptive names that convey the purpose of the variable. The compiler doesn't care about naming, but humans (and AI) do. We aim for stable, maintainable software, not competitive programming hacks.

### Model-Based Design & Modularization

- **Make Invalid States Unrepresentable**: When facing complex logic, avoid huge functions. Instead, design custom data structures (like enums with data) that only allow valid states. This eliminates entire classes of bugs and makes the code self-documenting (e.g., `LockState` in `lock.rs`).
- **Function Splitting**: If a task is big, break it down into smaller, focused functions.
- **Meaningful type boundaries, not type proliferation**: Reuse the existing validated `nutype` domain types in `src/core/common/domain_types.rs`; introduce new ones only when they protect a demonstrated identity, secret or invariant. Prefer existing standard types where sufficient. Validate untrusted data at the correct boundary without changing remoc/DB formats in an incidental refactor.
- **Feature ownership before sharing**: Similar Rust code in unrelated subsystems is not automatically one abstraction. Keep domain semantics with their owner; avoid broad `pub` exposure, global utilities, trait layers or crate splits without a concrete boundary.
- **Refactoring workflow**: Read root `../../AGENTS.md`, `../../docs/ai/REFACTORING_PLAYBOOK.md` and the relevant sections of `../../docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`, `../../docs/ai/DOMAIN_TYPES.md` and `../../docs/ai/REVIEW_GATES.md`. Check call sites, error/persistence order and tests before editing. Local API behavior, lock checks, process isolation and proxy interception are never negotiable.

---

## 🛠️ Agent Workflow

### Git Commit Policy

- **Commit After Tasks**: ALWAYS commit your changes after successfully finishing a task.
- **Message Format**: Use conventional commit prefixes:
  - `feat:` for new features
  - `fix:` for bug fixes
  - `tests:` for adding or updating tests
  - `refactor:` for code changes that neither fix a bug nor add a feature
  - `docs:` for documentation changes

### No Random Assumptions

- **Ask if Unsure**: If a requirement is ambiguous or you're unsure about the intended behavior (e.g., locking constraints), ALWAYS ask the user for clarification instead of assuming a "safe" or "correct" path. We prefer stability and correctness over fast guesses.

### Generated Ace Server API Client

- **Never hand-edit `src/ace_server_api/`**: it is generated code. Manual edits there are useless because the next client regeneration overwrites them.
- The OpenAPI generator only concerns the external Ace Server client. Local control methods must be added to the remoc trait and Tauri facade directly.
- Do **not** resolve Clippy or formatting findings by patching files under `src/ace_server_api/`; address the generation source/tooling instead.

---

## 📂 Architecture & Data Management

### 🗄️ Database (SQLite + Diesel)

- **ORM:** Diesel RS.
- **Migrations:** Located in `/migrations`.
- **Schema:** Defined in `src/schema.rs`.
- **Repositories:** Modularized in `src/db/` (e.g., `sites_list_repo.rs`, `network_block_repo.rs`).
- **Policy:** Repository functions should return Diesel models or specific requested values. UUID strings in the database are converted to `uuid::Uuid` during retrieval. Use basic models when appropriate.

### 🧠 Network Manager (`src/manager/network/`)

The main orchestrator of the system.

- **Policy:** Keep rule metadata in memory. Site-pack membership uses lazy,
  indexed SQLite lookups behind a bounded per-pack cache; never preload all
  pack entries into RAM. Do not query the main database during URI evaluation.
- **Thread Safety:** Wrapped in `Arc<RwLock<NetworkManager>>` within the AppState.

### 🛡️ Proxy Service (`src/service/proxy/mod.rs`)

The MITM proxy that intercepts traffic. It consults the `NetworkManager` for every request to determine if it should be blocked, allowed, or modified (script injection).

---

## 🛡 Process Hardening (user-mode)

Tamper protection is user-mode only (no kernel driver):

- **File hiding**: app-data and installation directories are marked
  HIDDEN+SYSTEM via `SetFileAttributesW` (`hide_important_files`,
  `src/platform.rs`).
- **Production watchdog (Windows)**: `ace-watchdog.exe` is a separate
  LocalSystem Windows service. The MSI owns `AceEngineWatchdog` registration,
  restart-on-failure policy, service SID, and ACLs. It queries the SCM-owned
  `AceEngine` state/PID and combines that with the restricted
  `\\.\pipe\AceBlocker\WatchdogV1` challenge/response heartbeat. Recovery
  is thresholded and uses bounded backoff; the watchdog never creates an orphan
  SYSTEM process.
- **Production watchdog (Linux)**: `ace-watchdog` runs as
  `ace-watchdog.service` and monitors `ace-engine.service`. The two services
  exchange the same challenge/response protocol over the root-only
  `/run/ace-blocker/watchdog.sock` Unix socket. The watchdog verifies the
  socket peer PID against systemd `MainPID`, uses systemd for service recovery,
  and only sends SIGKILL to a PID after re-verifying that exact `MainPID`.
  Release `ace-engine --install` writes/enables both systemd units; uninstall
  requires the engine to have been shut down through its authenticated API first.
- **Engine-to-watchdog recovery**: service-mode engine processes maintain the
  heartbeat link and ask their platform service manager to start the watchdog if
  it disappears. Each engine start uses a fresh instance ID.
- **Planned maintenance**: only after `/system/shutdown` passes the existing
  network-block and Resistance Mode lock checks does the engine request a
  bounded in-memory watchdog lease (normally three minutes). The watchdog does
  not duplicate lock semantics.
- **Legacy console guardian**: `ace-engine.exe --guardian` remains only for
  release console runs; it is not the installed production watchdog.

The engine service still **does not accept ordinary SCM STOP** (`sc stop` and
Services UI stop are rejected). Authorized shutdown remains API-driven and
reports `SERVICE_STOPPED` before a clean service exit, so SCM recovery remains
reserved for unexpected failures. The debug-only `watchdog-heartbeat.pause`
file under ProgramData (or `ACE_WATCHDOG_DEV_DROP_HEARTBEAT`) can suppress
heartbeat replies while the process stays alive for hang-recovery testing;
release builds ignore both.

---

## 🧪 Testing & Documentation

- **Local control RPC:** typed remoc methods, Windows Named Pipes, and
  Linux Unix sockets. Refer to the unified root docs/IPC_ARCHITECTURE.md.
- **External Ace Server API:** generated via openapi-to-rust under
  src/ace_server_api. This is not the local control API.
- **Tests:** cargo test -p ace-engine --lib (including remoc_tests) and
  cargo test -p ace-watchdog-protocol. Browser-mocked HTTP tests are obsolete
  as production IPC coverage.
- **Developer RPC methods:** remain denied in release builds.
- **Adding operations:** extend the EngineControl remoc trait and Tauri facade;
  do not add local HTTP listeners, OpenAPI or port discovery.

---

### API key delivery (`aceblocker://` protocol)

The Ace Blocker website hands the desktop-app API key to the engine without
forcing re-login. The key is delivered via:

- `typed set_api_key RPC` with a **JSON body** `{ "apiKey": "..." }` (preferred),
  or the legacy `?apiKey=` query parameter (still accepted for older clients).
- The `aceblocker://account/api-key#apiKey=...` deep link: the OS invokes the
  protocol handler (`ace-engine.exe` in debug, `ace-redirect.exe` in release),
  which parses the URL **fragment** (`src/protocol.rs` →
  `api_key_from_fragment`) and forwards the key as a JSON body to the running
  engine service.

Never log the raw API key: `handle_uri_scheme` / `ace-redirect` print only the
path (not the full URI), and `AccountManager::verify_pro_status_remote` logs
only the key length.

---


## 📖 Further Documentation

- Root docs/IPC_ARCHITECTURE.md: Typed remoc control service.
- `docs/error-policies.md`: Error handling conventions.
- `docs/update-policy.md`: Comprehensive update strategy for site lists, scripts, engine binary, and ace-app.
- `docs/ROADMAP.md`: Project future goals.

---

## 🦀 Rust skill catalog (conditional)

These names document earlier Rust skill references. Consult them only if their
actual `SKILL.md` exists and can be inspected in the active environment.
A list is not proof of availability; do not auto-install external code. Root
security boundaries and Ace-specific architecture/type guides take priority.

The following Rust skill references may help with idiomatic patterns, ownership, error handling, and testing **when their actual instructions are accessible**.

| Skill | Description | When to Use |
| :------------ | :----------------------------- | :---------- |
| **rust-best-practices** | Idiomatic Rust conventions, ownership patterns, and error handling | General Rust development, code reviews |
| **rust-async-patterns** | Tokio async patterns, futures, and concurrent programming | Async code, network operations, concurrent tasks |
| **rust-testing** | Unit tests, integration tests, property-based testing, and mocking | Writing or updating tests |
| **rust-patterns** | Common Rust design patterns and idiomatic implementations | Structuring new features, refactoring |

> [!IMPORTANT]
> If a referenced Rust skill is actually present, consult it as optional specialist guidance. If absent, use the repository's tests, code and docs; do not claim it was consulted.

---

## 📚 External Library Documentation

To ensure accuracy and avoid using deprecated or non-existent APIs, **ALWAYS refer to the local documentation** for external libraries before implementation or refactoring.

| Library       | Summary (`.llms.txt`)          | Full API Docs (`.llms-full.txt`)    |
| :------------ | :----------------------------- | :---------------------------------- |
| **Axum**      | `docs/libs/axum.llms.txt`      | `docs/libs/axum.llms-full.txt`      |
| **Tokio**     | `docs/libs/tokio.llms.txt`     | `docs/libs/tokio.llms-full.txt`     |
| **Diesel**    | `docs/libs/diesel.llms.txt`    | `docs/libs/diesel.llms-full.txt`    |
| **Serde**     | `docs/libs/serde.llms.txt`     | `docs/libs/serde.llms-full.txt`     |
| **Reqwest**   | `docs/libs/reqwest.llms.txt`   | `docs/libs/reqwest.llms-full.txt`   |
| **mitm-proxy** | `docs/libs/mitm-proxy.llms.txt` | `docs/libs/mitm-proxy.llms-full.txt` |
| **Utoipa**    | `docs/libs/utoipa.llms.txt`    | `docs/libs/utoipa.llms-full.txt`    |
| **Chrono**    | `docs/libs/chrono.llms.txt`    | `docs/libs/chrono.llms-full.txt`    |
| **Tracing**   | `docs/libs/tracing.llms.txt`   | `docs/libs/tracing.llms-full.txt`   |
| **Axum Test** | `docs/libs/axum-test.llms.txt` | `docs/libs/axum-test.llms-full.txt` |

> [!IMPORTANT]
> If a library is not listed here, or the documentation seems outdated, use search tools to find the latest documentation and update these files.

## Remoc local IPC
Read the root docs/IPC_ARCHITECTURE.md. Local REST control and port discovery have been removed. Extend the existing typed remoc RPC trait instead of adding HTTP endpoints.

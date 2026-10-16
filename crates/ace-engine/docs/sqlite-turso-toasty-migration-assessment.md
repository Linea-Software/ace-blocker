# SQLite → Turso + Toasty migration assessment

**Date:** 2026-09-05  
**Scope:** `ace-engine` database usage, including the main engine database, per-site-pack databases, installer lock checking, migrations, tests, and bundle-generation tooling.

## Executive summary

Yes: `ace-engine` can realistically move away from Diesel toward Toasty, and Toasty's Turso driver is now capable enough that Turso can be evaluated as the underlying local database engine.

However, there are two distinct migrations hiding behind that sentence:

1. **Diesel → Toasty** is mostly an ORM/API and sync→async refactor.
2. **SQLite/libsqlite3 → Turso** changes the database runtime and concurrency/multi-process assumptions.

They should not be treated as the same step.

My recommendation is:

- **Proceed with a Toasty migration spike.** The current data model is a good fit for Toasty's typed models, relations, transactions, targeted upserts, raw SQL escape hatch, and newly added migration system.
- **Do not make Turso Cloud the primary storage model for the engine.** The engine currently depends on authoritative local state, direct read-only inspection by the uninstall gate, filesystem-level recovery, and atomic replacement of site-pack databases. A local database should remain authoritative.
- **Keep the main DB on the ordinary Toasty SQLite driver for the first ORM migration, or prove Turso's multi-process behavior before switching it.** `--check-locks` is a separate process that opens the live DB file while the service may be writing. Turso exposes multi-process WAL as an experimental option, so this is a release gate for the main DB.
- **Evaluate local Turso independently for site-pack DBs.** They are single-purpose databases and do not need cross-process lock inspection, but they sit on a request-time hot path and bulk-import 50,000 hashes per batch. The Toasty high-level upsert API is single-row, so the existing bulk path should probably stay lower-level/raw SQL even if the connection is managed by Toasty/Turso.
- **Treat Toasty 0.10 migrations as usable but young.** Toasty 0.10.0 (2026-08-11) added embedded migrations, and the current CLI supports migration generation/application. This removes what would previously have been a hard blocker. `ace-engine` still has custom SQLite `CHECK` constraints, triggers, seeded SQL, and an existing Diesel migration history, so migration adoption needs an explicit baseline strategy and hand-reviewed SQL.

A likely long-term architecture is:

```text
Engine state
  EngineDB
    Toasty models/query API
      local SQLite-compatible DB
        ├─ Toasty SQLite initially
        └─ Toasty Turso after multi-process parity is proven

Site-pack content
  SitePackDB
    Turso/Toasty connection + raw/low-level SQL for hot operations
      └─ one local DB file per pack

Installer/uninstall gate
  direct read-only local DB access
    └─ deliberately independent from service/network availability
```

The main reason to do this migration should therefore be **ergonomics, async integration, and future storage flexibility**, not an assumption that moving the current local state to Turso Cloud is automatically better.

---

## 1. Current SQLite topology

`ace-engine` does not have one SQLite use case. It has four.

### 1.1 Main engine database

Implementation: `src/db.rs` and `src/db/*_repo.rs`.

`EngineDB` owns a Diesel `r2d2::Pool<ConnectionManager<SqliteConnection>>`. `EngineDB::from()`:

1. creates an r2d2 pool for a local file,
2. checks out a connection,
3. runs embedded Diesel migrations from `migrations/`, and
4. exposes synchronous repository methods that obtain a pooled connection per operation.

Relevant code: `src/db.rs:33-87` (pool + embedded migrations + constructor) and the repository modules under `src/db/`.

The main DB is passed around as `Arc<EngineDB>` and shared by account, network, content-update, API, and service code. Most higher-level code does not import Diesel directly, which is a useful migration boundary.

### 1.2 Per-site-pack content databases

Implementation: `src/core/network_block/site_pack/site_pack_db.rs`.

Every site pack has its own SQLite file. Each database contains only:

- `entries(host_hash INTEGER PRIMARY KEY)`; and
- one `pack_meta` row.

`SitePackDB` also owns an r2d2/Diesel pool and runs a separate embedded migration set from `migrations_site_packs/`.

This database is not merely import/storage infrastructure. On a block-check cache miss:

`NetworkBlock::is_content_blocked()` → `get_internal_content_blocking_result()` → each attached `SitePack::contains(host)` → `SitePackDB::contains(host)` → indexed DB lookup.

Relevant code:

- `src/core/network_block/mod.rs:233-277`
- `src/core/network_block/site_pack/mod.rs:142-150`
- `src/core/network_block/site_pack/site_pack_db.rs:164+`

So site-pack DB lookup latency can affect proxy/request latency.

Imports intentionally batch host hashes in groups of **50,000** (`INSERT_BATCH_SIZE`) and call `SitePackDB::insert_entries()` for each batch.

Relevant code: `src/core/network_block/site_pack/importer/importer.rs:25`, `:257-297`.

### 1.3 Standalone read-only lock checker

Implementation: `src/lock_check.rs`.

`ace-engine --check-locks` is intentionally independent of the running service. It:

- opens the main DB file directly with `mode=ro`;
- runs no migrations and performs no writes;
- sets `PRAGMA busy_timeout = 2000`;
- reads network-block locks and Resistance Mode lock state;
- fails safe if the DB is missing/unreadable; and
- is tested while another connection holds `BEGIN EXCLUSIVE` for ~400 ms.

Relevant code: `src/lock_check.rs:83-198`, contention test at `:392-428`.

This is a hard architectural constraint. The uninstall decision must not depend on:

- engine service availability;
- HTTP availability;
- cloud connectivity; or
- a remote database being reachable.

### 1.4 Site-pack bundle generation tooling

`src/bin/export_site_pack_bundle.rs` also reaches through `SitePackDB` to Diesel directly. It updates `pack_meta` and rewrites the timestamp in `__diesel_schema_migrations` so generated bundle bytes are deterministic.

Relevant code: `src/bin/export_site_pack_bundle.rs:409-433`.

If Diesel migration tracking is replaced with Toasty's `__toasty_migrations`, this deterministic-build behavior must be ported as well. It is easy to miss because it lives outside `src/db/`.

---

## 2. Current dependencies and tooling

`Cargo.toml` currently uses:

- `diesel ~2.3` with `sqlite`, `returning_clauses_for_sqlite_3_35`, `r2d2`, `numeric`;
- `diesel_migrations ~2.3`;
- `libsqlite3-sys ~0.37` with `bundled`.

There are two Diesel CLI configs:

- `diesel_main_db.toml` → `migrations/` → `src/schema_main_db.rs`
- `diesel_site_packs.toml` → `migrations_site_packs/` → `src/schema_site_packs.rs`

Both enable `sqlite_integer_primary_key_is_bigint = true` for schema generation.

The project uses Rust `stable` (`rust-toolchain.toml`) and edition 2024. Toasty's repository currently declares edition 2024 and `rust-version = "1.95"`; CI/release builders therefore need to resolve to Rust >= 1.95 before adopting it.

---

## 3. Main database schema

The generated Diesel schema currently contains 15 tables.

| Table | Primary key | Purpose / notable shape |
|---|---|---|
| `accounts` | `table_lock` | Singleton account row; API key, Pro-check timestamp, ToS acceptance timestamp |
| `blocked_attempts` | `date` | Daily aggregate counter |
| `network_block_exceptions` | `id` | Per-network-block exception rules |
| `network_block_locks` | `network_block_id` | One lock row per network block |
| `network_block_schedule_entries` | `id` | Schedule windows and weekday bitmask |
| `network_block_to_site_pack` | (`network_block_id`, `site_pack_id`) | Many-to-many join table |
| `network_blocks` | `id` | Network block metadata, safe-search flag, icon |
| `proxy_exclusions` | `host` | Hosts bypassed by proxy handling |
| `resistance_mode_settings` | `table_lock` | Singleton Resistance Mode settings + lock state |
| `safe_search_enforcements` | `domain` | Domain redirect/query-parameter enforcement records |
| `script_states` | `script_id` | Per-script enabled flag |
| `settings` | `table_lock` | Singleton engine state (`is_pro`, seeded state, update timestamp) |
| `site_pack_sources` | `id` | Local/URL source metadata for a site pack |
| `site_packs` | `id` | Pack metadata; optional remote ID/version/update time |
| `user_defined_rules` | `id` | Per-network-block blocking rules |

See `src/schema_main_db.rs`.

### 3.1 Persisted type conventions

The existing model layer (`src/db/models.rs`) deliberately keeps persistence simple:

- IDs are mostly UUID v7 values serialized to **TEXT**.
- Dates/timestamps/times are mostly **TEXT**.
- booleans are SQLite booleans/integer affinity.
- counters/bitmasks are integers.
- optional fields are ordinary nullable columns.
- there is no application JSON/document-column dependency in the main schema.

That is a very compatible shape for Toasty/Turso.

One migration detail matters: Toasty maps `uuid::Uuid` to SQLite/Turso `BLOB` by default. To preserve the current database byte-for-byte, either:

- initially keep model IDs as `String`; or
- use Toasty's explicit text-column mapping for `Uuid` fields.

Do **not** silently change existing UUID columns from TEXT to BLOB as part of the ORM swap.

The same principle applies to the current chrono-derived date/time strings: keep the storage representation unchanged during the first migration. A later cleanup can consider Toasty's `jiff` types after compatibility is established.

---

## 4. SQLite-specific schema behavior that must survive

The database is more than the generated Diesel table list. The migration SQL contains behavior that an ORM model definition alone does not capture.

### 4.1 Foreign keys and cascades

Relationships include foreign keys such as:

- rules/exceptions/schedules/locks → `network_blocks`;
- join table → `network_blocks` and `site_packs`;
- `site_pack_sources` → `site_packs`.

Several use `ON DELETE CASCADE`.

One migration explicitly warns that SQLite only honors cascade deletion when `PRAGMA foreign_keys` is enabled (`migrations/0021_seed_default_network_blocks/down.sql`). I found no global `PRAGMA foreign_keys = ON` initialization in the normal `EngineDB` pool; the explicit PRAGMA usage in Rust is currently only the lock checker's `busy_timeout`.

Regardless of ORM choice, connection setup should make the intended FK policy explicit and test it.

### 4.2 Custom CHECK constraints

Examples include:

- schedule times validated via `strftime('%H:%M:%S', ...)`;
- schedule day bitmask constrained to `0..=127`;
- lock states constrained to a fixed vocabulary;
- lock-end presence/format tied to lock state;
- Resistance Mode static intensity/state constraints;
- `site_pack_sources.source_type` constrained to `url|file`;
- `site_pack_sources.format` constrained to `standard|wildcard`;
- safe-search enforcement type and field-shape checks;
- singleton tables constrained to `table_lock = '0'`.

These constraints are part of data integrity and should not be dropped merely because Toasty can express the application types more ergonomically.

### 4.3 Main-DB trigger

`migrations/0005_create_list_locks/up.sql` defines `insert_network_block_lock`:

- after inserting a `network_blocks` row,
- automatically insert the corresponding inactive `network_block_locks` row.

Several higher-level paths implicitly rely on a lock row existing for every block.

### 4.4 Site-pack metadata triggers

`migrations_site_packs/0001_create_site_pack/up.sql` defines two triggers that reject `pack_meta.entry_count` values that do not equal `COUNT(entries)`.

They use `SELECT RAISE(ABORT, ...)` and a subquery in the trigger condition because SQLite cannot express that invariant in a normal `CHECK` constraint.

This custom SQL must remain explicitly tested after migration.

---

## 5. Current repository/query surface

The code is already reasonably well isolated behind `EngineDB`, which is the strongest argument that an ORM migration is tractable.

### `account_repo.rs`

- get singleton account row;
- update API key and Pro-check timestamp;
- update API key;
- update Pro-check timestamp;
- accept/reset ToS;
- several updates currently use SQLite `UPDATE ... RETURNING` through Diesel's `.get_result()`.

### `blocked_attempts_repo.rs`

- atomic daily upsert;
- on conflict, `attempts_count = attempts_count + 1`;
- optional read returning zero when the date is absent.

### `exceptions_repo.rs`

- list exceptions for a block;
- inner join to fetch the owning block;
- insert a list of entries inside a transaction, with generated UUID v7 IDs;
- delete and return the deleted rule.

### `lock_repo.rs`

- get lock for a block;
- update lock state/end and return the updated row.

### `network_block_repo.rs`

- load all blocks;
- insert block with generated or supplied ID and return it;
- delete block;
- update safe-search flag/name/icon.

### `network_block_to_site_pack_repo.rs`

- load all pack links for a block;
- insert link and return it;
- delete by composite predicate.

The main `EngineDB` also loads the full join table and groups it in Rust for startup materialization.

### `proxy_exclusion_repo.rs`

- load the entire host table.

### `resistance_mode_repo.rs`

- load singleton settings;
- upsert singleton row by `table_lock`, returning it;
- update scripts version.

### `safe_search_repo.rs`

- load all enforcements;
- insert a domain-redirection row;
- insert a query-parameter row.

### `schedule_repo.rs`

- list entries for a block;
- inner join to load owning block;
- **transactional replace:** delete all entries for the block and bulk insert the replacement list.

The atomic replace behavior must be preserved.

### `script_state_repo.rs`

- optional lookup by ID;
- upsert enabled state by primary key, returning the stored row;
- load all states.

### `settings_repo.rs`

- load singleton settings;
- update Pro status;
- update seeded status;
- update latest content-check timestamp.

### `site_pack_repo.rs`

- list non-ignored packs;
- lookup by local ID;
- several insert variants returning the inserted model;
- update last-updated/version/ignored state;
- delete;
- lookup by `remote_id`;
- list ignored remote packs.

`remote_id` is filtered as though it identifies one row but is not represented as a unique key in the current generated schema. If Toasty convenience `get_by_*` methods are desired here, decide explicitly whether `remote_id` should become a unique/indexed database constraint or keep using a general filter.

### `site_pack_source_repo.rs`

- list sources for pack;
- get source by ID;
- insert and return source;
- delete source.

### `user_defined_rule_repo.rs`

- list rules for a block;
- inner join to fetch owning block;
- insert a vector inside one transaction, returning each inserted model;
- delete by ID and return deleted rule;
- delete by `(network_block_id, rule)` and return deleted rule.

---

## 6. Per-site-pack DB operations

`SitePackDB` has a smaller but more performance-sensitive API:

- `get_entry_count()` — read `pack_meta.entry_count`;
- `clear()` — delete `entries`, then `pack_meta` (currently not one explicit transaction);
- `insert_entries(&[i64])` — one bulk `INSERT OR IGNORE` for a vector of hash rows;
- `count_entries()` — `COUNT(*)`;
- `contains(host)` — XXH3-64 → signed `i64`, primary-key existence/count lookup;
- `insert_pack_meta()` — insert metadata.

The content schema is intentionally minimal:

```sql
CREATE TABLE entries (
    host_hash INTEGER PRIMARY KEY NOT NULL
);

CREATE TABLE pack_meta (
    id TEXT PRIMARY KEY NOT NULL,
    format_version TEXT NOT NULL,
    name TEXT,
    entry_count INTEGER NOT NULL DEFAULT 0,
    built_at TEXT NOT NULL
);
```

The current import pipeline can feed extremely large lists without holding the full list in memory because it parses streams and flushes 50,000 hashes at a time.

The current restore/update design also depends on file semantics: a new DB is staged and validated, the live DB handle is closed, directories are atomically renamed, then the DB is reopened (`src/core/network_block/site_pack/mod.rs:183-255`). A Turso-local design should keep this local-file replacement property unless there is a compelling reason to redesign the whole updater.

---

## 7. Startup, recovery, and concurrency assumptions

### 7.1 Startup is async at the application level but synchronous at the DB layer

`initialize_db()` is async, but `EngineDB::from()` and all repository calls are synchronous Diesel calls.

On startup the engine:

1. tries to open/migrate the DB up to three times;
2. waits two seconds between attempts;
3. on connection/migration failure, treats the DB as corrupted/inaccessible;
4. renames it to a `.bak` path (or deletes it if rename fails);
5. creates a fresh DB.

Relevant code: `src/lib.rs:192-267`.

A Toasty/Turso migration should preserve these recovery semantics deliberately. It should not accidentally treat a transient Turso serialization/IO condition as permanent corruption and rename a healthy DB.

### 7.2 Higher-level constructors are synchronous

`NetworkManager::from`, `AccountManager::from`, and many manager methods synchronously call `EngineDB` methods. Toasty operations are async, so changing the DB facade to Toasty will propagate `.await` into manager/service code.

This is a larger source change than the SQL/query rewrite itself.

The upside is that the engine already runs Tokio, and several DB-heavy workflows (content updates and pack imports) are already async at the surrounding layer.

### 7.3 Site-pack membership is hot-path but cached above the DB

Network blocks have a result cache. A cache hit avoids the site-pack lookup. A miss can query every attached site pack until one matches.

This means Turso's async-native engine may be attractive, but making `SitePack::contains()` async would also push async through `NetworkBlock::is_content_blocked()` and the proxy decision path. That API impact must be measured against the benefit.

An alternative is to keep site-pack membership access synchronous/local, or build an explicit async/cache layer rather than letting the ORM choice dictate the request-path API.

---

## 8. What Toasty/Turso provides now

This assessment targets **Toasty 0.10.0** documentation as of 2026-09-05.

Notable current capabilities:

- async ORM API;
- SQLite and Turso drivers with the same SQLite SQL dialect/capability flags;
- model→table and field→column customization;
- single and composite primary keys;
- indexes and unique constraints;
- `belongs_to`, `has_many`, and join-model many-to-many relations;
- create/query/update/delete APIs;
- mutation `RETURNING` support at the driver level;
- targeted upsert for SQLite and Turso;
- `or_ignore()` conflict behavior;
- atomic numeric updates such as increment;
- transactions, nested transactions/savepoints, rollback-on-drop;
- raw SQL execution through `Db`, `Connection`, or `Transaction`;
- pooled connections;
- a Turso local file-backed driver;
- opt-in Turso `concurrent_writes()` / MVCC behavior;
- optional manual Turso remote `push()` / `pull()` sync;
- migration CLI and schema snapshots;
- embedded application migrations (added in Toasty 0.10.0).

This feature set covers almost all of the **query semantics** currently used by `ace-engine`.

---

## 9. Diesel operation → Toasty compatibility matrix

| Current requirement | Toasty/Turso status | Migration note |
|---|---|---|
| Basic select/filter | Good | Direct fit |
| Insert and return model | Good | Toasty create returns model |
| Arbitrary update | Good | Direct fit, but query update returns `()` rather than Diesel `.get_result()` style |
| Delete | Good | Direct fit, but high-level delete returns `()` |
| Composite primary key | Good | Supported; fits `network_block_to_site_pack` |
| Relations/joins | Good | Can model with `belongs_to`/`has_many`; direct filters also work |
| Explicit transactions | Good | Async transactions + nested savepoints |
| Daily counter upsert | Good | Targeted upsert + atomic increment supported on SQLite/Turso |
| Script/settings upsert | Good | Targeted upsert supported |
| `INSERT OR IGNORE` | Good for single-row upsert | `or_ignore()` exists; the high-level upsert API is single-row, so site-pack bulk ingestion needs a different path |
| Mutation `RETURNING` | Driver supports it | High-level create/upsert returns models; update/delete API shape differs from Diesel and may need load-before-mutate or raw SQL |
| Raw SQL/PRAGMA | Good | Raw SQL escape hatch exists; connection-specific PRAGMAs still need deliberate setup |
| Custom CHECK constraints | Partial | Preserve in reviewed migration SQL; do not assume model derive captures every current invariant |
| Custom triggers | Raw SQL required | Main lock trigger and site-pack count triggers must be retained |
| Embedded migrations | Good, new in 0.10 | Separate migration set per DB is supported |
| Existing Diesel migration history | Requires bridge | Toasty tracks `__toasty_migrations`, not `__diesel_schema_migrations` |
| Read-only second process on live DB | **Needs Turso validation** | Main-DB release gate |
| 50k-row bulk insert | **Needs benchmark / lower-level path** | High-level Toasty upsert is one row at a time |
| Atomic local DB/directory swap | Likely compatible with local files | Must verify handles/WAL sidecars are closed/checkpointed before rename |

---

## 10. Important migration gaps and traps

### 10.1 Sync → async is the largest source-code blast radius

Today, repository methods are ordinary synchronous methods:

```text
EngineDB method -> get r2d2 connection -> Diesel query -> Result<T, DBError>
```

With Toasty they naturally become:

```text
async EngineDB method -> Toasty Db/Transaction -> .exec(...).await -> Result<T, DBError>
```

That affects constructors and call chains in:

- `src/manager/account.rs`
- `src/manager/network/mod.rs`
- `src/manager/network/{backup,exceptions,lock,resistance_mode,schedule,site_pack,user_defined}.rs`
- `src/service/content_updater.rs`
- `src/service/bundled_site_packs.rs`
- tests that instantiate/use `EngineDB` synchronously.

I would avoid hiding an async ORM behind `block_on` or `spawn_blocking`. Make the async boundary explicit instead.

### 10.2 Update/delete returning semantics need small rewrites

Diesel currently uses `.get_result()` for several UPDATEs and DELETEs.

Toasty's normal update and delete statement APIs execute to `()`. Instance updates mutate/reload the Rust instance, but query updates do not have the same ergonomic shape as Diesel's `UPDATE ... RETURNING model`.

Affected examples:

- account update methods;
- `set_lock`;
- `delete_exception`;
- user-defined-rule deletes.

Options:

1. load → instance update/delete inside a transaction;
2. perform update then explicit get;
3. use raw `UPDATE/DELETE ... RETURNING` for the few methods where one-round-trip returning behavior matters.

This is a localized gap, not a blocker.

### 10.3 Existing TEXT UUIDs should not silently become BLOBs

Toasty's native `uuid::Uuid` mapping is BLOB by default. Existing `ace-engine` IDs are TEXT.

For an in-place migration, preserve TEXT storage using either `String` or an explicit text column mapping. Do type cleanup later, separately from ORM migration.

### 10.4 Existing migration state needs a baseline bridge

Diesel records applied migrations in `__diesel_schema_migrations`.

Toasty records applied migration IDs in `__toasty_migrations`.

Existing installations already contain all current tables. A new Toasty migration history cannot simply begin with `CREATE TABLE ...` and be blindly applied to those DBs.

The migration needs a one-time adoption strategy, for example:

- ship a bootstrap path that detects the known Diesel schema/version;
- verify required tables/columns/triggers;
- initialize Toasty's migration tracking to an equivalent baseline without replaying destructive/current-schema DDL;
- then apply only Toasty migrations newer than that baseline.

The exact implementation should be prototyped against copies of real installed DBs before changing production migration ownership.

### 10.5 Toasty migration generation must not erase custom invariants

Toasty infers physical schema from model definitions. `ace-engine` has arbitrary SQL constraints/triggers that are not represented by ordinary model fields/indices.

Do not assume generated migration diffs understand all of:

- the lock-row creation trigger;
- the site-pack `RAISE(ABORT)` validation triggers;
- `strftime`-based CHECKs;
- state-dependent CHECK expressions;
- seeded SQL behavior.

For this project, generated migrations should be treated as a **starting point that is reviewed and supplemented with hand-written SQL**, not as an unquestioned source of truth.

Every table-rebuild migration also needs a test that all custom triggers/checks still exist afterward.

### 10.6 Main DB multi-process access is a Turso gate

The uninstall checker and engine service can access the same local file from separate processes.

Toasty's Turso guide exposes `experimental_multiprocess_wal(true)`, which is a warning sign that multi-process WAL behavior is not something to assume from SQLite compatibility. Likewise, `concurrent_writes()` changes the journal mode to Turso MVCC and uses `BEGIN CONCURRENT`.

For the main DB:

- do **not** enable Turso MVCC merely because it exists;
- first reproduce every `lock_check.rs` concurrency test with the Turso-backed engine;
- add a real two-process test, not only two connections in one process;
- verify read-only opening while the service writes;
- verify busy/serialization behavior is still fail-safe;
- verify a crashed writer cannot make the uninstall checker incorrectly report “no locks”.

If this does not pass cleanly, use Toasty's SQLite driver for the main DB even if Turso is used elsewhere.

### 10.7 Site-pack 50k-row insertion is not an ORM-shaped workload

Toasty's typed upsert API is explicitly single-row. Replacing this:

```text
50,000 EntryModel values -> one Diesel INSERT OR IGNORE statement
```

with 50,000 awaited ORM upserts would be a regression unless Toasty batching produces equivalent SQL/throughput.

The site-pack DB is essentially a persisted hash set. The best implementation may be:

- Toasty/Turso for connection ownership and transactions;
- raw SQL / lower-level Turso statements for bulk multi-value insert;
- raw or minimal typed query for `contains`;
- Toasty model only for `pack_meta` if useful.

Using the raw SQL escape hatch here is not a failure of the migration; it is a better match for this workload.

### 10.8 Turso Cloud sync would change the trust/privacy model

The main DB contains more than generic settings. It contains, among other state:

- `accounts.api_key`;
- lock state/end times;
- user-defined rules/exceptions;
- usage counters;
- site-pack/source configuration.

Enabling remote sync for the entire DB would copy local user/security state to a remote service and introduce conflict-resolution/synchronization questions around protection-critical lock state.

Toasty's Turso sync is explicit/manual (`push()` / `pull()`), which is useful, but I would keep it disabled initially.

If cross-device sync becomes a product goal later, split data by trust/sync policy instead of syncing the complete engine DB by default. Toasty's partial sync is currently experimental.

### 10.9 Recovery errors need reclassification

The current startup recovery path treats `DatabaseConnectionError` and `MigrationError` after retries as reasons to rename/delete the database.

Turso surfaces retryable serialization/busy errors separately and can surface connection/IO failures. Do not map all Toasty/Turso failures to the current “corrupt DB” branch. Introduce at least:

- retryable contention/serialization;
- transient IO/connection;
- migration/schema incompatibility;
- confirmed corrupt/unreadable local DB.

Only the last category should trigger destructive recovery.

### 10.10 Deterministic bundle output depends on migration bookkeeping

The site-pack exporter currently normalizes `__diesel_schema_migrations.run_on` because wall-clock migration timestamps would make bundle bytes non-deterministic.

After moving to Toasty migrations:

- inspect the exact `__toasty_migrations` row format;
- either normalize its applied timestamp too, or omit migration bookkeeping from distributed pack DBs if the new design safely allows it;
- add a reproducibility test that builds the same bundle twice and compares bytes/hash.

---

## 11. Model mapping strategy

For the first migration, optimize for **physical-schema compatibility**, not type-system cleanup.

Recommended rules:

1. Use `#[table = "..."]` to map Toasty models to the existing table names exactly.
2. Keep existing column names exactly; use `#[column("...")]` only where Rust naming differs.
3. Keep UUID/date/time fields in their existing TEXT representation initially.
4. Represent `network_block_to_site_pack` with its current composite primary key.
5. Add relations where they genuinely simplify code, but do not require a relationship rewrite to switch the ORM.
6. Preserve the singleton `table_lock = "0"` rows rather than redesigning settings storage during the migration.
7. Preserve all current DB-level checks/triggers in SQL migrations.
8. Avoid adding uniqueness to `site_packs.remote_id` until current data is audited and product semantics explicitly require it.

A later cleanup can move IDs to `Uuid`, use Toasty auto UUID v7 generation, or model state vocabularies as embedded enums. Those are separate schema/domain changes and should not be bundled with the backend migration.

---

## 12. Recommended staged migration

### Phase 0 — characterization and parity tests

Before changing production code:

- capture current schema including `sqlite_master` SQL for tables, indexes, triggers;
- build fixture DBs from a fresh install and from upgraded historical migrations;
- add FK cascade tests with `PRAGMA foreign_keys` explicitly asserted;
- capture representative site-pack sizes and import/query timings;
- keep the current lock-check contention test as a mandatory compatibility test.

### Phase 1 — make `EngineDB` async without changing storage semantics

Refactor the application boundary first.

- Convert `EngineDB` repository methods to async signatures behind the existing facade.
- Propagate async through manager constructors/mutations and content-service call sites.
- Keep `DBError` as the application-facing error abstraction.
- Do not change table layouts or behavior yet.

This separates the architectural async change from the ORM/storage change and makes later diffs reviewable.

If doing this as one branch is too invasive, introduce an internal async DB trait/facade and migrate repository modules incrementally.

### Phase 2 — migrate the main query layer from Diesel to Toasty SQLite

Use Toasty's ordinary SQLite driver first against copies of existing DB files.

Why this intermediate step is valuable:

- proves model/table mapping while preserving familiar SQLite file semantics;
- lets Diesel-specific repository code disappear before Turso-specific behavior is introduced;
- makes parity failures attributable to ORM mapping rather than a new engine;
- keeps `--check-locks` multi-process behavior on conventional SQLite while the async refactor lands.

Port all repository tests and verify query parity.

### Phase 3 — move migration ownership to Toasty 0.10

- Create a Toasty migration/snapshot representation for the current physical schema.
- Build an explicit Diesel→Toasty baseline/adoption path for existing installations.
- Preserve custom CHECKs/triggers in reviewed SQL.
- Embed two migration sets: main DB and site-pack DB, matching the existing split.
- Port migration idempotency/upgrade tests.
- Update the deterministic site-pack exporter bookkeeping behavior.

Only after this phase can Diesel and `diesel_migrations` be removed cleanly from the main runtime.

### Phase 4 — prototype local Turso for site-pack DBs

This is a good isolated Turso experiment because the schema is tiny and the files are replaceable artifacts.

Do **not** port the import loop to naive per-row ORM upserts.

Benchmark:

- 50k-row `INSERT OR IGNORE` batches;
- full import time for realistic large packs;
- file size;
- p50/p95/p99 `contains(host)` latency;
- simultaneous lookups during proxy load;
- connection/open cost across many site packs;
- close/checkpoint/rename/reopen behavior during atomic pack swaps.

If raw Turso/Toasty SQL preserves or improves those numbers, the site-pack store is a strong candidate for Turso local.

### Phase 5 — evaluate Turso for the main DB

Only after the main ORM layer is stable:

- point the same Toasty models at the Turso driver;
- run full repository/migration tests;
- run a true multi-process lock-check test suite;
- test crash/recovery and `.bak` behavior;
- test migrations while another process only reads;
- initially leave `concurrent_writes()` disabled;
- evaluate MVCC only if measured write contention justifies it.

The main DB workload is small enough that concurrent-writer throughput is unlikely to be the primary reason for this change.

### Phase 6 — optional remote sync, as a separate product feature

Only pursue Turso remote sync if there is an actual cross-device/cloud requirement.

If pursued:

- classify tables/columns by sensitivity and authority;
- never let a remote conflict weaken a local active lock;
- decide whether API keys can ever be synchronized;
- define offline behavior and conflict rules;
- use explicit push/pull scheduling;
- consider a separate syncable database rather than syncing the entire engine DB.

---

## 13. Suggested implementation shape

A reasonable end-state keeps the current public concept of `EngineDB` while replacing its internals:

```text
EngineDB
  ├─ Toasty Db handle
  ├─ app DB error mapping
  └─ repository methods
       ├─ typed Toasty queries for normal CRUD
       ├─ explicit transactions for multi-step invariants
       └─ raw SQL for the few SQLite/Turso-specific operations
```

For site packs:

```text
SitePackDB
  ├─ local Turso database/connection pool
  ├─ contains(hash)       -> optimized indexed lookup
  ├─ insert_entries(batch)-> optimized bulk SQL, not N ORM awaits
  ├─ count/metadata       -> typed or raw query
  └─ migration set       -> preserves validation triggers
```

For the uninstall gate, I would keep a deliberately tiny independent reader. If Diesel is removed completely, two viable choices are:

- use a minimal synchronous SQLite reader for `--check-locks`; or
- make `--check-locks` async and use Turso only after the separate-process read-only behavior is proven.

The first option deliberately allows the installer safety path to remain simpler than the application ORM.

---

## 14. Files likely to change in a real migration

Core DB layer:

- `Cargo.toml`, `Cargo.lock`
- `src/db.rs`
- `src/db/models.rs`
- every `src/db/*_repo.rs`
- eventually remove `src/schema_main_db.rs`
- `migrations/` / new Toasty migration history
- `diesel_main_db.toml` (eventually removed)

Site-pack DB:

- `src/core/network_block/site_pack/site_pack_db.rs`
- `src/schema_site_packs.rs` (eventually removed)
- `migrations_site_packs/` / new Toasty migration history
- `diesel_site_packs.toml` (eventually removed)
- importer call sites if `insert_entries` becomes async
- site-pack restore/reload code if DB-handle closing semantics change

Async propagation:

- `src/lib.rs`
- `src/manager/account.rs`
- `src/manager/network/mod.rs`
- `src/manager/network/*.rs` modules that call the DB
- `src/service/content_updater.rs`
- `src/service/bundled_site_packs.rs`
- relevant API/service tests

Direct Diesel escape hatches:

- `src/lock_check.rs`
- `src/bin/export_site_pack_bundle.rs`

Tests:

- DB migration/seed tests in `src/db.rs`
- lock-check tests
- manager tests that construct `EngineDB`
- site-pack DB/import/restore tests
- proxy/API tests using temp DBs.

---

## 15. Acceptance tests before enabling Turso in production

### Main DB correctness

- Fresh DB builds to the expected schema.
- Every historical supported Diesel DB upgrades without data loss.
- UUID TEXT representation is unchanged unless explicitly migrated.
- All seeded rows remain idempotent.
- All CHECK constraints still reject invalid states.
- Network-block insertion still creates a lock row.
- FK cascades are explicitly enabled and tested.
- Schedule replacement is atomic.
- Rule/exception batch insertion is atomic.
- Counter upsert is atomic under concurrent tasks.

### Lock/uninstall safety

- `--check-locks` succeeds when service is down.
- `--check-locks` succeeds while service is running.
- active lock always blocks uninstall.
- malformed/unknown lock state still fails safe.
- read-only checker waits through transient writer contention.
- two **separate processes** can access the live DB safely.
- crash during a write cannot cause false “no active locks”.

### Site-pack correctness/performance

- XXH3 hash→`i64` representation unchanged.
- duplicate hashes remain ignored.
- 50k-row batch semantics preserved.
- pack metadata count trigger still rejects mismatch.
- large import throughput is not materially worse.
- membership p99 latency does not regress under proxy concurrency.
- staged DB can be validated while live DB remains open.
- DB can be closed/checkpointed, atomically swapped, and reopened.
- corrupted/empty staged packs never replace the live pack.

### Packaging/reproducibility

- generated site-pack bundles remain reproducible.
- migration bookkeeping does not introduce wall-clock byte differences.
- no Diesel/libsqlite native dependency remains unintentionally after final cleanup.

---

## 16. Decision recommendation

### Is Toasty more ergonomic than Diesel for this codebase?

Probably yes.

The strongest improvements for `ace-engine` would be:

- model-centric generated CRUD instead of Diesel schema DSL plumbing;
- simpler relations for the block/rule/exception/schedule/source graph;
- concise targeted upserts for counters/settings/script state;
- one async API aligned with the Tokio application;
- easier backend switching between SQLite and Turso;
- raw SQL available when the database-specific behavior is important.

The existing repository boundary also means the ergonomics benefit is concentrated in code that is already isolated.

### Can the main DB use Turso?

**Likely yes technically, but I would not approve it based on SQL compatibility alone.** The release gate is the separate-process uninstall checker and its fail-safe local-read behavior. Prove that before changing the main file's engine/journal semantics.

### Can site-pack DBs use Turso?

**Yes, they are a good candidate for a controlled benchmark.** But the critical API is not ORM CRUD; it is bulk insertion and primary-key membership lookup. Keep those optimized, potentially using Toasty's raw SQL escape hatch or the lower-level Turso API.

### Should `ace-engine` use Turso Cloud sync now?

**No, not as part of the ORM migration.** It provides little value for the current local protection architecture and raises privacy, authority, offline, and lock-conflict questions. Local Turso does not require Turso Cloud.

### Should Diesel be removed in one step?

**No.** The safest sequence is:

1. establish parity tests;
2. move the application DB boundary to async;
3. migrate queries/models to Toasty while preserving local SQLite behavior;
4. adopt Toasty migration ownership with a Diesel-history baseline bridge;
5. remove Diesel from normal runtime code;
6. evaluate Turso separately for site packs and then the main DB;
7. remove the last Diesel-only tooling/checker dependency once equivalent behavior is proven.

This gets the ergonomic benefit early without coupling it to the riskiest storage-engine changes.

---

## 17. Concrete spike I would do next

A small proof-of-concept should answer the remaining unknowns before a full branch is started:

1. Add Toasty 0.10 with both `sqlite` and `turso` features on a branch.
2. Model only:
   - `network_blocks`,
   - `network_block_locks`,
   - `blocked_attempts`, and
   - the site-pack `entries`/`pack_meta` tables.
3. Open a **copy of an existing Diesel-created DB** with Toasty without recreating tables.
4. Exercise:
   - normal load/insert/update;
   - lock trigger behavior;
   - counter upsert + increment;
   - explicit transaction rollback;
   - custom raw SQL.
5. Repeat with the local Turso driver.
6. Run the lock checker from a separate process while Turso owns/writes the main DB.
7. Benchmark one realistic site-pack import with the existing 50k-batch strategy implemented via raw/low-level SQL.
8. Compare p99 site-pack membership lookup latency against current Diesel SQLite.
9. Prototype the Diesel→Toasty migration baseline on an upgraded fixture database.

If those pass, there is no obvious architectural reason the broader migration cannot proceed.

---

## 18. External references checked

Current Toasty information was verified against the project/API documentation on 2026-09-05. Toasty is moving quickly, so pin the exact release and re-run this compatibility check before implementation.

- Toasty repository / overview: https://github.com/tokio-rs/toasty
- Toasty 0.10 API docs: https://docs.rs/toasty/0.10.0/toasty/
- Toasty 0.10 changelog (0.10.0 released 2026-08-11; embedded migrations added): https://docs.rs/crate/toasty/0.10.0/source/CHANGELOG.md
- Toasty model macro attributes (`#[table]`, keys, composite keys, etc.): https://docs.rs/toasty-macros/0.10.0/toasty_macros/derive.Model.html
- Toasty database handle/pool: https://docs.rs/toasty/0.10.0/toasty/db/
- Toasty transactions: https://docs.rs/toasty/0.10.0/toasty/db/struct.Transaction.html
- Toasty raw SQL: https://docs.rs/toasty/0.10.0/toasty/sql/
- Toasty Turso driver: https://docs.rs/toasty-driver-turso/0.10.0/toasty_driver_turso/
- Toasty Turso guide (local DB, MVCC, remote sync, experimental multiprocess WAL): https://tokio-rs.github.io/toasty/nightly/guide/turso.html
- Toasty SQLite guide (type mapping, UUID TEXT override, `RETURNING`, migrations): https://tokio-rs.github.io/toasty/nightly/guide/sqlite.html
- Toasty migration/schema management guide: https://tokio-rs.github.io/toasty/nightly/guide/schema-management.html
- Toasty upsert behavior: https://tokio-rs.github.io/toasty/nightly/guide/upserting-records.html
- Toasty delete behavior: https://tokio-rs.github.io/toasty/nightly/guide/deleting-records.html
- Toasty update behavior: https://tokio-rs.github.io/toasty/nightly/guide/updating-records.html

### Documentation freshness note

During research, GitHub's indexed `main` roadmap still showed parts of migration work as planned, while the newer published 0.10.0 docs/changelog show that migration CLI support and embedded migrations have landed. This report uses the published **0.10.0** behavior as the current release reference and treats migration tooling as newly available rather than absent.

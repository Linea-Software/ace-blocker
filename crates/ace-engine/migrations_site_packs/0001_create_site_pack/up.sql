-- Site pack content database.
-- One SQLite file per site pack (replaces the per-pack RocksDB directory).
-- `entries` stores the 64-bit XXH3 host hashes, `pack_meta` the pack metadata.

CREATE TABLE IF NOT EXISTS entries (
    host_hash INTEGER PRIMARY KEY NOT NULL
);

CREATE TABLE IF NOT EXISTS pack_meta (
    id TEXT PRIMARY KEY NOT NULL,
    format_version TEXT NOT NULL,
    name TEXT,
    entry_count INTEGER NOT NULL DEFAULT 0,
    built_at TEXT NOT NULL
);

-- SQLite does not allow subqueries inside CHECK constraints, so the intended
-- invariant `entry_count = COUNT(entries)` is enforced with triggers instead.
-- They validate whenever pack_meta is written (the same point in time the
-- CHECK constraint would have been evaluated).

CREATE TRIGGER IF NOT EXISTS pack_meta_validate_entry_count_on_insert
AFTER INSERT ON pack_meta
WHEN NEW.entry_count <> (SELECT COUNT(*) FROM entries)
BEGIN
    SELECT RAISE(ABORT, 'pack_meta.entry_count must equal the entries row count');
END;

CREATE TRIGGER IF NOT EXISTS pack_meta_validate_entry_count_on_update
AFTER UPDATE OF entry_count ON pack_meta
WHEN NEW.entry_count <> (SELECT COUNT(*) FROM entries)
BEGIN
    SELECT RAISE(ABORT, 'pack_meta.entry_count must equal the entries row count');
END;

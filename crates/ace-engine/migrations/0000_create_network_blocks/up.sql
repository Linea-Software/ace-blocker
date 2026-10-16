-- Main List Table
-- This is the central table, acting as the "parent" for all list types.
CREATE TABLE IF NOT EXISTS network_blocks (
    id TEXT NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL,
    safe_search BOOLEAN NOT NULL DEFAULT 0
);
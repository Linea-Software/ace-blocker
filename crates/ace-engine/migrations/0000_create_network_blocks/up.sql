-- Main List Table
-- This is the central table, acting as the "parent" for all list types.
CREATE TABLE IF NOT EXISTS network_blocks (
    id TEXT NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL,
    safe_search BOOLEAN NOT NULL DEFAULT 0,
    icon TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    schedule_timezone TEXT NOT NULL DEFAULT '__legacy_system_timezone__',
    schedule_mode TEXT NOT NULL DEFAULT 'always'
        CHECK (schedule_mode IN ('always', 'scheduled'))
);
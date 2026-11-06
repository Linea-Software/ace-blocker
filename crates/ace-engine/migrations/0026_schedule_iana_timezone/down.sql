-- SQLite >= 3.35 supports DROP COLUMN; pre-release schema.
ALTER TABLE network_blocks DROP COLUMN schedule_timezone;

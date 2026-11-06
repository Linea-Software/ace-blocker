-- The timezone belongs to the policy, not to the mutable OS setting.
-- Existing rows require a one-time capture of the installed machine's IANA zone.
-- EngineDB::from pins this sentinel before loading policies. SQLite cannot
-- determine a canonical IANA zone from the operating system by itself.
ALTER TABLE network_blocks ADD COLUMN schedule_timezone TEXT NOT NULL DEFAULT '__legacy_system_timezone__';

-- Keep legacy and user-created blocks enabled as before: the new flag must
-- not silently change an existing installation's enforcement.
ALTER TABLE network_blocks ADD COLUMN enabled BOOLEAN NOT NULL DEFAULT 1;

-- Only fresh starter blocks contain the fixed 24/7 schedule IDs inserted by
-- the updated 0021 seed. Earlier installations never received these IDs, so
-- their enabled state and schedules stay untouched on upgrade.
UPDATE network_blocks
SET enabled = 0
WHERE id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
)
AND EXISTS (
    SELECT 1 FROM network_block_schedule_entries AS entry
    WHERE entry.network_block_id = network_blocks.id
      AND entry.id IN (
          '01950000-0000-7000-8000-000000001001',
          '01950000-0000-7000-8000-000000001002',
          '01950000-0000-7000-8000-000000001003',
          '01950000-0000-7000-8000-000000001004'
      )
      AND entry.start_time = '00:00:00'
      AND entry.end_time = '00:00:00'
      AND entry.schedule_days_bitmask = 127
);

-- Activation is independent of the enabled switch and schedule rows.
-- Existing explicitly scheduled blocks preserve their windows; old empty
-- blocks now have the same Always semantics as newly created blocks.
ALTER TABLE network_blocks ADD COLUMN schedule_mode TEXT NOT NULL DEFAULT 'always' CHECK (schedule_mode IN ('always','scheduled'));
UPDATE network_blocks
SET schedule_mode = 'scheduled'
WHERE EXISTS (
    SELECT 1 FROM network_block_schedule_entries AS entry
    WHERE entry.network_block_id = network_blocks.id
);

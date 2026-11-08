-- Remove the seeded default network blocks. Children are deleted explicitly
-- because SQLite only honours ON DELETE CASCADE when `PRAGMA foreign_keys` is on.

DELETE FROM user_defined_rules WHERE network_block_id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
);

DELETE FROM network_block_exceptions WHERE network_block_id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
);

DELETE FROM network_block_schedule_entries WHERE network_block_id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
);

DELETE FROM network_block_to_site_pack WHERE network_block_id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
);

DELETE FROM network_block_locks WHERE network_block_id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
);

DELETE FROM network_blocks WHERE id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
);

-- Remove only the pack rows created by this first-install preset. This
-- migration is for disposable pre-release databases.
DELETE FROM site_packs WHERE id IN ('pack-adult-free', 'pack-social-free', 'pack-gambling-free', 'pack-streaming-free');

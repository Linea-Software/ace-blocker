-- Reverses 0022: drops the pack connections, the anchored curated rules (and
-- any other rules on the seeded blocks), and the two seeded site_packs rows.
--
-- NOTE: this does NOT restore the original 0021 regex lists verbatim — 0021's
-- rules were deleted in 0022's up and their rule IDs were generated randomly
-- at seed time, so they cannot be recreated deterministically. The down only
-- exists for dev rollbacks; a fresh 0021 → 0022 upgrade path is unaffected.
-- Also note that if a pack was already downloaded, its pack.db directory is
-- left on disk (the row is gone, but SQLite FK enforcement is off during
-- migrations so the delete succeeds and the updater will re-create the row on
-- the next sync).

-- 1. Drop the pack connections added by 0022.
DELETE FROM network_block_to_site_pack WHERE network_block_id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002'
);

-- 2. Drop the anchored rules (and any rules added on the seeded blocks).
DELETE FROM user_defined_rules WHERE network_block_id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
);

-- 3. Drop the seeded free site packs.
DELETE FROM site_packs WHERE id IN (
    'pack-adult-free',
    'pack-social-free'
);

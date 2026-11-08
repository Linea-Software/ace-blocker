-- First-install starter policies. This pre-release baseline creates fully
-- formed starter rows directly, without requiring later repair migrations.
-- All four are disabled until onboarding explicitly enables the selected ones.
-- Always means 24/7; no schedule rows are inserted for these policies.
INSERT INTO network_blocks (id, "name", safe_search, icon, enabled, schedule_mode)
VALUES
 ('01950000-0000-7000-8000-000000000001', 'Adult Content', 1, 'adult', 0, 'always'),
 ('01950000-0000-7000-8000-000000000002', 'Social Media', 0, 'social', 0, 'always'),
 ('01950000-0000-7000-8000-000000000003', 'Gambling & Betting', 0, 'gambling', 0, 'always'),
 ('01950000-0000-7000-8000-000000000004', 'Video & Streaming', 0, 'streaming', 0, 'always');

-- Free catalog rows are stable so the updater and installed bundle can attach
-- content by remote_id without orphaning connections.
INSERT OR IGNORE INTO site_packs (id, remote_id, name, description, requires_pro, is_ignored, icon)
VALUES
    ('pack-adult-free', 'pack-adult-free', 'Adult Websites — Free', 'Blocks known adult content websites.', 0, 0, 'adult'),
    ('pack-social-free', 'pack-social-free', 'Social Media Websites — Free', 'Blocks social media sites.', 0, 0, 'social'),
    ('pack-gambling-free', 'pack-gambling-free', 'Gambling Websites — Free', 'Blocks popular gambling and betting sites.', 0, 0, 'gambling'),
    ('pack-streaming-free', 'pack-streaming-free', 'Streaming Websites — Free', 'Blocks popular video services.', 0, 0, 'streaming');

-- Every preselected category is Free; Pro upgrades are never a dependency.
INSERT OR IGNORE INTO network_block_to_site_pack (network_block_id, site_pack_id) VALUES
    ('01950000-0000-7000-8000-000000000001', 'pack-adult-free'),
    ('01950000-0000-7000-8000-000000000002', 'pack-social-free'),
    ('01950000-0000-7000-8000-000000000003', 'pack-gambling-free'),
    ('01950000-0000-7000-8000-000000000004', 'pack-streaming-free');

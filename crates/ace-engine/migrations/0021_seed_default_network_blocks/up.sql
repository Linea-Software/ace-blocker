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
    ('pack-social-free', 'pack-social-free', 'Social Media Websites — Free', 'Blocks social media sites.', 0, 0, 'social');

-- Starter-specific curated domain rules. These are typed host matches, not
-- unanchored URL regexes; later work can replace them with Free category packs.
INSERT OR IGNORE INTO user_defined_rules (id, network_block_id, rule, rule_type)
SELECT
    lower(
        hex(randomblob(4)) || '-' ||
        hex(randomblob(2)) || '-4' ||
        substr(hex(randomblob(2)), 2) || '-' ||
        substr('89ab', abs(random()) % 4 + 1, 1) ||
        substr(hex(randomblob(2)), 2) || '-' ||
        hex(randomblob(6))
    ),
    column1,
    column2,
    'domain'
FROM (
    VALUES
        -- Gambling & Betting
        ('01950000-0000-7000-8000-000000000003', 'bet365.com'),
        ('01950000-0000-7000-8000-000000000003', 'betway.com'),
        ('01950000-0000-7000-8000-000000000003', 'betfair.com'),
        ('01950000-0000-7000-8000-000000000003', 'williamhill.com'),
        ('01950000-0000-7000-8000-000000000003', 'ladbrokes.com'),
        ('01950000-0000-7000-8000-000000000003', 'paddypower.com'),
        ('01950000-0000-7000-8000-000000000003', 'unibet.com'),
        ('01950000-0000-7000-8000-000000000003', 'bwin.com'),
        ('01950000-0000-7000-8000-000000000003', 'pokerstars.com'),
        ('01950000-0000-7000-8000-000000000003', '888casino.com'),
        ('01950000-0000-7000-8000-000000000003', 'stake.com'),
        ('01950000-0000-7000-8000-000000000003', 'roobet.com'),
        ('01950000-0000-7000-8000-000000000003', 'draftkings.com'),
        ('01950000-0000-7000-8000-000000000003', 'fanduel.com'),

        -- Video & Streaming
        ('01950000-0000-7000-8000-000000000004', 'youtube.com'),
        ('01950000-0000-7000-8000-000000000004', 'netflix.com'),
        ('01950000-0000-7000-8000-000000000004', 'twitch.tv'),
        ('01950000-0000-7000-8000-000000000004', 'hulu.com'),
        ('01950000-0000-7000-8000-000000000004', 'disneyplus.com'),
        ('01950000-0000-7000-8000-000000000004', 'primevideo.com'),
        ('01950000-0000-7000-8000-000000000004', 'dailymotion.com'),
        ('01950000-0000-7000-8000-000000000004', 'vimeo.com'),
        ('01950000-0000-7000-8000-000000000004', 'crunchyroll.com')
);

-- Every preselected category is Free; Pro upgrades are never a dependency.
INSERT OR IGNORE INTO network_block_to_site_pack (network_block_id, site_pack_id) VALUES
    ('01950000-0000-7000-8000-000000000001', 'pack-adult-free'),
    ('01950000-0000-7000-8000-000000000002', 'pack-social-free');

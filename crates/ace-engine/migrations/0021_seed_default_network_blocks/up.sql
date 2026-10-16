-- Seed the default network blocks that a brand new installation starts with.
--
-- Each starter has a 24/7 schedule from installation but is initially
-- disabled by 0023. Onboarding enables selected blocks independently of their
-- schedules. User-defined schedules can be customized afterward.
--
-- The block IDs are fixed so the app can recognise the seeded blocks and so a
-- re-run of the migration can never duplicate them.

INSERT OR IGNORE INTO network_blocks (id, "name", safe_search, icon) VALUES
    ('01950000-0000-7000-8000-000000000001', 'Adult Content',      1, 'adult'),
    ('01950000-0000-7000-8000-000000000002', 'Social Media',       0, 'social'),
    ('01950000-0000-7000-8000-000000000003', 'Gambling & Betting', 0, 'gambling'),
    ('01950000-0000-7000-8000-000000000004', 'Video & Streaming',  0, 'streaming');

-- The fixed entry IDs distinguish schedules seeded on a NEW installation
-- from existing users' own schedules. 00:00–00:00 means all day to the
-- engine, and bitmask 127 covers all seven days.
INSERT OR IGNORE INTO network_block_schedule_entries
    (id, network_block_id, start_time, end_time, schedule_days_bitmask)
SELECT entry_id, block_id, '00:00:00', '00:00:00', 127
FROM (
    SELECT '01950000-0000-7000-8000-000000001001' AS entry_id,
           '01950000-0000-7000-8000-000000000001' AS block_id
    UNION ALL SELECT '01950000-0000-7000-8000-000000001002', '01950000-0000-7000-8000-000000000002'
    UNION ALL SELECT '01950000-0000-7000-8000-000000001003', '01950000-0000-7000-8000-000000000003'
    UNION ALL SELECT '01950000-0000-7000-8000-000000001004', '01950000-0000-7000-8000-000000000004'
) AS defaults
WHERE EXISTS (SELECT 1 FROM network_blocks WHERE id = block_id)
  AND NOT EXISTS (
      SELECT 1 FROM network_block_schedule_entries
      WHERE network_block_id = block_id
  );

-- Rule IDs must be UUIDs (the engine parses them as `uuid::Uuid`), so generate a
-- v4 UUID per row instead of hardcoding dozens of literals.
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
        -- Adult Content
        ('01950000-0000-7000-8000-000000000001', 'pornhub.com'),
        ('01950000-0000-7000-8000-000000000001', 'xvideos.com'),
        ('01950000-0000-7000-8000-000000000001', 'xnxx.com'),
        ('01950000-0000-7000-8000-000000000001', 'xhamster.com'),
        ('01950000-0000-7000-8000-000000000001', 'redtube.com'),
        ('01950000-0000-7000-8000-000000000001', 'youporn.com'),
        ('01950000-0000-7000-8000-000000000001', 'spankbang.com'),
        ('01950000-0000-7000-8000-000000000001', 'eporner.com'),
        ('01950000-0000-7000-8000-000000000001', 'tnaflix.com'),
        ('01950000-0000-7000-8000-000000000001', 'motherless.com'),
        ('01950000-0000-7000-8000-000000000001', 'brazzers.com'),
        ('01950000-0000-7000-8000-000000000001', 'chaturbate.com'),
        ('01950000-0000-7000-8000-000000000001', 'stripchat.com'),
        ('01950000-0000-7000-8000-000000000001', 'bongacams.com'),
        ('01950000-0000-7000-8000-000000000001', 'livejasmin.com'),
        ('01950000-0000-7000-8000-000000000001', 'onlyfans.com'),
        ('01950000-0000-7000-8000-000000000001', 'fansly.com'),
        ('01950000-0000-7000-8000-000000000001', 'rule34.xxx'),
        ('01950000-0000-7000-8000-000000000001', 'nhentai.net'),
        ('01950000-0000-7000-8000-000000000001', 'e-hentai.org'),

        -- Social Media
        ('01950000-0000-7000-8000-000000000002', 'facebook.com'),
        ('01950000-0000-7000-8000-000000000002', 'instagram.com'),
        ('01950000-0000-7000-8000-000000000002', 'tiktok.com'),
        ('01950000-0000-7000-8000-000000000002', 'twitter.com'),
        ('01950000-0000-7000-8000-000000000002', 'x.com'),
        ('01950000-0000-7000-8000-000000000002', 'reddit.com'),
        ('01950000-0000-7000-8000-000000000002', 'snapchat.com'),
        ('01950000-0000-7000-8000-000000000002', 'threads.net'),
        ('01950000-0000-7000-8000-000000000002', 'tumblr.com'),
        ('01950000-0000-7000-8000-000000000002', 'pinterest.com'),
        ('01950000-0000-7000-8000-000000000002', '9gag.com'),
        ('01950000-0000-7000-8000-000000000002', 'vk.com'),

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

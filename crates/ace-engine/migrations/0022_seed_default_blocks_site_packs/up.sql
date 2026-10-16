-- Hybrid default-block seeding (founder decision: seed regex over-match).
--
-- 0021 seeded the four default blocks with plain domain strings used as
-- regexes. Those rules are tested against the FULL URI string with unanchored
-- substring semantics (`RegexSet::is_match`), so `x.com` also matched
-- `x.community`, `notx.com`, etc. The founder's hybrid decision replaces the
-- over-matching seeds:
--
--   * Adult Content (...0001) and Social Media (...0002) drop their curated
--     regex lists and connect to their FREE site packs instead (pack data is
--     delivered by the content updater syncing from the Ace Server).
--   * Gambling & Betting (...0003) and Video & Streaming (...0004) keep the
--     SAME curated domain set, re-anchored to host boundaries.
--
-- Pack ID chain (verified against src/service/content_updater.rs and
-- src/manager/network/site_pack.rs): the engine's sync flow stores the server
-- catalog id in `site_packs.remote_id` and generates a FRESH v7 UUID for
-- `site_packs.id`; `network_block_to_site_pack.site_pack_id` references
-- `site_packs.id`. Join rows pointing at runtime UUIDs would therefore
-- dangle (the manager skips dangling pack references at load — MGR-01). So we
-- seed the two `site_packs` rows here with id = remote_id = the canonical
-- catalog id from ace-app/src/lib/data/site-packs.json (`pack-adult-free`,
-- `pack-social-free`; ids are plain strings, never UUID-parsed — see
-- `SitePackId`). The updater matches packs by remote_id
-- (`get_site_pack_by_remote_id`) and skips ones that already exist, so these
-- rows are recognised and never duplicated; `commit_site_pack_update` later
-- delivers the pack data in place on the SAME row (MGR-01), keeping the join
-- rows valid.
--
-- The blocks retain the 24/7 schedules seeded by 0021 on fresh installs.
-- 0023 keeps their Enabled flag off until the user opts in.

-- 1. Seed the two free site packs the pack-backed blocks connect to.
--    Names/icons match the approved user-facing values (adult/social packs
--    are Free tier; icons are the canonical slug vocabulary resolved by
--    ace-app/src/lib/utils/pack-icons.ts).
INSERT OR IGNORE INTO site_packs (id, remote_id, name, description, requires_pro, is_ignored, icon)
VALUES
    ('pack-adult-free', 'pack-adult-free', 'Adult Websites — Free', 'Blocks known adult content websites.', 0, 0, 'adult'),
    ('pack-social-free', 'pack-social-free', 'Social Media Websites — Free', 'Blocks social media sites.', 0, 0, 'social');

-- 2. Drop the 0021 seed rules for the four default blocks. Adult/social are
--    replaced by site packs below; gambling/video are re-seeded as typed domain
--    rules so host matching uses exact domain + subdomain semantics.
DELETE FROM user_defined_rules WHERE network_block_id IN (
    '01950000-0000-7000-8000-000000000001',
    '01950000-0000-7000-8000-000000000002',
    '01950000-0000-7000-8000-000000000003',
    '01950000-0000-7000-8000-000000000004'
);

-- 3. Re-seed the two blocks that keep curated domains. A `domain` rule is
--    matched against the parsed request hostname and includes true subdomains:
--    `x.com` matches `x.com` and `www.x.com`, but not `notx.com`, `x.community`,
--    or `x.com.evil.com`. Rule IDs must be UUIDs, so reuse 0021's v4 generation.
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

-- 4. Connect the pack-backed blocks to their free site packs.
INSERT OR IGNORE INTO network_block_to_site_pack (network_block_id, site_pack_id) VALUES
    ('01950000-0000-7000-8000-000000000001', 'pack-adult-free'),
    ('01950000-0000-7000-8000-000000000002', 'pack-social-free');

CREATE TABLE IF NOT EXISTS site_pack_sources (
    id TEXT NOT NULL PRIMARY KEY,
    site_pack_id TEXT NOT NULL,
    source TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type in ('url', 'file')),
    format TEXT NOT NULL DEFAULT 'standard' CHECK (format in ('standard', 'wildcard')),

    FOREIGN KEY(site_pack_id) REFERENCES site_packs(id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_site_pack_sources_pack_id
    ON site_pack_sources(site_pack_id);

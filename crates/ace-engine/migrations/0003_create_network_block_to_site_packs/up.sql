-- Linking Table (Many-to-Many)
-- Connects a NetworkBlock to one or more SitePacks.
-- This allows multiple blocks to reuse the same set of site packs.
CREATE TABLE IF NOT EXISTS network_block_to_site_pack (
    network_block_id TEXT NOT NULL,
    site_pack_id TEXT NOT NULL,

    FOREIGN KEY(network_block_id) REFERENCES network_blocks(id)
    	ON DELETE CASCADE,
    FOREIGN KEY(site_pack_id) REFERENCES site_packs(id)
    	ON DELETE CASCADE,

    -- Ensures a block can't have the same pack more than once
    PRIMARY KEY (network_block_id, site_pack_id)
);

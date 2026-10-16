CREATE TABLE IF NOT EXISTS network_block_exceptions(
    id TEXT NOT NULL PRIMARY KEY,
    network_block_id TEXT NOT NULL,
    rule TEXT NOT NULL,

    FOREIGN KEY(network_block_id) REFERENCES network_blocks(id) 
        ON DELETE CASCADE

    UNIQUE (network_block_id, rule)
)
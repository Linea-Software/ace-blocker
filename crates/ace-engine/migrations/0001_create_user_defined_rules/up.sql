-- User-Defined Rules
-- Holds individual user-defined rules (like a single domain).
CREATE TABLE IF NOT EXISTS user_defined_rules (
    id TEXT NOT NULL PRIMARY KEY,
    network_block_id TEXT NOT NULL,
    rule TEXT NOT NULL,
    rule_type TEXT NOT NULL CHECK(rule_type IN ('domain', 'regex')),
    
    FOREIGN KEY(network_block_id) REFERENCES network_blocks(id) 
        ON DELETE CASCADE

    UNIQUE(network_block_id, rule, rule_type)
);

-- Index for faster lookups by list
CREATE INDEX IF NOT EXISTS idx_user_rule_list_id 
    ON user_defined_rules(network_block_id);
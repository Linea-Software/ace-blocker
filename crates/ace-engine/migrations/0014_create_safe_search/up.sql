-- Create safe_search_enforcements table
CREATE TABLE IF NOT EXISTS safe_search_enforcements (
    domain TEXT NOT NULL PRIMARY KEY,
    enforcement_type TEXT NOT NULL,
    
    -- Used for domain_redirection
    target_host TEXT,
    
    -- Used for query_parameter
    query_param_key TEXT,
    query_param_value TEXT,

    CONSTRAINT valid_enforcement_type 
        CHECK (enforcement_type IN ('domain_redirection', 'query_parameter')),

    CONSTRAINT valid_enforcement_state
        CHECK (
            (enforcement_type = 'domain_redirection' AND target_host IS NOT NULL AND query_param_key IS NULL AND query_param_value IS NULL)
            OR
            (enforcement_type = 'query_parameter' AND target_host IS NULL AND query_param_key IS NOT NULL AND query_param_value IS NOT NULL)
        )
);

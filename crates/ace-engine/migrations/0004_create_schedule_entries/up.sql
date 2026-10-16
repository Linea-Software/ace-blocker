CREATE TABLE IF NOT EXISTS network_block_schedule_entries (
    id TEXT NOT NULL PRIMARY KEY,
    network_block_id TEXT NOT NULL,

    -- Stores start time as "HH:MM:SS" text
    start_time TEXT NOT NULL,

    -- Stores end time as "HH:MM:SS" text
    end_time TEXT NOT NULL,

    -- schedule_days TEXT NOT NULL,
    schedule_days_bitmask INTEGER NOT NULL,

    FOREIGN KEY(network_block_id) REFERENCES network_blocks(id)
        ON DELETE CASCADE


    -- CONSTRAINT: Ensure start_time is strictly HH:MM:SS and valid
    CONSTRAINT valid_start_format
        CHECK (start_time IS strftime('%H:%M:%S', start_time)),

    -- CONSTRAINT: Ensure end_time is strictly HH:MM:SS and valid
    CONSTRAINT valid_end_format
        CHECK (end_time IS strftime('%H:%M:%S', end_time))

    -- CONSTRAINT: The schedule bitmask should be a valid 7-bit value
    CONSTRAINT valid_u8_bitmask
        CHECK (schedule_days_bitmask BETWEEN 0 AND 127)
);

-- Index for faster schedule lookups by list
CREATE INDEX IF NOT EXISTS idx_net_schedule_list_id
    ON network_block_schedule_entries(network_block_id);

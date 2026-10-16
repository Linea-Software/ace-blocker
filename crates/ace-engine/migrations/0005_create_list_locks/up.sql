CREATE TABLE network_block_locks (
    network_block_id TEXT NOT NULL PRIMARY KEY,

    -- active_during_schedule -> active when list is active
    -- active_until_lock_end -> active until time surpasses lock_end
    lock_state TEXT NOT NULL DEFAULT "inactive",

    -- if null, it never ends
    -- if not null, ends at: "YYYY-MM-DD HH:MM:SS"
    lock_end TEXT,

    FOREIGN KEY(network_block_id) REFERENCES network_blocks(id)
        ON DELETE CASCADE,

    -- CONSTRAINT: Lock type must valid
    CONSTRAINT valid_lock_type
        CHECK (lock_state IN ("inactive", "soft_unlockable", "active_during_schedule", "active_until_lock_end")),

    -- CONSTRAINT: lock_end is either NULL OR a valid ISO 8601 datetime
    CONSTRAINT valid_lock_end_format
        CHECK (
            lock_end IS NULL
            OR
            strftime('%Y-%m-%dT%H:%M:%S', lock_end) IS NOT NULL
        )

    CONSTRAINT valid_lock_end_state
        CHECK (
            ((lock_end IS NOT NULL) AND (lock_state = "active_until_lock_end"))
            or ((lock_end IS NULL) AND (lock_state !=  "active_until_lock_end"))
        )
);

-- Triggers to create locks for new lists
CREATE TRIGGER insert_network_block_lock AFTER INSERT ON network_blocks
BEGIN
    INSERT INTO network_block_locks (network_block_id, lock_state, lock_end)
    VALUES (NEW.id, "inactive", NULL);
END;

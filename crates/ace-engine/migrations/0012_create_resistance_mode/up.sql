CREATE TABLE resistance_mode_settings (
    table_lock CHAR(1) PRIMARY KEY NOT NULL DEFAULT '0' CHECK (table_lock = '0'),
    enabled BOOLEAN NOT NULL DEFAULT FALSE,

    lock_state TEXT NOT NULL DEFAULT "inactive",
    lock_end TEXT,

    -- Static settings
    static_intensity TEXT NOT NULL DEFAULT "none",

    scripts_version TEXT,

    -- CONSTRAINT: Intensity types must be valid
    CONSTRAINT valid_static_intensity_type
        CHECK (static_intensity IN ("none", "low", "medium", "high", "extreme")),

    -- CONSTRAINT: Lock type must valid
    CONSTRAINT valid_lock_type
        CHECK (lock_state IN ("inactive", "active_until_lock_end")),

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

    -- Lock always null if the engine is not enabled.
    -- If enabled is FALSE, lock_state MUST be "inactive".
    CONSTRAINT lock_null_if_disabled
        CHECK (
            enabled = TRUE OR lock_state = "inactive"
        )
);

INSERT INTO resistance_mode_settings (table_lock) VALUES ('0');

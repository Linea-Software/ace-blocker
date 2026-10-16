CREATE TABLE settings (
    table_lock CHAR(1) PRIMARY KEY NOT NULL DEFAULT '0' CHECK (table_lock = '0'),
    
    is_pro BOOLEAN NOT NULL DEFAULT FALSE,
    is_seeded BOOLEAN NOT NULL DEFAULT FALSE,
    latest_update_check_at TEXT
);  

INSERT INTO settings (table_lock) VALUES ('0');

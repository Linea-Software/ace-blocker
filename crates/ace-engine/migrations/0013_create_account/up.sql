CREATE TABLE accounts (
    table_lock CHAR(1) PRIMARY KEY NOT NULL DEFAULT '0' CHECK (table_lock = '0'),
    api_key TEXT,
    last_pro_check TEXT
);

INSERT INTO accounts (table_lock, api_key) VALUES ('0', NULL);

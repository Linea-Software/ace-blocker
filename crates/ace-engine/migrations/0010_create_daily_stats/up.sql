CREATE TABLE blocked_attempts (
    date TEXT PRIMARY KEY NOT NULL,
    attempts_count INTEGER NOT NULL DEFAULT 0
);

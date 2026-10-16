CREATE TABLE IF NOT EXISTS proxy_exclusions (
    host TEXT NOT NULL PRIMARY KEY
);

-- Intentionally empty. Ace keeps browser TLS opaque by default and only
-- intercepts hosts when an active feature requires decrypted HTTP, so a
-- growing compatibility allow-list is neither necessary nor desirable.

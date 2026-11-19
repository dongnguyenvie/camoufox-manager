-- 0007_settings — named API keys (phase 12). The secret is never stored: only sha256(salt || secret).
CREATE TABLE api_keys (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    salt         TEXT NOT NULL,
    hash         TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at   TEXT
);

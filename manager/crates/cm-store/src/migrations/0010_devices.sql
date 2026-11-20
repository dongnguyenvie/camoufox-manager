-- 0010_devices — paired devices for sync (phase 16). Only public keys are stored; this install's
-- own private key lives in <data_dir>/sync/device.key (0600), never in the database.
CREATE TABLE IF NOT EXISTS devices (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    public_key      TEXT NOT NULL UNIQUE,
    url             TEXT,
    tailscale_login TEXT,
    paired_at       TEXT NOT NULL,
    last_seen_at    TEXT,
    last_seen_addr  TEXT,
    revoked_at      TEXT
);

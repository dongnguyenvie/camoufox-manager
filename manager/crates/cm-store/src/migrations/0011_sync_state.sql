-- 0011_sync_state — device sync bookkeeping (phase 17).
-- device_id = ''  : the profile's own current state (content hash + revision counter).
-- device_id = <d> : what was agreed with paired device <d> at the last successful sync of this
--                   profile (rev / content_hash at that time, the profile's id over there).
CREATE TABLE IF NOT EXISTS sync_state (
    device_id        TEXT NOT NULL,
    profile_id       TEXT NOT NULL,
    rev              INTEGER NOT NULL DEFAULT 0,
    content_hash     TEXT NOT NULL DEFAULT '',
    last_synced_hash TEXT,
    peer_profile_id  TEXT,
    updated_at       TEXT NOT NULL,
    PRIMARY KEY (device_id, profile_id)
);
CREATE INDEX IF NOT EXISTS sync_state_peer ON sync_state (device_id, peer_profile_id);

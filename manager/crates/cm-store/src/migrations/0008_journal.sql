-- Snapshot apply journal (phase 15): one row per journalled import-over-existing.
-- state: preparing | prepared | publishing | complete | aborted. Anything but the last two is
-- "unresolved": recovered on next startup and it blocks starting the profile.
CREATE TABLE IF NOT EXISTS snapshot_journal (
    id          TEXT PRIMARY KEY,
    profile_id  TEXT NOT NULL,
    state       TEXT NOT NULL CHECK (state IN ('preparing','prepared','publishing','complete','aborted')),
    staging     TEXT NOT NULL,
    backup      TEXT NOT NULL,
    inventory   TEXT NOT NULL DEFAULT '[]',
    record      TEXT,
    error       TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_snapshot_journal_profile ON snapshot_journal (profile_id, state);

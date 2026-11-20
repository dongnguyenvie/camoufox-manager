-- 0009_notifications — in-app notifications (phase 19). Local only.
CREATE TABLE notifications (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    kind       TEXT NOT NULL,
    level      TEXT NOT NULL DEFAULT 'info',
    title      TEXT NOT NULL,
    body       TEXT NOT NULL DEFAULT '',
    params     TEXT NOT NULL DEFAULT '{}',
    dedupe     TEXT UNIQUE,
    created_at INTEGER NOT NULL,
    read       INTEGER NOT NULL DEFAULT 0,
    -- "Clear" hides rows but keeps their dedupe key, so a crash still shown by the overview is not reported again.
    cleared    INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX notifications_created ON notifications (created_at DESC, id DESC);

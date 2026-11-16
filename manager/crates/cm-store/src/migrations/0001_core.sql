-- 0001_core — phase 1 (Foundation): base tables every area builds on.
-- Areas add their own tables / columns in their own migration file (see migrate.rs).

-- Key/value app settings (value is JSON).
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Proxy library. Passwords are local plaintext for now (D2); never sent to the UI.
CREATE TABLE proxies (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    scheme     TEXT NOT NULL DEFAULT 'http',
    host       TEXT NOT NULL,
    port       INTEGER NOT NULL,
    username   TEXT,
    password   TEXT,
    last_check TEXT,              -- JSON ProxyCheck
    created_at TEXT NOT NULL
);

-- Profiles. `machine` is the pinned fingerprint (JSON, engine-specific, frozen at creation);
-- `settings` is the user-editable ProfileSettings (JSON).
CREATE TABLE profiles (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    os            TEXT NOT NULL,
    notes         TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    last_activity TEXT,
    proxy_id      INTEGER REFERENCES proxies(id) ON DELETE SET NULL,
    machine       TEXT NOT NULL,
    settings      TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX profiles_proxy ON profiles(proxy_id);

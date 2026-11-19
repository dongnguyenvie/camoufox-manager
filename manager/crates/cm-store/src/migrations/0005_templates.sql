-- 0005_templates: extension library, templates, and their extension / tag assignments.
CREATE TABLE extensions (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    name     TEXT NOT NULL,
    addon_id TEXT NOT NULL UNIQUE,   -- manifest browser_specific_settings.gecko.id
    version  TEXT,
    file     TEXT NOT NULL           -- file name inside <data_dir>/extensions/
);
CREATE TABLE templates (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    name     TEXT NOT NULL UNIQUE COLLATE NOCASE,
    os       TEXT NOT NULL,
    settings TEXT NOT NULL DEFAULT '{}',   -- JSON ProfileSettings
    proxy_id INTEGER REFERENCES proxies(id) ON DELETE SET NULL,
    group_id INTEGER REFERENCES groups(id) ON DELETE SET NULL
);
CREATE TABLE template_extensions (
    template_id  INTEGER NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    extension_id INTEGER NOT NULL REFERENCES extensions(id) ON DELETE CASCADE,
    PRIMARY KEY (template_id, extension_id)
);
CREATE TABLE template_tags (
    template_id INTEGER NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (template_id, tag_id)
);
CREATE TABLE profile_extensions (
    profile_id   TEXT    NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    extension_id INTEGER NOT NULL REFERENCES extensions(id) ON DELETE CASCADE,
    PRIMARY KEY (profile_id, extension_id)
);
CREATE INDEX profile_extensions_ext ON profile_extensions(extension_id);

-- 0004_org: tags (many per profile), groups (one per profile), saved views.
CREATE TABLE tags (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE,
    color TEXT
);
CREATE TABLE profile_tags (
    profile_id TEXT    NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (profile_id, tag_id)
);
CREATE INDEX profile_tags_tag ON profile_tags(tag_id);
CREATE TABLE groups (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    description TEXT
);
-- one group per profile: profile_id is the primary key
CREATE TABLE profile_groups (
    profile_id TEXT PRIMARY KEY REFERENCES profiles(id) ON DELETE CASCADE,
    group_id   INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE
);
CREATE INDEX profile_groups_group ON profile_groups(group_id);
CREATE TABLE saved_views (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE,
    query TEXT NOT NULL
);

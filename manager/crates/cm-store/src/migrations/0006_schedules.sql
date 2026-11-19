-- 0006_schedules — phase 10 (Scheduling): schedules and their durable run history.
-- target_kind: profile | tag | group (target_id is the profile id or the tag/group id as text).
-- action_json / recurrence_json / precheck_json hold the cm-core enums as JSON.
CREATE TABLE schedules (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    target_kind     TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    action_json     TEXT NOT NULL,
    recurrence_json TEXT NOT NULL,
    timezone        TEXT NOT NULL,
    enabled         INTEGER NOT NULL DEFAULT 1,
    grace_minutes   INTEGER NOT NULL DEFAULT 10,
    precheck_json   TEXT NOT NULL,
    max_concurrent  INTEGER NOT NULL DEFAULT 1,
    next_run_at     INTEGER,
    anchor          INTEGER NOT NULL DEFAULT 0
);

-- Not a foreign key on profile_id: history outlives a deleted profile.
CREATE TABLE schedule_runs (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    schedule_id   INTEGER NOT NULL REFERENCES schedules(id) ON DELETE CASCADE,
    profile_id    TEXT,
    scheduled_for INTEGER NOT NULL,
    started_at    INTEGER NOT NULL,
    finished_at   INTEGER,
    outcome       TEXT NOT NULL,
    message       TEXT,
    trigger       TEXT NOT NULL DEFAULT 'schedule'
);
CREATE INDEX schedule_runs_by_schedule ON schedule_runs(schedule_id, id DESC);

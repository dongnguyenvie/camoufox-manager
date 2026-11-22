-- 0012_trash — CM-3.1 (Trash): soft delete. A trashed profile keeps its row and its data on disk
-- until it is restored or purged; NULL = not trashed (ISO 8601 UTC when it was trashed).
ALTER TABLE profiles ADD COLUMN deleted_at TEXT;
CREATE INDEX idx_profiles_deleted_at ON profiles(deleted_at);

CREATE TABLE sync_conflicts (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK(entity_type IN ('memory')),
    entity_id TEXT NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    local_value TEXT NOT NULL,
    remote_value TEXT NOT NULL,
    remote_device_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unresolved' CHECK(status IN ('unresolved', 'keep_local', 'use_remote')),
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE UNIQUE INDEX idx_unresolved_sync_conflict
    ON sync_conflicts(entity_type, entity_id, remote_device_id)
    WHERE status = 'unresolved';

CREATE INDEX idx_sync_conflicts_status_created
    ON sync_conflicts(status, created_at DESC);

PRAGMA user_version = 10;

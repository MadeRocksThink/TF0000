ALTER TABLE memory_sources ADD COLUMN source_type TEXT;
ALTER TABLE memory_sources ADD COLUMN source_id TEXT;
ALTER TABLE memory_sources ADD COLUMN source_hash TEXT;
ALTER TABLE memory_sources ADD COLUMN source_label TEXT;
ALTER TABLE memory_sources ADD COLUMN source_excerpt TEXT;

CREATE UNIQUE INDEX idx_memory_sources_identity
    ON memory_sources(memory_version_id, source_type, source_id)
    WHERE source_type IS NOT NULL AND source_id IS NOT NULL;

ALTER TABLE branches ADD COLUMN memory_id TEXT REFERENCES memories(id) ON DELETE CASCADE;
ALTER TABLE branches ADD COLUMN base_version_id TEXT REFERENCES memory_versions(id) ON DELETE SET NULL;
ALTER TABLE branches ADD COLUMN updated_at TEXT;

CREATE TABLE branch_versions (
    id TEXT PRIMARY KEY,
    branch_id TEXT NOT NULL REFERENCES branches(id) ON DELETE CASCADE,
    content TEXT NOT NULL CHECK(length(trim(content)) > 0),
    change_type TEXT NOT NULL CHECK(change_type IN ('create', 'add', 'merge', 'replace', 'restore')),
    created_at TEXT NOT NULL,
    supersedes_version_id TEXT REFERENCES branch_versions(id) ON DELETE SET NULL
);

CREATE INDEX idx_branch_versions_branch_created
    ON branch_versions(branch_id, created_at);

CREATE UNIQUE INDEX idx_active_branch_name
    ON branches(memory_id, name)
    WHERE memory_id IS NOT NULL AND status = 'active';

CREATE TABLE memory_conflicts (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    conflicting_memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    conflict_key TEXT NOT NULL,
    current_value TEXT NOT NULL,
    conflicting_value TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unresolved' CHECK(status IN ('unresolved', 'resolved', 'ignored')),
    created_at TEXT NOT NULL,
    resolved_at TEXT,
    CHECK(memory_id <> conflicting_memory_id)
);

CREATE UNIQUE INDEX idx_unresolved_memory_conflict
    ON memory_conflicts(memory_id, conflicting_memory_id, conflict_key)
    WHERE status = 'unresolved';

CREATE INDEX idx_memory_conflicts_status_created
    ON memory_conflicts(status, created_at);

PRAGMA user_version = 3;

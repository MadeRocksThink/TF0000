CREATE TABLE import_runs (
    id TEXT PRIMARY KEY,
    source_label TEXT NOT NULL,
    source_format TEXT NOT NULL CHECK(source_format IN ('json', 'markdown', 'text')),
    source_hash TEXT NOT NULL,
    imported_count INTEGER NOT NULL CHECK(imported_count >= 0),
    skipped_count INTEGER NOT NULL CHECK(skipped_count >= 0),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_import_runs_created ON import_runs(created_at DESC);

CREATE TABLE import_items (
    import_id TEXT NOT NULL REFERENCES import_runs(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK(entity_type IN ('memory')),
    entity_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    PRIMARY KEY(import_id, content_hash),
    UNIQUE(content_hash)
);

CREATE TABLE backup_settings (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0, 1)),
    interval_hours INTEGER NOT NULL DEFAULT 24 CHECK(interval_hours BETWEEN 1 AND 8760),
    directory TEXT NOT NULL DEFAULT '',
    last_backup_at TEXT,
    updated_at TEXT NOT NULL
);

INSERT INTO backup_settings
    (singleton, enabled, interval_hours, directory, updated_at)
VALUES (1, 0, 24, '', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

PRAGMA user_version = 6;

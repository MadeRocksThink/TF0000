CREATE TABLE smart_settings (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    mode TEXT NOT NULL DEFAULT 'off' CHECK(mode IN ('off', 'local', 'provider')),
    provider_name TEXT NOT NULL DEFAULT '',
    provider_endpoint TEXT NOT NULL DEFAULT '',
    provider_model TEXT NOT NULL DEFAULT '',
    recommendation_mode TEXT NOT NULL DEFAULT 'off'
        CHECK(recommendation_mode IN ('off', 'ask', 'automatic')),
    updated_at TEXT NOT NULL
);

INSERT INTO smart_settings
    (singleton, mode, provider_name, provider_endpoint, provider_model,
     recommendation_mode, updated_at)
VALUES (1, 'off', '', '', '', 'off', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

CREATE TABLE semantic_documents (
    entity_type TEXT NOT NULL CHECK(entity_type IN ('memory', 'message', 'fragment')),
    entity_id TEXT NOT NULL,
    parent_id TEXT,
    project_id TEXT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    provider TEXT,
    memory_type TEXT,
    authority TEXT,
    status TEXT,
    source_url TEXT,
    created_at TEXT NOT NULL,
    is_current INTEGER NOT NULL CHECK(is_current IN (0, 1)),
    content_hash TEXT NOT NULL,
    embedding TEXT NOT NULL,
    PRIMARY KEY(entity_type, entity_id)
);

CREATE INDEX idx_semantic_documents_scope
    ON semantic_documents(project_id, entity_type, created_at DESC);
CREATE INDEX idx_semantic_documents_hash
    ON semantic_documents(content_hash);

CREATE TABLE generated_artifacts (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL CHECK(source_type IN ('memory', 'conversation', 'conflict')),
    source_id TEXT NOT NULL,
    artifact_type TEXT NOT NULL CHECK(artifact_type IN ('summary', 'conflict_explanation')),
    content TEXT NOT NULL,
    model_mode TEXT NOT NULL CHECK(model_mode IN ('local', 'provider')),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_generated_artifacts_source
    ON generated_artifacts(source_type, source_id, created_at DESC);

CREATE TABLE smart_candidates (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('memory', 'conversation')),
    source_id TEXT NOT NULL,
    memory_type TEXT NOT NULL
        CHECK(memory_type IN ('decision', 'requirement', 'suggestion')),
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    confidence REAL NOT NULL CHECK(confidence BETWEEN 0 AND 1),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending', 'accepted', 'dismissed')),
    created_at TEXT NOT NULL,
    reviewed_at TEXT
);

CREATE INDEX idx_smart_candidates_status
    ON smart_candidates(status, created_at DESC);

PRAGMA user_version = 7;

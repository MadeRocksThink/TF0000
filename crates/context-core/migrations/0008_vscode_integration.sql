CREATE TABLE workspace_mappings (
    workspace_uri TEXT PRIMARY KEY,
    repository_root TEXT NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_workspace_mappings_project ON workspace_mappings(project_id);

CREATE TABLE code_references (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    workspace_uri TEXT NOT NULL,
    repository_root TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    language TEXT NOT NULL,
    start_line INTEGER CHECK(start_line IS NULL OR start_line >= 1),
    end_line INTEGER CHECK(end_line IS NULL OR end_line >= start_line),
    content TEXT NOT NULL CHECK(length(trim(content)) > 0),
    content_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(project_id, relative_path, start_line, end_line, content_hash)
);

CREATE INDEX idx_code_references_project_created
    ON code_references(project_id, created_at DESC);

CREATE VIRTUAL TABLE code_references_fts USING fts5(
    reference_id UNINDEXED,
    project_id UNINDEXED,
    relative_path,
    language UNINDEXED,
    content,
    created_at UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER code_references_fts_insert AFTER INSERT ON code_references BEGIN
    INSERT INTO code_references_fts
    VALUES (NEW.id, NEW.project_id, NEW.relative_path, NEW.language, NEW.content, NEW.created_at);
END;

CREATE TRIGGER code_references_fts_update AFTER UPDATE ON code_references BEGIN
    DELETE FROM code_references_fts WHERE reference_id = OLD.id;
    INSERT INTO code_references_fts
    VALUES (NEW.id, NEW.project_id, NEW.relative_path, NEW.language, NEW.content, NEW.created_at);
END;

CREATE TRIGGER code_references_fts_delete AFTER DELETE ON code_references BEGIN
    DELETE FROM code_references_fts WHERE reference_id = OLD.id;
END;

CREATE TABLE agent_write_candidates (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    requested_by TEXT NOT NULL,
    memory_type TEXT NOT NULL
        CHECK(memory_type IN ('decision', 'requirement', 'fact', 'preference', 'suggestion',
                              'idea', 'rejected_idea', 'task', 'bug', 'question', 'reference',
                              'summary')),
    title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 160),
    content TEXT NOT NULL CHECK(length(trim(content)) > 0),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending', 'accepted', 'rejected')),
    created_at TEXT NOT NULL,
    reviewed_at TEXT,
    memory_id TEXT REFERENCES memories(id) ON DELETE SET NULL
);

CREATE INDEX idx_agent_write_candidates_status
    ON agent_write_candidates(status, created_at DESC);

PRAGMA user_version = 8;

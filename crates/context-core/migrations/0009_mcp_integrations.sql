CREATE TABLE mcp_clients (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL CHECK(length(trim(display_name)) BETWEEN 1 AND 160),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE mcp_audit_log (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES mcp_clients(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    access_type TEXT NOT NULL CHECK(access_type IN ('read', 'candidate_write')),
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    entity_type TEXT,
    entity_id TEXT,
    outcome TEXT NOT NULL CHECK(outcome IN ('allowed', 'denied', 'error')),
    occurred_at TEXT NOT NULL
);

CREATE INDEX idx_mcp_audit_client_time
    ON mcp_audit_log(client_id, occurred_at DESC);

CREATE INDEX idx_mcp_audit_project_time
    ON mcp_audit_log(project_id, occurred_at DESC);

PRAGMA user_version = 9;

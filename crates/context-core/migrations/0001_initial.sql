CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 160),
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    archived_at TEXT
);

CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    provider TEXT NOT NULL,
    external_ref TEXT,
    title TEXT NOT NULL,
    url TEXT,
    created_at TEXT,
    captured_at TEXT NOT NULL,
    UNIQUE(provider, external_ref)
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool', 'unknown')),
    speaker TEXT,
    body TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    sent_at TEXT,
    source_hash TEXT NOT NULL,
    UNIQUE(conversation_id, ordinal),
    UNIQUE(conversation_id, source_hash)
);

CREATE TABLE fragments (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    start_offset INTEGER CHECK(start_offset IS NULL OR start_offset >= 0),
    end_offset INTEGER CHECK(end_offset IS NULL OR end_offset >= start_offset),
    selected_text TEXT NOT NULL CHECK(length(selected_text) > 0),
    source_hash TEXT NOT NULL,
    UNIQUE(message_id, source_hash)
);

CREATE TABLE memory_spaces (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    parent_id TEXT REFERENCES memory_spaces(id) ON DELETE SET NULL,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 160),
    description TEXT NOT NULL DEFAULT '',
    default_scope TEXT NOT NULL DEFAULT 'project' CHECK(default_scope IN ('global', 'project', 'conversation', 'task')),
    created_at TEXT NOT NULL,
    archived_at TEXT,
    UNIQUE(project_id, parent_id, name)
);

CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    type TEXT NOT NULL CHECK(type IN ('decision', 'requirement', 'fact', 'preference', 'suggestion', 'idea', 'rejected_idea', 'task', 'bug', 'question', 'reference', 'summary')),
    authority TEXT NOT NULL CHECK(authority IN ('user_confirmed', 'external_fact', 'ai_suggestion', 'inferred')),
    status TEXT NOT NULL CHECK(status IN ('active', 'draft', 'superseded', 'rejected', 'archived')),
    title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 160),
    current_version_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(current_version_id) REFERENCES memory_versions(id) DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE memory_versions (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    content TEXT NOT NULL CHECK(length(trim(content)) > 0),
    change_type TEXT NOT NULL CHECK(change_type IN ('create', 'add', 'merge', 'replace', 'supersede', 'restore')),
    created_at TEXT NOT NULL,
    supersedes_version_id TEXT REFERENCES memory_versions(id) ON DELETE SET NULL
);

CREATE TABLE memory_sources (
    memory_version_id TEXT NOT NULL REFERENCES memory_versions(id) ON DELETE CASCADE,
    conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
    message_id TEXT REFERENCES messages(id) ON DELETE SET NULL,
    fragment_id TEXT REFERENCES fragments(id) ON DELETE SET NULL,
    file_ref TEXT,
    source_role TEXT NOT NULL DEFAULT 'supporting',
    CHECK(conversation_id IS NOT NULL OR message_id IS NOT NULL OR fragment_id IS NOT NULL OR file_ref IS NOT NULL),
    UNIQUE(memory_version_id, conversation_id, message_id, fragment_id, file_ref)
);

CREATE TABLE memory_space_links (
    memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    memory_space_id TEXT NOT NULL REFERENCES memory_spaces(id) ON DELETE CASCADE,
    PRIMARY KEY(memory_id, memory_space_id)
);

CREATE TABLE chat_context_bindings (
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    context_target_type TEXT NOT NULL CHECK(context_target_type IN ('memory', 'memory_space', 'context_pack')),
    context_target_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    lifetime_mode TEXT NOT NULL CHECK(lifetime_mode IN ('one_prompt', 'n_prompts', 'session', 'conversation', 'manual')),
    PRIMARY KEY(conversation_id, context_target_type, context_target_id)
);

CREATE TABLE temporary_attachments (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL CHECK(target_type IN ('memory', 'memory_space', 'context_pack')),
    target_id TEXT NOT NULL,
    lifetime_mode TEXT NOT NULL CHECK(lifetime_mode IN ('one_prompt', 'n_prompts', 'session', 'conversation', 'manual')),
    remaining_prompts INTEGER CHECK(remaining_prompts IS NULL OR remaining_prompts > 0),
    expires_at TEXT,
    created_at TEXT NOT NULL,
    CHECK(
        (lifetime_mode = 'one_prompt' AND remaining_prompts = 1) OR
        (lifetime_mode = 'n_prompts' AND remaining_prompts > 0) OR
        (lifetime_mode IN ('session', 'conversation', 'manual') AND remaining_prompts IS NULL)
    )
);

CREATE TABLE context_packs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 160),
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    current_version INTEGER NOT NULL DEFAULT 1 CHECK(current_version > 0),
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(project_id, name)
);

CREATE TABLE context_pack_items (
    pack_id TEXT NOT NULL REFERENCES context_packs(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL CHECK(target_type IN ('memory', 'fragment', 'file', 'requirement', 'question')),
    target_id TEXT NOT NULL,
    ordering INTEGER NOT NULL CHECK(ordering >= 0),
    inclusion_mode TEXT NOT NULL DEFAULT 'full' CHECK(inclusion_mode IN ('full', 'summary', 'reference')),
    PRIMARY KEY(pack_id, target_type, target_id),
    UNIQUE(pack_id, ordering)
);

CREATE TABLE branches (
    id TEXT PRIMARY KEY,
    base_context_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'promoted', 'merged', 'abandoned')),
    created_at TEXT NOT NULL
);

CREATE TABLE integration_permissions (
    integration_id TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK(scope_type IN ('global', 'project', 'memory_space', 'memory')),
    scope_id TEXT NOT NULL DEFAULT '*',
    read_allowed INTEGER NOT NULL DEFAULT 0 CHECK(read_allowed IN (0, 1)),
    write_allowed INTEGER NOT NULL DEFAULT 0 CHECK(write_allowed IN (0, 1)),
    PRIMARY KEY(integration_id, scope_type, scope_id)
);

CREATE TABLE adapter_state (
    adapter_id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    version TEXT NOT NULL,
    last_health_check TEXT,
    status TEXT NOT NULL DEFAULT 'unknown' CHECK(status IN ('unknown', 'healthy', 'degraded', 'unavailable')),
    diagnostics TEXT
);

CREATE INDEX idx_conversations_project ON conversations(project_id);
CREATE INDEX idx_messages_conversation ON messages(conversation_id, ordinal);
CREATE INDEX idx_fragments_message ON fragments(message_id);
CREATE INDEX idx_spaces_project ON memory_spaces(project_id);
CREATE INDEX idx_memories_project_status ON memories(project_id, status);
CREATE INDEX idx_versions_memory_created ON memory_versions(memory_id, created_at);
CREATE INDEX idx_temp_conversation ON temporary_attachments(conversation_id);
CREATE INDEX idx_packs_project ON context_packs(project_id);

PRAGMA user_version = 1;


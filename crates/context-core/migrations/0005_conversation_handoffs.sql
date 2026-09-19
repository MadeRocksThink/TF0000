CREATE TABLE conversation_handoffs (
    id TEXT PRIMARY KEY,
    source_conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    destination_provider TEXT NOT NULL CHECK(destination_provider IN ('chatgpt', 'claude', 'gemini')),
    destination_conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
    mode TEXT NOT NULL CHECK(mode IN ('full', 'minimal', 'custom')),
    selection_json TEXT NOT NULL,
    handoff_text TEXT NOT NULL CHECK(length(handoff_text) > 0),
    content_hash TEXT NOT NULL,
    item_count INTEGER NOT NULL CHECK(item_count >= 0),
    character_count INTEGER NOT NULL CHECK(character_count > 0),
    estimated_tokens INTEGER NOT NULL CHECK(estimated_tokens > 0),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_handoffs_source_created
    ON conversation_handoffs(source_conversation_id, created_at DESC);

CREATE INDEX idx_handoffs_destination_created
    ON conversation_handoffs(destination_provider, created_at DESC);

PRAGMA user_version = 5;

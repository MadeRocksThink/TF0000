ALTER TABLE messages ADD COLUMN external_ref TEXT;

CREATE UNIQUE INDEX idx_messages_external_ref
    ON messages(conversation_id, external_ref)
    WHERE external_ref IS NOT NULL;

PRAGMA user_version = 2;

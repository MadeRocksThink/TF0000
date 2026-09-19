CREATE VIRTUAL TABLE messages_fts USING fts5(
    message_id UNINDEXED,
    conversation_id UNINDEXED,
    project_id UNINDEXED,
    provider UNINDEXED,
    title,
    role UNINDEXED,
    body,
    source_url UNINDEXED,
    captured_at UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE fragments_fts USING fts5(
    fragment_id UNINDEXED,
    message_id UNINDEXED,
    conversation_id UNINDEXED,
    project_id UNINDEXED,
    provider UNINDEXED,
    title,
    role UNINDEXED,
    selected_text,
    source_url UNINDEXED,
    captured_at UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE memory_versions_fts USING fts5(
    memory_version_id UNINDEXED,
    memory_id UNINDEXED,
    project_id UNINDEXED,
    title,
    content,
    memory_type UNINDEXED,
    authority UNINDEXED,
    status UNINDEXED,
    created_at UNINDEXED,
    is_current UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE context_packs_fts USING fts5(
    pack_id UNINDEXED,
    project_id UNINDEXED,
    name,
    description,
    content,
    updated_at UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

INSERT INTO messages_fts
SELECT m.id, m.conversation_id, c.project_id, c.provider, c.title, m.role, m.body, c.url, c.captured_at
FROM messages m JOIN conversations c ON c.id = m.conversation_id;

INSERT INTO fragments_fts
SELECT f.id, m.id, m.conversation_id, c.project_id, c.provider, c.title, m.role,
       f.selected_text, c.url, c.captured_at
FROM fragments f
JOIN messages m ON m.id = f.message_id
JOIN conversations c ON c.id = m.conversation_id;

INSERT INTO memory_versions_fts
SELECT v.id, v.memory_id, m.project_id, m.title, v.content, m.type, m.authority, m.status,
       v.created_at, CASE WHEN m.current_version_id = v.id THEN '1' ELSE '0' END
FROM memory_versions v JOIN memories m ON m.id = v.memory_id;

INSERT INTO context_packs_fts
SELECT p.id, p.project_id, p.name, p.description,
       COALESCE((
           SELECT group_concat(
               CASE
                   WHEN i.target_type = 'memory' THEN mv.content
                   WHEN i.target_type = 'fragment' THEN f.selected_text
                   ELSE ''
               END,
               char(10) || char(10)
           )
           FROM context_pack_items i
           LEFT JOIN memories m ON i.target_type = 'memory' AND m.id = i.target_id
           LEFT JOIN memory_versions mv ON mv.id = m.current_version_id
           LEFT JOIN fragments f ON i.target_type = 'fragment' AND f.id = i.target_id
           WHERE i.pack_id = p.id
       ), ''),
       p.updated_at
FROM context_packs p;

CREATE TRIGGER messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts
    SELECT NEW.id, NEW.conversation_id, c.project_id, c.provider, c.title, NEW.role, NEW.body,
           c.url, c.captured_at
    FROM conversations c WHERE c.id = NEW.conversation_id;
END;

CREATE TRIGGER messages_fts_update AFTER UPDATE ON messages BEGIN
    DELETE FROM messages_fts WHERE message_id = OLD.id;
    INSERT INTO messages_fts
    SELECT NEW.id, NEW.conversation_id, c.project_id, c.provider, c.title, NEW.role, NEW.body,
           c.url, c.captured_at
    FROM conversations c WHERE c.id = NEW.conversation_id;
END;

CREATE TRIGGER messages_fts_delete AFTER DELETE ON messages BEGIN
    DELETE FROM messages_fts WHERE message_id = OLD.id;
END;

CREATE TRIGGER fragments_fts_insert AFTER INSERT ON fragments BEGIN
    INSERT INTO fragments_fts
    SELECT NEW.id, m.id, m.conversation_id, c.project_id, c.provider, c.title, m.role,
           NEW.selected_text, c.url, c.captured_at
    FROM messages m JOIN conversations c ON c.id = m.conversation_id
    WHERE m.id = NEW.message_id;
END;

CREATE TRIGGER fragments_fts_update AFTER UPDATE ON fragments BEGIN
    DELETE FROM fragments_fts WHERE fragment_id = OLD.id;
    INSERT INTO fragments_fts
    SELECT NEW.id, m.id, m.conversation_id, c.project_id, c.provider, c.title, m.role,
           NEW.selected_text, c.url, c.captured_at
    FROM messages m JOIN conversations c ON c.id = m.conversation_id
    WHERE m.id = NEW.message_id;
END;

CREATE TRIGGER fragments_fts_delete AFTER DELETE ON fragments BEGIN
    DELETE FROM fragments_fts WHERE fragment_id = OLD.id;
END;

CREATE TRIGGER conversations_fts_update AFTER UPDATE ON conversations BEGIN
    DELETE FROM messages_fts WHERE conversation_id = NEW.id;
    INSERT INTO messages_fts
    SELECT m.id, m.conversation_id, NEW.project_id, NEW.provider, NEW.title, m.role, m.body,
           NEW.url, NEW.captured_at
    FROM messages m WHERE m.conversation_id = NEW.id;
    DELETE FROM fragments_fts WHERE conversation_id = NEW.id;
    INSERT INTO fragments_fts
    SELECT f.id, m.id, m.conversation_id, NEW.project_id, NEW.provider, NEW.title, m.role,
           f.selected_text, NEW.url, NEW.captured_at
    FROM fragments f JOIN messages m ON m.id = f.message_id
    WHERE m.conversation_id = NEW.id;
END;

CREATE TRIGGER memory_versions_fts_insert AFTER INSERT ON memory_versions BEGIN
    INSERT INTO memory_versions_fts
    SELECT NEW.id, NEW.memory_id, m.project_id, m.title, NEW.content, m.type, m.authority,
           m.status, NEW.created_at, CASE WHEN m.current_version_id = NEW.id THEN '1' ELSE '0' END
    FROM memories m WHERE m.id = NEW.memory_id;
END;

CREATE TRIGGER memory_versions_fts_update AFTER UPDATE ON memory_versions BEGIN
    DELETE FROM memory_versions_fts WHERE memory_version_id = OLD.id;
    INSERT INTO memory_versions_fts
    SELECT NEW.id, NEW.memory_id, m.project_id, m.title, NEW.content, m.type, m.authority,
           m.status, NEW.created_at, CASE WHEN m.current_version_id = NEW.id THEN '1' ELSE '0' END
    FROM memories m WHERE m.id = NEW.memory_id;
END;

CREATE TRIGGER memory_versions_fts_delete AFTER DELETE ON memory_versions BEGIN
    DELETE FROM memory_versions_fts WHERE memory_version_id = OLD.id;
END;

CREATE TRIGGER memories_fts_update AFTER UPDATE ON memories BEGIN
    DELETE FROM memory_versions_fts WHERE memory_id = NEW.id;
    INSERT INTO memory_versions_fts
    SELECT v.id, v.memory_id, NEW.project_id, NEW.title, v.content, NEW.type, NEW.authority,
           NEW.status, v.created_at, CASE WHEN NEW.current_version_id = v.id THEN '1' ELSE '0' END
    FROM memory_versions v WHERE v.memory_id = NEW.id;

    DELETE FROM context_packs_fts
    WHERE pack_id IN (
        SELECT pack_id FROM context_pack_items WHERE target_type = 'memory' AND target_id = NEW.id
    );
    INSERT INTO context_packs_fts
    SELECT p.id, p.project_id, p.name, p.description,
           COALESCE((
               SELECT group_concat(CASE WHEN i.target_type = 'memory' THEN mv.content WHEN i.target_type = 'fragment' THEN f.selected_text ELSE '' END, char(10) || char(10))
               FROM context_pack_items i
               LEFT JOIN memories m ON i.target_type = 'memory' AND m.id = i.target_id
               LEFT JOIN memory_versions mv ON mv.id = m.current_version_id
               LEFT JOIN fragments f ON i.target_type = 'fragment' AND f.id = i.target_id
               WHERE i.pack_id = p.id
           ), ''), p.updated_at
    FROM context_packs p
    WHERE p.id IN (
        SELECT pack_id FROM context_pack_items WHERE target_type = 'memory' AND target_id = NEW.id
    );
END;

CREATE TRIGGER context_packs_fts_insert AFTER INSERT ON context_packs BEGIN
    INSERT INTO context_packs_fts VALUES (NEW.id, NEW.project_id, NEW.name, NEW.description, '', NEW.updated_at);
END;

CREATE TRIGGER context_packs_fts_update AFTER UPDATE ON context_packs BEGIN
    DELETE FROM context_packs_fts WHERE pack_id = OLD.id;
    INSERT INTO context_packs_fts
    SELECT NEW.id, NEW.project_id, NEW.name, NEW.description,
           COALESCE((
               SELECT group_concat(CASE WHEN i.target_type = 'memory' THEN mv.content WHEN i.target_type = 'fragment' THEN f.selected_text ELSE '' END, char(10) || char(10))
               FROM context_pack_items i
               LEFT JOIN memories m ON i.target_type = 'memory' AND m.id = i.target_id
               LEFT JOIN memory_versions mv ON mv.id = m.current_version_id
               LEFT JOIN fragments f ON i.target_type = 'fragment' AND f.id = i.target_id
               WHERE i.pack_id = NEW.id
           ), ''), NEW.updated_at;
END;

CREATE TRIGGER context_packs_fts_delete AFTER DELETE ON context_packs BEGIN
    DELETE FROM context_packs_fts WHERE pack_id = OLD.id;
END;

CREATE TRIGGER context_pack_items_fts_insert AFTER INSERT ON context_pack_items BEGIN
    DELETE FROM context_packs_fts WHERE pack_id = NEW.pack_id;
    INSERT INTO context_packs_fts
    SELECT p.id, p.project_id, p.name, p.description,
           COALESCE((
               SELECT group_concat(CASE WHEN i.target_type = 'memory' THEN mv.content WHEN i.target_type = 'fragment' THEN f.selected_text ELSE '' END, char(10) || char(10))
               FROM context_pack_items i
               LEFT JOIN memories m ON i.target_type = 'memory' AND m.id = i.target_id
               LEFT JOIN memory_versions mv ON mv.id = m.current_version_id
               LEFT JOIN fragments f ON i.target_type = 'fragment' AND f.id = i.target_id
               WHERE i.pack_id = p.id
           ), ''), p.updated_at
    FROM context_packs p WHERE p.id = NEW.pack_id;
END;

CREATE TRIGGER context_pack_items_fts_update AFTER UPDATE ON context_pack_items BEGIN
    DELETE FROM context_packs_fts WHERE pack_id IN (OLD.pack_id, NEW.pack_id);
    INSERT INTO context_packs_fts
    SELECT p.id, p.project_id, p.name, p.description,
           COALESCE((
               SELECT group_concat(CASE WHEN i.target_type = 'memory' THEN mv.content WHEN i.target_type = 'fragment' THEN f.selected_text ELSE '' END, char(10) || char(10))
               FROM context_pack_items i
               LEFT JOIN memories m ON i.target_type = 'memory' AND m.id = i.target_id
               LEFT JOIN memory_versions mv ON mv.id = m.current_version_id
               LEFT JOIN fragments f ON i.target_type = 'fragment' AND f.id = i.target_id
               WHERE i.pack_id = p.id
           ), ''), p.updated_at
    FROM context_packs p WHERE p.id IN (OLD.pack_id, NEW.pack_id);
END;

CREATE TRIGGER context_pack_items_fts_delete AFTER DELETE ON context_pack_items BEGIN
    DELETE FROM context_packs_fts WHERE pack_id = OLD.pack_id;
    INSERT INTO context_packs_fts
    SELECT p.id, p.project_id, p.name, p.description,
           COALESCE((
               SELECT group_concat(CASE WHEN i.target_type = 'memory' THEN mv.content WHEN i.target_type = 'fragment' THEN f.selected_text ELSE '' END, char(10) || char(10))
               FROM context_pack_items i
               LEFT JOIN memories m ON i.target_type = 'memory' AND m.id = i.target_id
               LEFT JOIN memory_versions mv ON mv.id = m.current_version_id
               LEFT JOIN fragments f ON i.target_type = 'fragment' AND f.id = i.target_id
               WHERE i.pack_id = p.id
           ), ''), p.updated_at
    FROM context_packs p WHERE p.id = OLD.pack_id;
END;

PRAGMA user_version = 4;

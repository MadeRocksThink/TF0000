use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, backup::Backup, params};
use uuid::Uuid;

use crate::{
    error::{CoreError, Result},
    models::{
        AgentWriteCandidate, AppendBranchVersionInput, AppendVersionInput, ApplyMemoryUpdateInput,
        ApplyPortableWorkspaceInput, AskMemoryInput, AskMemoryResponse, BackupSettings,
        CaptureConversationInput, CaptureResult, CapturedSource, CodeReference,
        ComposeContextInput, ComposedContext, ConflictCandidate, ConflictExplanation,
        ConflictExplanationSource, ContextBinding, ContextPack, ContextPackDetail, ContextPackItem,
        ContextRecommendation, ContextRecommendationInput, ConversationContextState,
        ConversationHandoff, CreateAgentWriteCandidateInput, CreateContextPackInput,
        CreateMemoryBranchInput, CreateMemoryInput, CreateMemorySpaceInput, CreateProjectInput,
        CreateTemporaryAttachmentInput, DashboardSnapshot, DiagnosticReport, EmbeddingBenchmark,
        ExportBundle, ExportMemory, ExportPortableWorkspaceInput, ExtractCandidatesInput,
        GenerateSummaryInput, GeneratedArtifact, GetDecisionsInput, HandoffInput, HandoffItem,
        HandoffPreview, HealthReport, ImportCandidate, ImportInput, ImportPreview, ImportResult,
        MapWorkspaceInput, McpAuditEntry, McpClient, McpPermission, Memory, MemoryBranch,
        MemoryConflict, MemoryHistory, MemorySource, MemorySpace, MemorySpaceLink,
        MemoryUpdatePreview, MemoryUpdateResult, MemoryVersion, MergeSourceInput,
        PortableWorkspace, PreviewMemoryUpdateInput, Project, ProjectContext, ProjectContextInput,
        RecordMcpAuditInput, RegisterMcpClientInput, ResolveSyncConflictInput, ResolvedMergeSource,
        RestoreResult, ReviewAgentWriteCandidateInput, ReviewSmartCandidateInput,
        SaveCodeReferenceInput, SaveContextPackInput, ScheduledBackupResult, SearchChatsInput,
        SearchDecisionsInput, SearchInput, SearchMemoriesInput, SearchResponse, SearchResult,
        SecretWarning, SetMcpPermissionInput, SmartBenchmarkReport, SmartCandidate,
        SmartCandidateReview, SmartSettings, SyncApplyResult, SyncConflict, TemporaryAttachment,
        UpdateBackupSettingsInput, UpdateDiff, UpdateSmartSettingsInput, WorkspaceMapping,
    },
    validation::{choice, optional_text, optional_uuid, required_text, uuid},
};

const SCHEMA_VERSION: i64 = 10;
const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");
const BROWSER_CAPTURE_MIGRATION: &str = include_str!("../migrations/0002_browser_capture.sql");
const MERGE_VERSIONING_MIGRATION: &str = include_str!("../migrations/0003_merge_versioning.sql");
const LOCAL_SEARCH_MIGRATION: &str = include_str!("../migrations/0004_local_search.sql");
const CONVERSATION_HANDOFFS_MIGRATION: &str =
    include_str!("../migrations/0005_conversation_handoffs.sql");
const RELEASE_HARDENING_MIGRATION: &str = include_str!("../migrations/0006_release_hardening.sql");
const OPTIONAL_SMART_LAYER_MIGRATION: &str =
    include_str!("../migrations/0007_optional_smart_layer.sql");
const VSCODE_INTEGRATION_MIGRATION: &str =
    include_str!("../migrations/0008_vscode_integration.sql");
const MCP_INTEGRATIONS_MIGRATION: &str = include_str!("../migrations/0009_mcp_integrations.sql");
const ECOSYSTEM_SYNC_MIGRATION: &str = include_str!("../migrations/0010_ecosystem_sync.sql");
const LOCAL_EMBEDDING_MODEL: &str = "tf0000-mini-embed-v1";
const LOCAL_EMBEDDING_DIMENSIONS: usize = 128;
const MEMORY_TYPES: &[&str] = &[
    "decision",
    "requirement",
    "fact",
    "preference",
    "suggestion",
    "idea",
    "rejected_idea",
    "task",
    "bug",
    "question",
    "reference",
    "summary",
];
const AUTHORITIES: &[&str] = &[
    "user_confirmed",
    "external_fact",
    "ai_suggestion",
    "inferred",
];
const STATUSES: &[&str] = &["active", "draft", "superseded", "rejected", "archived"];
const SCOPES: &[&str] = &["global", "project", "conversation", "task"];
const CHANGE_TYPES: &[&str] = &["add", "merge", "replace", "supersede", "restore"];
const UPDATE_ACTIONS: &[&str] = &["add", "merge", "replace", "supersede"];
const MERGE_SOURCE_TYPES: &[&str] = &["conversation", "message", "fragment", "memory", "manual"];
const BRANCH_CHANGE_TYPES: &[&str] = &["add", "merge", "replace", "restore"];
const TARGET_TYPES: &[&str] = &["memory", "memory_space", "context_pack"];
const LIFETIMES: &[&str] = &[
    "one_prompt",
    "n_prompts",
    "session",
    "conversation",
    "manual",
];

#[derive(Debug, Clone)]
pub struct ContextStore {
    database_path: PathBuf,
}

impl ContextStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let database_path = path.as_ref().to_path_buf();
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let store = Self { database_path };
        let mut connection = store.connect()?;
        store.migrate(&mut connection)?;
        store.verify_integrity_with(&connection)?;
        Ok(store)
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    fn connect(&self) -> Result<Connection> {
        let connection = Connection::open(&self.database_path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;\nPRAGMA journal_mode = WAL;\nPRAGMA synchronous = NORMAL;",
        )?;
        Ok(connection)
    }

    fn migrate(&self, connection: &mut Connection) -> Result<()> {
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSchema {
                found: version,
                supported: SCHEMA_VERSION,
            });
        }
        if version == 0 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(INITIAL_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 1 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(BROWSER_CAPTURE_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 2 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MERGE_VERSIONING_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 3 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(LOCAL_SEARCH_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 4 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(CONVERSATION_HANDOFFS_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 5 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(RELEASE_HARDENING_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 6 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(OPTIONAL_SMART_LAYER_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 7 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(VSCODE_INTEGRATION_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 8 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(MCP_INTEGRATIONS_MIGRATION)?;
            transaction.commit()?;
        }
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 9 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(ECOSYSTEM_SYNC_MIGRATION)?;
            transaction.commit()?;
        }
        Ok(())
    }

    fn verify_integrity_with(&self, connection: &Connection) -> Result<()> {
        let result: String =
            connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if result != "ok" {
            return Err(CoreError::Integrity(result));
        }
        Ok(())
    }

    pub fn health(&self) -> Result<HealthReport> {
        let connection = self.connect()?;
        self.verify_integrity_with(&connection)?;
        let schema_version =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let sqlite_version =
            connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))?;
        Ok(HealthReport {
            status: "healthy".into(),
            schema_version,
            sqlite_version,
            database_path: self.database_path.display().to_string(),
        })
    }

    pub fn search(&self, input: SearchInput) -> Result<SearchResponse> {
        let search = validate_search_input(input)?;
        let connection = self.connect()?;
        ensure_optional_exists(
            &connection,
            "projects",
            search.project_id.as_deref(),
            "project",
        )?;
        let fetch_limit = (search.limit + search.offset).min(500) as i64;
        let mut results = Vec::new();

        if search.memory_type.is_none() && search.status.is_none() {
            search_messages(&connection, &search, fetch_limit, &mut results)?;
            search_fragments(&connection, &search, fetch_limit, &mut results)?;
        }
        if search.provider.is_none() {
            search_memory_versions(&connection, &search, fetch_limit, &mut results)?;
            if search.memory_type.is_none() && search.status.is_none() {
                search_context_packs(&connection, &search, fetch_limit, &mut results)?;
                search_code_references(&connection, &search, fetch_limit, &mut results)?;
            }
        }

        if read_smart_settings(&connection)?.mode != "off" {
            sync_semantic_index(&connection)?;
            let semantic_results = semantic_search(&connection, &search, fetch_limit as usize)?;
            for semantic in semantic_results {
                if let Some(existing) = results.iter_mut().find(|result| {
                    result.result_type == semantic.result_type && result.id == semantic.id
                }) {
                    existing.score = existing.score.max(semantic.score);
                } else {
                    results.push(semantic);
                }
            }
        }

        results.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| right.created_at.cmp(&left.created_at))
                .then_with(|| left.result_type.cmp(&right.result_type))
                .then_with(|| left.id.cmp(&right.id))
        });
        let results = results
            .into_iter()
            .skip(search.offset)
            .take(search.limit)
            .collect::<Vec<_>>();
        Ok(SearchResponse {
            query: search.query,
            result_count: results.len(),
            results,
        })
    }

    pub fn ask_memory(&self, input: AskMemoryInput) -> Result<AskMemoryResponse> {
        let query = required_text("query", &input.query, 500)?;
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let scope = choice(
            "scope",
            input.scope.as_deref().unwrap_or(if project_id.is_some() {
                "project"
            } else {
                "all"
            }),
            &["all", "global", "project"],
        )?;
        if scope == "project" && project_id.is_none() {
            return Err(CoreError::Validation {
                field: "projectId",
                message: "is required for project scope".into(),
            });
        }
        let provider = input
            .provider
            .as_deref()
            .map(|value| required_text("provider", value, 80))
            .transpose()?;
        let scope = Some(scope);
        let evidence = self
            .search(SearchInput {
                query: query.clone(),
                scope: scope.clone(),
                project_id: project_id.clone(),
                provider,
                date_from: None,
                date_to: None,
                memory_type: None,
                status: None,
                limit: Some(12),
                offset: None,
            })?
            .results;
        let current_decisions = self
            .search(SearchInput {
                query: query.clone(),
                scope,
                project_id,
                provider: None,
                date_from: None,
                date_to: None,
                memory_type: Some("decision".into()),
                status: Some("active".into()),
                limit: Some(6),
                offset: None,
            })?
            .results
            .into_iter()
            .filter(|result| result.is_current)
            .collect::<Vec<_>>();
        if evidence.is_empty() && current_decisions.is_empty() {
            return Ok(AskMemoryResponse {
                status: "not_recorded".into(),
                message:
                    "Not recorded in TF0000. No matching evidence or current decision was found."
                        .into(),
                evidence,
                current_decisions,
            });
        }
        Ok(AskMemoryResponse {
            status: "evidence".into(),
            message: format!(
                "Found {} direct evidence item(s) and {} matching current decision(s).",
                evidence.len(),
                current_decisions.len()
            ),
            evidence,
            current_decisions,
        })
    }

    pub fn capture_conversation(&self, input: CaptureConversationInput) -> Result<CaptureResult> {
        let provider = required_text("provider", &input.provider, 80)?;
        let title = required_text("title", &input.title, 500)?;
        let external_ref = input
            .external_ref
            .as_deref()
            .map(|value| required_text("externalRef", value, 500))
            .transpose()?;
        let url = input
            .url
            .as_deref()
            .map(|value| required_text("url", value, 4_000))
            .transpose()?;
        let captured_at = required_text("capturedAt", &input.captured_at, 80)?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let conversation_id = external_ref
            .as_deref()
            .map(|reference| {
                transaction
                    .query_row(
                        "SELECT id FROM conversations WHERE provider = ?1 AND external_ref = ?2",
                        params![provider, reference],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
            })
            .transpose()?
            .flatten()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        transaction.execute(
            "INSERT INTO conversations (id, provider, external_ref, title, url, captured_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(provider, external_ref) DO UPDATE SET title = excluded.title, url = excluded.url, captured_at = excluded.captured_at",
            params![conversation_id, provider, external_ref, title, url, captured_at],
        )?;

        let mut message_ids = Vec::with_capacity(input.messages.len());
        for message in input.messages {
            let role = choice(
                "role",
                &message.role,
                &["user", "assistant", "system", "tool", "unknown"],
            )?;
            let body = required_text("body", &message.body, 1_000_000)?;
            let source_hash = required_text("sourceHash", &message.source_hash, 256)?;
            if message.ordinal < 0 {
                return Err(CoreError::Validation {
                    field: "ordinal",
                    message: "must be zero or greater".into(),
                });
            }
            let id = transaction
                .query_row(
                    "SELECT id FROM messages WHERE conversation_id = ?1 AND ordinal = ?2",
                    params![conversation_id, message.ordinal],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            transaction.execute(
                "INSERT INTO messages (id, conversation_id, external_ref, role, speaker, body, ordinal, sent_at, source_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) ON CONFLICT(conversation_id, ordinal) DO UPDATE SET external_ref = excluded.external_ref, role = excluded.role, speaker = excluded.speaker, body = excluded.body, sent_at = excluded.sent_at, source_hash = excluded.source_hash",
                params![id, conversation_id, message.external_ref, role, message.speaker, body, message.ordinal, message.sent_at, source_hash],
            )?;
            message_ids.push((message.external_ref, source_hash, id));
        }

        let mut saved_fragments = 0;
        for fragment in input.fragments {
            let selected_text = required_text("selectedText", &fragment.selected_text, 1_000_000)?;
            let source_hash = required_text("sourceHash", &fragment.source_hash, 256)?;
            let message_id = message_ids
                .iter()
                .find(|(external, hash, _)| {
                    fragment
                        .message_external_ref
                        .as_ref()
                        .is_some_and(|value| external.as_ref() == Some(value))
                        || fragment.message_source_hash.as_ref() == Some(hash)
                })
                .map(|(_, _, id)| id.clone())
                .ok_or_else(|| CoreError::Validation {
                    field: "fragment",
                    message: "must reference a captured message".into(),
                })?;
            transaction.execute(
                "INSERT INTO fragments (id, message_id, start_offset, end_offset, selected_text, source_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(message_id, source_hash) DO UPDATE SET start_offset = excluded.start_offset, end_offset = excluded.end_offset, selected_text = excluded.selected_text",
                params![Uuid::new_v4().to_string(), message_id, fragment.start_offset, fragment.end_offset, selected_text, source_hash],
            )?;
            saved_fragments += 1;
        }
        let saved_messages = message_ids.len();
        transaction.commit()?;
        Ok(CaptureResult {
            conversation_id,
            saved_messages,
            saved_fragments,
        })
    }

    pub fn list_captured_sources(&self) -> Result<Vec<CapturedSource>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT c.id, 'conversation', c.provider, c.title, 'conversation',
                    group_concat('[' || upper(m.role) || '] ' || m.body, char(10) || char(10)),
                    c.url, c.captured_at, 0 AS source_rank, -1 AS source_order
             FROM conversations c
             JOIN messages m ON m.conversation_id = c.id
             GROUP BY c.id
             UNION ALL
             SELECT m.id, 'message', c.provider, c.title, m.role, m.body, c.url, c.captured_at, 1 AS source_rank, m.ordinal AS source_order
             FROM messages m
             JOIN conversations c ON c.id = m.conversation_id
             UNION ALL
             SELECT f.id, 'fragment', c.provider, c.title, m.role, f.selected_text, c.url, c.captured_at, 2 AS source_rank, m.ordinal AS source_order
             FROM fragments f
             JOIN messages m ON m.id = f.message_id
             JOIN conversations c ON c.id = m.conversation_id
             ORDER BY captured_at DESC, source_rank ASC, source_order ASC
             LIMIT 500",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(CapturedSource {
                id: row.get(0)?,
                source_type: row.get(1)?,
                provider: row.get(2)?,
                conversation_title: row.get(3)?,
                role: row.get(4)?,
                content: row.get(5)?,
                source_url: row.get(6)?,
                captured_at: row.get(7)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn compose_context(&self, input: ComposeContextInput) -> Result<ComposedContext> {
        let connection = self.connect()?;
        let mut memory_targets = input
            .memory_ids
            .iter()
            .map(|id| Ok((uuid("memoryId", id)?, "full".to_string())))
            .collect::<Result<Vec<_>>>()?;
        for id in &input.memory_space_ids {
            collect_target_memories(
                &connection,
                "memory_space",
                &uuid("memorySpaceId", id)?,
                &mut memory_targets,
            )?;
        }
        for id in &input.context_pack_ids {
            collect_target_memories(
                &connection,
                "context_pack",
                &uuid("contextPackId", id)?,
                &mut memory_targets,
            )?;
        }
        if let Some(conversation_id) = input.conversation_id.as_deref() {
            let conversation_id = uuid("conversationId", conversation_id)?;
            ensure_exists(
                &connection,
                "conversations",
                &conversation_id,
                "conversation",
            )?;
            for binding in list_context_bindings(&connection, &conversation_id)? {
                if binding.enabled {
                    collect_target_memories(
                        &connection,
                        &binding.target_type,
                        &binding.target_id,
                        &mut memory_targets,
                    )?;
                }
            }
            for attachment in list_temporary_attachments(&connection, &conversation_id)? {
                collect_target_memories(
                    &connection,
                    &attachment.target_type,
                    &attachment.target_id,
                    &mut memory_targets,
                )?;
            }
        }

        let mut sections = Vec::new();
        let mut seen_memories = HashSet::new();
        for (id, inclusion_mode) in memory_targets {
            if !seen_memories.insert(id.clone()) {
                continue;
            }
            let memory = self.get_memory(&id)?;
            let content = match inclusion_mode.as_str() {
                "full" => memory.current_content.trim().to_string(),
                "summary" => deterministic_summary(&memory.current_content, 400),
                "reference" => {
                    "Reference only — open this memory in TF0000 for the complete content.".into()
                }
                _ => unreachable!("inclusion modes are validated before collection"),
            };
            sections.push(format!(
                "## {}. {}\nMode: {}\n\n{}",
                sections.len() + 1,
                memory.title,
                inclusion_mode,
                content
            ));
        }
        for reference in &input.sources {
            let id = uuid("sourceId", &reference.id)?;
            let source: Option<(String, String, String, String, Option<String>)> =
                match reference.source_type.as_str() {
                    "conversation" => connection
                        .query_row(
                            "SELECT c.provider, c.title, 'conversation', group_concat('[' || upper(m.role) || '] ' || m.body, char(10) || char(10)), c.url FROM conversations c JOIN messages m ON m.conversation_id = c.id WHERE c.id = ?1 GROUP BY c.id",
                            [&id],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                        )
                        .optional()?,
                    "fragment" => connection
                        .query_row(
                            "SELECT c.provider, c.title, m.role, f.selected_text, c.url FROM fragments f JOIN messages m ON m.id = f.message_id JOIN conversations c ON c.id = m.conversation_id WHERE f.id = ?1",
                            [&id],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                        )
                        .optional()?,
                    "message" => connection
                        .query_row(
                            "SELECT c.provider, c.title, m.role, m.body, c.url FROM messages m JOIN conversations c ON c.id = m.conversation_id WHERE m.id = ?1",
                            [&id],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                        )
                        .optional()?,
                    _ => {
                        return Err(CoreError::Validation {
                            field: "sourceType",
                            message: "must be conversation, fragment or message".into(),
                        });
                    }
                };
            let (provider, title, role, content, source_url) =
                source.ok_or_else(|| CoreError::NotFound {
                    entity: "captured source",
                    id,
                })?;
            let number = sections.len() + 1;
            let source_line = source_url
                .as_deref()
                .map(|url| format!("\nSource: {url}"))
                .unwrap_or_default();
            sections.push(format!(
                "## {number}. {} — {} ({}){source_line}\n\n{}",
                title,
                provider.to_uppercase(),
                role,
                content.trim()
            ));
        }
        let text = if sections.is_empty() {
            String::new()
        } else {
            format!("# Attached Context\n\n{}", sections.join("\n\n"))
        };
        let character_count = text.chars().count();
        Ok(ComposedContext {
            text,
            item_count: sections.len(),
            character_count,
            estimated_tokens: character_count.div_ceil(4),
        })
    }

    pub fn preview_handoff(&self, input: HandoffInput) -> Result<HandoffPreview> {
        build_handoff_preview(&self.connect()?, &input)
    }

    pub fn record_handoff(&self, input: HandoffInput) -> Result<ConversationHandoff> {
        let mut connection = self.connect()?;
        let preview = build_handoff_preview(&connection, &input)?;
        let handoff = ConversationHandoff {
            id: Uuid::new_v4().to_string(),
            source_conversation_id: preview.source_conversation_id.clone(),
            destination_provider: preview.destination_provider.clone(),
            destination_conversation_id: None,
            mode: preview.mode.clone(),
            content_hash: preview.content_hash.clone(),
            item_count: preview.item_count,
            character_count: preview.character_count,
            estimated_tokens: preview.estimated_tokens,
            created_at: now(),
        };
        let selection_json = serde_json::to_string(&input)?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO conversation_handoffs
             (id, source_conversation_id, destination_provider, destination_conversation_id,
              mode, selection_json, handoff_text, content_hash, item_count, character_count,
              estimated_tokens, created_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                handoff.id,
                handoff.source_conversation_id,
                handoff.destination_provider,
                handoff.mode,
                selection_json,
                preview.text,
                handoff.content_hash,
                handoff.item_count as i64,
                handoff.character_count as i64,
                handoff.estimated_tokens as i64,
                handoff.created_at,
            ],
        )?;
        transaction.commit()?;
        Ok(handoff)
    }

    pub fn list_conversation_handoffs(
        &self,
        source_conversation_id: Option<&str>,
    ) -> Result<Vec<ConversationHandoff>> {
        let connection = self.connect()?;
        let source_conversation_id = source_conversation_id
            .map(|id| uuid("sourceConversationId", id))
            .transpose()?;
        let sql = if source_conversation_id.is_some() {
            "SELECT id, source_conversation_id, destination_provider,
                    destination_conversation_id, mode, content_hash, item_count,
                    character_count, estimated_tokens, created_at
             FROM conversation_handoffs WHERE source_conversation_id = ?1
             ORDER BY created_at DESC, id DESC"
        } else {
            "SELECT id, source_conversation_id, destination_provider,
                    destination_conversation_id, mode, content_hash, item_count,
                    character_count, estimated_tokens, created_at
             FROM conversation_handoffs
             ORDER BY created_at DESC, id DESC"
        };
        let mut statement = connection.prepare(sql)?;
        let map = |row: &rusqlite::Row<'_>| {
            Ok(ConversationHandoff {
                id: row.get(0)?,
                source_conversation_id: row.get(1)?,
                destination_provider: row.get(2)?,
                destination_conversation_id: row.get(3)?,
                mode: row.get(4)?,
                content_hash: row.get(5)?,
                item_count: row.get::<_, i64>(6)? as usize,
                character_count: row.get::<_, i64>(7)? as usize,
                estimated_tokens: row.get::<_, i64>(8)? as usize,
                created_at: row.get(9)?,
            })
        };
        let rows = if let Some(id) = source_conversation_id.as_deref() {
            statement.query_map([id], map)?
        } else {
            statement.query_map([], map)?
        };
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_project(&self, input: CreateProjectInput) -> Result<Project> {
        let name = required_text("name", &input.name, 160)?;
        let description = optional_text("description", &input.description, 10_000)?;
        let project = Project {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            created_at: now(),
            archived_at: None,
        };
        self.connect()?.execute(
            "INSERT INTO projects (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                project.id,
                project.name,
                project.description,
                project.created_at
            ],
        )?;
        Ok(project)
    }

    pub fn create_memory_space(&self, input: CreateMemorySpaceInput) -> Result<MemorySpace> {
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let parent_id = optional_uuid("parentId", input.parent_id.as_deref())?;
        let name = required_text("name", &input.name, 160)?;
        let description = optional_text("description", &input.description, 10_000)?;
        let default_scope = choice("defaultScope", &input.default_scope, SCOPES)?;
        let connection = self.connect()?;
        ensure_optional_exists(&connection, "projects", project_id.as_deref(), "project")?;
        ensure_optional_exists(
            &connection,
            "memory_spaces",
            parent_id.as_deref(),
            "memory space",
        )?;

        if let Some(parent) = parent_id.as_deref() {
            let parent_project: Option<String> = connection.query_row(
                "SELECT project_id FROM memory_spaces WHERE id = ?1",
                [parent],
                |row| row.get(0),
            )?;
            if parent_project != project_id {
                return Err(CoreError::Validation {
                    field: "parentId",
                    message: "parent and child must belong to the same project".into(),
                });
            }
        }

        let space = MemorySpace {
            id: Uuid::new_v4().to_string(),
            project_id,
            parent_id,
            name,
            description,
            default_scope,
            created_at: now(),
            archived_at: None,
        };
        connection.execute(
            "INSERT INTO memory_spaces (id, project_id, parent_id, name, description, default_scope, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![space.id, space.project_id, space.parent_id, space.name, space.description, space.default_scope, space.created_at],
        )?;
        Ok(space)
    }

    pub fn move_memory_space(&self, space_id: &str, parent_id: Option<&str>) -> Result<()> {
        let space_id = uuid("spaceId", space_id)?;
        let parent_id = optional_uuid("parentId", parent_id)?;
        let connection = self.connect()?;
        ensure_exists(&connection, "memory_spaces", &space_id, "memory space")?;
        if parent_id.as_deref() == Some(&space_id) {
            return Err(CoreError::Validation {
                field: "parentId",
                message: "a space cannot be its own parent".into(),
            });
        }
        if let Some(parent) = parent_id.as_deref() {
            ensure_exists(&connection, "memory_spaces", parent, "memory space")?;
            let creates_cycle: bool = connection.query_row(
                "WITH RECURSIVE descendants(id) AS (
                    SELECT id FROM memory_spaces WHERE parent_id = ?1
                    UNION ALL
                    SELECT s.id FROM memory_spaces s JOIN descendants d ON s.parent_id = d.id
                 ) SELECT EXISTS(SELECT 1 FROM descendants WHERE id = ?2)",
                params![space_id, parent],
                |row| row.get(0),
            )?;
            if creates_cycle {
                return Err(CoreError::Validation {
                    field: "parentId",
                    message: "moving this space would create a cycle".into(),
                });
            }
            let child_project: Option<String> = connection.query_row(
                "SELECT project_id FROM memory_spaces WHERE id = ?1",
                [&space_id],
                |row| row.get(0),
            )?;
            let parent_project: Option<String> = connection.query_row(
                "SELECT project_id FROM memory_spaces WHERE id = ?1",
                [parent],
                |row| row.get(0),
            )?;
            if child_project != parent_project {
                return Err(CoreError::Validation {
                    field: "parentId",
                    message: "parent and child must belong to the same project".into(),
                });
            }
        }
        connection.execute(
            "UPDATE memory_spaces SET parent_id = ?1 WHERE id = ?2",
            params![parent_id, space_id],
        )?;
        Ok(())
    }

    pub fn move_memory_to_space(&self, memory_id: &str, space_id: Option<&str>) -> Result<()> {
        let memory_id = uuid("memoryId", memory_id)?;
        let space_id = optional_uuid("spaceId", space_id)?;
        let mut connection = self.connect()?;
        ensure_exists(&connection, "memories", &memory_id, "memory")?;
        ensure_optional_exists(
            &connection,
            "memory_spaces",
            space_id.as_deref(),
            "memory space",
        )?;
        if let Some(space) = space_id.as_deref() {
            let memory_project: Option<String> = connection.query_row(
                "SELECT project_id FROM memories WHERE id = ?1",
                [&memory_id],
                |row| row.get(0),
            )?;
            let space_project: Option<String> = connection.query_row(
                "SELECT project_id FROM memory_spaces WHERE id = ?1",
                [space],
                |row| row.get(0),
            )?;
            if memory_project != space_project {
                return Err(CoreError::Validation {
                    field: "spaceId",
                    message: "memory and space must belong to the same project".into(),
                });
            }
        }
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM memory_space_links WHERE memory_id = ?1",
            [&memory_id],
        )?;
        if let Some(space) = space_id {
            transaction.execute(
                "INSERT INTO memory_space_links (memory_id, memory_space_id) VALUES (?1, ?2)",
                params![memory_id, space],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn create_memory(&self, input: CreateMemoryInput) -> Result<Memory> {
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let memory_space_id = optional_uuid("memorySpaceId", input.memory_space_id.as_deref())?;
        let memory_type = choice("memoryType", &input.memory_type, MEMORY_TYPES)?;
        let authority = choice("authority", &input.authority, AUTHORITIES)?;
        let status = choice("status", &input.status, STATUSES)?;
        let title = required_text("title", &input.title, 160)?;
        let content = required_text("content", &input.content, 1_000_000)?;
        let mut connection = self.connect()?;
        ensure_optional_exists(&connection, "projects", project_id.as_deref(), "project")?;
        ensure_optional_exists(
            &connection,
            "memory_spaces",
            memory_space_id.as_deref(),
            "memory space",
        )?;

        if let Some(space_id) = memory_space_id.as_deref() {
            let space_project: Option<String> = connection.query_row(
                "SELECT project_id FROM memory_spaces WHERE id = ?1",
                [space_id],
                |row| row.get(0),
            )?;
            if project_id.is_some() && space_project != project_id {
                return Err(CoreError::Validation {
                    field: "memorySpaceId",
                    message: "memory and space must belong to the same project".into(),
                });
            }
        }

        let id = Uuid::new_v4().to_string();
        let version_id = Uuid::new_v4().to_string();
        let created_at = now();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO memories (id, project_id, type, authority, status, title, current_version_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            params![id, project_id, memory_type, authority, status, title, created_at],
        )?;
        transaction.execute(
            "INSERT INTO memory_versions (id, memory_id, content, change_type, created_at) VALUES (?1, ?2, ?3, 'create', ?4)",
            params![version_id, id, content, created_at],
        )?;
        transaction.execute(
            "UPDATE memories SET current_version_id = ?1 WHERE id = ?2",
            params![version_id, id],
        )?;
        if let Some(space_id) = memory_space_id {
            transaction.execute(
                "INSERT INTO memory_space_links (memory_id, memory_space_id) VALUES (?1, ?2)",
                params![id, space_id],
            )?;
        }
        transaction.commit()?;
        self.get_memory(&id)
    }

    pub fn append_memory_version(&self, input: AppendVersionInput) -> Result<MemoryVersion> {
        let memory_id = uuid("memoryId", &input.memory_id)?;
        let content = required_text("content", &input.content, 1_000_000)?;
        let change_type = choice("changeType", &input.change_type, CHANGE_TYPES)?;
        let next_status = input
            .next_status
            .as_deref()
            .map(|status| choice("nextStatus", status, STATUSES))
            .transpose()?;
        let mut connection = self.connect()?;
        let current_version_id: String = connection
            .query_row(
                "SELECT current_version_id FROM memories WHERE id = ?1",
                [&memory_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound {
                entity: "memory",
                id: memory_id.clone(),
            })?;
        let version = MemoryVersion {
            id: Uuid::new_v4().to_string(),
            memory_id: memory_id.clone(),
            content,
            change_type,
            created_at: now(),
            supersedes_version_id: Some(current_version_id),
        };
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO memory_versions (id, memory_id, content, change_type, created_at, supersedes_version_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![version.id, version.memory_id, version.content, version.change_type, version.created_at, version.supersedes_version_id],
        )?;
        transaction.execute(
            "UPDATE memories SET current_version_id = ?1, status = COALESCE(?2, status) WHERE id = ?3",
            params![version.id, next_status, memory_id],
        )?;
        transaction.commit()?;
        Ok(version)
    }

    pub fn restore_memory_version(
        &self,
        memory_id: &str,
        version_id: &str,
    ) -> Result<MemoryVersion> {
        let memory_id = uuid("memoryId", memory_id)?;
        let version_id = uuid("versionId", version_id)?;
        let connection = self.connect()?;
        let content: String = connection
            .query_row(
                "SELECT content FROM memory_versions WHERE id = ?1 AND memory_id = ?2",
                params![version_id, memory_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound {
                entity: "memory version",
                id: version_id.clone(),
            })?;
        drop(connection);
        let restored = self.append_memory_version(AppendVersionInput {
            memory_id,
            content,
            change_type: "restore".into(),
            next_status: Some("active".into()),
        })?;
        let connection = self.connect()?;
        insert_memory_source(
            &connection,
            &restored.id,
            &ResolvedMergeSource {
                source_type: "memory".into(),
                source_id: version_id.clone(),
                label: "Restored memory version".into(),
                content: restored.content.clone(),
                source_hash: stable_hash(&restored.content),
                duplicate_of: None,
            },
        )?;
        refresh_memory_conflicts(&connection, &restored.memory_id)?;
        Ok(restored)
    }

    pub fn preview_memory_update(
        &self,
        input: PreviewMemoryUpdateInput,
    ) -> Result<MemoryUpdatePreview> {
        let action = choice("action", &input.action, UPDATE_ACTIONS)?;
        let connection = self.connect()?;
        let memory = input
            .memory_id
            .as_deref()
            .map(|id| self.get_memory(id))
            .transpose()?;
        let before = memory
            .as_ref()
            .map(|value| value.current_content.clone())
            .unwrap_or_default();
        let mut sources = resolve_merge_sources(&connection, input.sources)?;
        mark_duplicate_sources(
            &connection,
            memory.as_ref().map(|value| value.id.as_str()),
            &mut sources,
            &before,
        )?;
        let after = compose_memory_update(&action, &before, &sources)?;
        let conflicts = if let Some(memory) = memory.as_ref() {
            find_conflict_candidates(
                &connection,
                Some(&memory.id),
                memory.project_id.as_deref(),
                &memory.memory_type,
                &memory.title,
                &after,
            )?
        } else {
            Vec::new()
        };
        let duplicate_count = sources
            .iter()
            .filter(|source| source.duplicate_of.is_some())
            .count();
        Ok(MemoryUpdatePreview {
            memory_id: memory.as_ref().map(|value| value.id.clone()),
            current_version_id: memory
                .as_ref()
                .map(|value| value.current_version_id.clone()),
            action,
            unique_source_count: sources.len() - duplicate_count,
            duplicate_count,
            diff: diff_summary(&before, &after),
            sources,
            conflicts,
        })
    }

    pub fn apply_memory_update(&self, input: ApplyMemoryUpdateInput) -> Result<MemoryUpdateResult> {
        let preview = self.preview_memory_update(PreviewMemoryUpdateInput {
            memory_id: input.memory_id.clone(),
            action: input.action.clone(),
            sources: input.sources,
        })?;
        let content = required_text("content", &preview.diff.after, 1_000_000)?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let timestamp = now();

        let (memory_id, version) = if let Some(memory_id) = input.memory_id.as_deref() {
            let memory_id = uuid("memoryId", memory_id)?;
            let current_version_id: String = transaction
                .query_row(
                    "SELECT current_version_id FROM memories WHERE id = ?1",
                    [&memory_id],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or_else(|| CoreError::NotFound {
                    entity: "memory",
                    id: memory_id.clone(),
                })?;
            let expected = input
                .expected_current_version_id
                .as_deref()
                .ok_or_else(|| CoreError::Validation {
                    field: "expectedCurrentVersionId",
                    message: "is required when updating an existing memory".into(),
                })?;
            if uuid("expectedCurrentVersionId", expected)? != current_version_id {
                return Err(CoreError::Validation {
                    field: "expectedCurrentVersionId",
                    message: "memory changed after preview; preview it again".into(),
                });
            }
            let version = MemoryVersion {
                id: Uuid::new_v4().to_string(),
                memory_id: memory_id.clone(),
                content,
                change_type: preview.action.clone(),
                created_at: timestamp.clone(),
                supersedes_version_id: Some(current_version_id.clone()),
            };
            transaction.execute(
                "INSERT INTO memory_versions (id, memory_id, content, change_type, created_at, supersedes_version_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![version.id, version.memory_id, version.content, version.change_type, version.created_at, version.supersedes_version_id],
            )?;
            transaction.execute(
                "UPDATE memories SET current_version_id = ?1, status = 'active' WHERE id = ?2",
                params![version.id, memory_id],
            )?;
            if matches!(preview.action.as_str(), "add" | "merge") {
                copy_memory_sources(&transaction, &current_version_id, &version.id)?;
            }
            (memory_id, version)
        } else {
            let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
            let memory_space_id = optional_uuid("memorySpaceId", input.memory_space_id.as_deref())?;
            let memory_type = choice(
                "memoryType",
                input.memory_type.as_deref().unwrap_or("summary"),
                MEMORY_TYPES,
            )?;
            let authority = choice(
                "authority",
                input.authority.as_deref().unwrap_or("user_confirmed"),
                AUTHORITIES,
            )?;
            let title = required_text("title", input.title.as_deref().unwrap_or(""), 160)?;
            ensure_optional_exists(&transaction, "projects", project_id.as_deref(), "project")?;
            ensure_optional_exists(
                &transaction,
                "memory_spaces",
                memory_space_id.as_deref(),
                "memory space",
            )?;
            if let Some(space_id) = memory_space_id.as_deref() {
                let space_project: Option<String> = transaction.query_row(
                    "SELECT project_id FROM memory_spaces WHERE id = ?1",
                    [space_id],
                    |row| row.get(0),
                )?;
                if space_project != project_id {
                    return Err(CoreError::Validation {
                        field: "memorySpaceId",
                        message: "memory and space must belong to the same project".into(),
                    });
                }
            }
            let memory_id = Uuid::new_v4().to_string();
            let version = MemoryVersion {
                id: Uuid::new_v4().to_string(),
                memory_id: memory_id.clone(),
                content,
                change_type: "merge".into(),
                created_at: timestamp.clone(),
                supersedes_version_id: None,
            };
            transaction.execute(
                "INSERT INTO memories (id, project_id, type, authority, status, title, current_version_id, created_at) VALUES (?1, ?2, ?3, ?4, 'active', ?5, NULL, ?6)",
                params![memory_id, project_id, memory_type, authority, title, timestamp],
            )?;
            transaction.execute(
                "INSERT INTO memory_versions (id, memory_id, content, change_type, created_at) VALUES (?1, ?2, ?3, 'merge', ?4)",
                params![version.id, memory_id, version.content, version.created_at],
            )?;
            transaction.execute(
                "UPDATE memories SET current_version_id = ?1 WHERE id = ?2",
                params![version.id, memory_id],
            )?;
            if let Some(space_id) = memory_space_id {
                transaction.execute(
                    "INSERT INTO memory_space_links (memory_id, memory_space_id) VALUES (?1, ?2)",
                    params![memory_id, space_id],
                )?;
            }
            (memory_id, version)
        };

        for source in preview
            .sources
            .iter()
            .filter(|source| source.duplicate_of.is_none())
        {
            insert_memory_source(&transaction, &version.id, source)?;
        }
        refresh_memory_conflicts(&transaction, &memory_id)?;
        transaction.commit()?;
        Ok(MemoryUpdateResult {
            memory: self.get_memory(&memory_id)?,
            version,
            conflicts: self.list_memory_conflicts(Some(&memory_id), "unresolved")?,
        })
    }

    pub fn memory_history(&self, memory_id: &str) -> Result<MemoryHistory> {
        let memory = self.get_memory(memory_id)?;
        let versions = self.list_memory_versions(memory_id)?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT memory_version_id, source_type, source_id, source_hash, source_label, source_excerpt
             FROM memory_sources
             WHERE memory_version_id IN (SELECT id FROM memory_versions WHERE memory_id = ?1)
               AND source_type IS NOT NULL
             ORDER BY rowid",
        )?;
        let sources = statement
            .query_map([memory_id], |row| {
                Ok(MemorySource {
                    memory_version_id: row.get(0)?,
                    source_type: row.get(1)?,
                    source_id: row.get(2)?,
                    source_hash: row.get(3)?,
                    source_label: row.get(4)?,
                    source_excerpt: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(MemoryHistory {
            memory,
            versions,
            sources,
        })
    }

    pub fn create_memory_branch(&self, input: CreateMemoryBranchInput) -> Result<MemoryBranch> {
        let memory_id = uuid("memoryId", &input.memory_id)?;
        let name = required_text("name", &input.name, 160)?;
        let memory = self.get_memory(&memory_id)?;
        let mut connection = self.connect()?;
        let timestamp = now();
        let branch_id = Uuid::new_v4().to_string();
        let branch_version_id = Uuid::new_v4().to_string();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO branches (id, base_context_id, name, status, created_at, memory_id, base_version_id, updated_at) VALUES (?1, ?2, ?3, 'active', ?4, ?5, ?6, ?4)",
            params![branch_id, memory.current_version_id, name, timestamp, memory_id, memory.current_version_id],
        )?;
        transaction.execute(
            "INSERT INTO branch_versions (id, branch_id, content, change_type, created_at) VALUES (?1, ?2, ?3, 'create', ?4)",
            params![branch_version_id, branch_id, memory.current_content, timestamp],
        )?;
        transaction.commit()?;
        self.get_memory_branch(&branch_id)
    }

    pub fn append_branch_version(&self, input: AppendBranchVersionInput) -> Result<MemoryBranch> {
        let branch_id = uuid("branchId", &input.branch_id)?;
        let content = required_text("content", &input.content, 1_000_000)?;
        let change_type = choice("changeType", &input.change_type, BRANCH_CHANGE_TYPES)?;
        let mut connection = self.connect()?;
        let (status, current_version_id): (String, String) = connection
            .query_row(
                "SELECT b.status, bv.id FROM branches b JOIN branch_versions bv ON bv.branch_id = b.id WHERE b.id = ?1 ORDER BY bv.created_at DESC, bv.rowid DESC LIMIT 1",
                [&branch_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound {
                entity: "memory branch",
                id: branch_id.clone(),
            })?;
        if status != "active" {
            return Err(CoreError::Validation {
                field: "branchId",
                message: "only active branches can be edited".into(),
            });
        }
        let timestamp = now();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO branch_versions (id, branch_id, content, change_type, created_at, supersedes_version_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![Uuid::new_v4().to_string(), branch_id, content, change_type, timestamp, current_version_id],
        )?;
        transaction.execute(
            "UPDATE branches SET updated_at = ?1 WHERE id = ?2",
            params![timestamp, branch_id],
        )?;
        transaction.commit()?;
        self.get_memory_branch(&branch_id)
    }

    pub fn list_memory_branches(&self, memory_id: Option<&str>) -> Result<Vec<MemoryBranch>> {
        let memory_id = optional_uuid("memoryId", memory_id)?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT b.id, b.memory_id, b.base_version_id, b.name, b.status,
                    bv.id, bv.content, b.created_at, COALESCE(b.updated_at, b.created_at)
             FROM branches b
             JOIN branch_versions bv ON bv.id = (
                 SELECT id FROM branch_versions WHERE branch_id = b.id ORDER BY created_at DESC, rowid DESC LIMIT 1
             )
             WHERE b.memory_id IS NOT NULL AND (?1 IS NULL OR b.memory_id = ?1)
             ORDER BY b.status != 'active', COALESCE(b.updated_at, b.created_at) DESC",
        )?;
        let rows = statement.query_map([memory_id], map_memory_branch)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn get_memory_branch(&self, branch_id: &str) -> Result<MemoryBranch> {
        let branch_id = uuid("branchId", branch_id)?;
        self.connect()?
            .query_row(
                "SELECT b.id, b.memory_id, b.base_version_id, b.name, b.status,
                        bv.id, bv.content, b.created_at, COALESCE(b.updated_at, b.created_at)
                 FROM branches b
                 JOIN branch_versions bv ON bv.id = (
                     SELECT id FROM branch_versions WHERE branch_id = b.id ORDER BY created_at DESC, rowid DESC LIMIT 1
                 )
                 WHERE b.id = ?1 AND b.memory_id IS NOT NULL",
                [&branch_id],
                map_memory_branch,
            )
            .optional()?
            .ok_or(CoreError::NotFound {
                entity: "memory branch",
                id: branch_id,
            })
    }

    pub fn finalize_memory_branch(&self, branch_id: &str, mode: &str) -> Result<MemoryBranch> {
        let branch_id = uuid("branchId", branch_id)?;
        let mode = choice("mode", mode, &["promote", "merge", "abandon"])?;
        let branch = self.get_memory_branch(&branch_id)?;
        if branch.status != "active" {
            return Err(CoreError::Validation {
                field: "branchId",
                message: "branch is already finalized".into(),
            });
        }
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let status = match mode.as_str() {
            "abandon" => "abandoned",
            "promote" => "promoted",
            "merge" => "merged",
            _ => unreachable!(),
        };
        if mode != "abandon" {
            let current: (String, String) = transaction.query_row(
                "SELECT current_version_id, v.content FROM memories m JOIN memory_versions v ON v.id = m.current_version_id WHERE m.id = ?1",
                [&branch.memory_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            let content = if mode == "merge" {
                merge_paragraphs(&current.1, [branch.current_content.as_str()])
            } else {
                branch.current_content.clone()
            };
            let version_id = Uuid::new_v4().to_string();
            let timestamp = now();
            let change_type = if mode == "merge" { "merge" } else { "replace" };
            transaction.execute(
                "INSERT INTO memory_versions (id, memory_id, content, change_type, created_at, supersedes_version_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![version_id, branch.memory_id, content, change_type, timestamp, current.0],
            )?;
            transaction.execute(
                "UPDATE memories SET current_version_id = ?1, status = 'active' WHERE id = ?2",
                params![version_id, branch.memory_id],
            )?;
            if mode == "merge" {
                copy_memory_sources(&transaction, &current.0, &version_id)?;
            }
            insert_memory_source(
                &transaction,
                &version_id,
                &ResolvedMergeSource {
                    source_type: "branch".into(),
                    source_id: branch.id.clone(),
                    label: format!("Branch: {}", branch.name),
                    content: branch.current_content.clone(),
                    source_hash: stable_hash(&branch.current_content),
                    duplicate_of: None,
                },
            )?;
            refresh_memory_conflicts(&transaction, &branch.memory_id)?;
        }
        transaction.execute(
            "UPDATE branches SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now(), branch_id],
        )?;
        transaction.commit()?;
        self.get_memory_branch(&branch_id)
    }

    pub fn list_memory_conflicts(
        &self,
        memory_id: Option<&str>,
        status: &str,
    ) -> Result<Vec<MemoryConflict>> {
        let memory_id = optional_uuid("memoryId", memory_id)?;
        let status = choice(
            "status",
            status,
            &["unresolved", "resolved", "ignored", "all"],
        )?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT c.id, c.memory_id, m.title, c.conflicting_memory_id, other.title,
                    c.conflict_key, c.current_value, c.conflicting_value, c.status,
                    c.created_at, c.resolved_at
             FROM memory_conflicts c
             JOIN memories m ON m.id = c.memory_id
             JOIN memories other ON other.id = c.conflicting_memory_id
             WHERE (?1 IS NULL OR c.memory_id = ?1) AND (?2 = 'all' OR c.status = ?2)
             ORDER BY c.status != 'unresolved', c.created_at DESC",
        )?;
        let rows = statement.query_map(params![memory_id, status], map_memory_conflict)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn resolve_memory_conflict(&self, conflict_id: &str, resolution: &str) -> Result<()> {
        let conflict_id = uuid("conflictId", conflict_id)?;
        let resolution = choice("resolution", resolution, &["resolved", "ignored"])?;
        let changed = self.connect()?.execute(
            "UPDATE memory_conflicts SET status = ?1, resolved_at = ?2 WHERE id = ?3 AND status = 'unresolved'",
            params![resolution, now(), conflict_id],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound {
                entity: "unresolved conflict",
                id: conflict_id,
            });
        }
        Ok(())
    }

    pub fn create_context_pack(&self, input: CreateContextPackInput) -> Result<ContextPack> {
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let name = required_text("name", &input.name, 160)?;
        let description = optional_text("description", &input.description, 10_000)?;
        let connection = self.connect()?;
        ensure_optional_exists(&connection, "projects", project_id.as_deref(), "project")?;
        let timestamp = now();
        let pack = ContextPack {
            id: Uuid::new_v4().to_string(),
            project_id,
            name,
            description,
            current_version: 1,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        connection.execute(
            "INSERT INTO context_packs (id, project_id, name, description, current_version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![pack.id, pack.project_id, pack.name, pack.description, pack.current_version, pack.created_at, pack.updated_at],
        )?;
        Ok(pack)
    }

    pub fn save_context_pack(&self, input: SaveContextPackInput) -> Result<ContextPackDetail> {
        let id = input
            .id
            .as_deref()
            .map(|value| uuid("id", value))
            .transpose()?
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let name = required_text("name", &input.name, 160)?;
        let description = optional_text("description", &input.description, 10_000)?;
        let mut connection = self.connect()?;
        ensure_optional_exists(&connection, "projects", project_id.as_deref(), "project")?;
        for item in &input.items {
            uuid("targetId", &item.target_id)?;
            choice(
                "inclusionMode",
                &item.inclusion_mode,
                &["full", "summary", "reference"],
            )?;
            ensure_exists(&connection, "memories", &item.target_id, "memory")?;
        }
        let timestamp = now();
        let transaction = connection.transaction()?;
        let existing: Option<(String, i64)> = transaction
            .query_row(
                "SELECT created_at, current_version FROM context_packs WHERE id = ?1",
                [&id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (created_at, current_version) = if let Some((created_at, version)) = existing {
            transaction.execute(
                "UPDATE context_packs SET project_id = ?1, name = ?2, description = ?3, current_version = current_version + 1, updated_at = ?4 WHERE id = ?5",
                params![project_id, name, description, timestamp, id],
            )?;
            transaction.execute("DELETE FROM context_pack_items WHERE pack_id = ?1", [&id])?;
            (created_at, version + 1)
        } else {
            transaction.execute(
                "INSERT INTO context_packs (id, project_id, name, description, current_version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
                params![id, project_id, name, description, timestamp],
            )?;
            (timestamp.clone(), 1)
        };
        let mut items = Vec::with_capacity(input.items.len());
        for (ordering, item) in input.items.into_iter().enumerate() {
            let item = ContextPackItem {
                target_id: item.target_id,
                ordering: ordering as i64,
                inclusion_mode: item.inclusion_mode,
            };
            transaction.execute(
                "INSERT INTO context_pack_items (pack_id, target_type, target_id, ordering, inclusion_mode) VALUES (?1, 'memory', ?2, ?3, ?4)",
                params![id, item.target_id, item.ordering, item.inclusion_mode],
            )?;
            items.push(item);
        }
        transaction.commit()?;
        Ok(ContextPackDetail {
            pack: ContextPack {
                id,
                project_id,
                name,
                description,
                current_version,
                created_at,
                updated_at: timestamp,
            },
            items,
        })
    }

    pub fn list_context_pack_details(&self) -> Result<Vec<ContextPackDetail>> {
        let packs = list_context_packs(&self.connect()?)?;
        let connection = self.connect()?;
        packs
            .into_iter()
            .map(|pack| {
                let mut statement = connection.prepare(
                    "SELECT target_id, ordering, inclusion_mode FROM context_pack_items WHERE pack_id = ?1 AND target_type = 'memory' ORDER BY ordering",
                )?;
                let rows = statement.query_map([&pack.id], |row| {
                    Ok(ContextPackItem {
                        target_id: row.get(0)?,
                        ordering: row.get(1)?,
                        inclusion_mode: row.get(2)?,
                    })
                })?;
                Ok(ContextPackDetail {
                    pack,
                    items: rows.collect::<std::result::Result<Vec<_>, _>>()?,
                })
            })
            .collect()
    }

    pub fn create_temporary_attachment(
        &self,
        input: CreateTemporaryAttachmentInput,
    ) -> Result<TemporaryAttachment> {
        let conversation_id = uuid("conversationId", &input.conversation_id)?;
        let target_type = choice("targetType", &input.target_type, TARGET_TYPES)?;
        let target_id = uuid("targetId", &input.target_id)?;
        let lifetime_mode = choice("lifetimeMode", &input.lifetime_mode, LIFETIMES)?;
        let remaining_prompts = match lifetime_mode.as_str() {
            "one_prompt" => Some(1),
            "n_prompts" => match input.remaining_prompts {
                Some(value) if value > 0 => Some(value),
                _ => {
                    return Err(CoreError::Validation {
                        field: "remainingPrompts",
                        message: "must be positive for n_prompts".into(),
                    });
                }
            },
            _ => None,
        };
        let mut connection = self.connect()?;
        ensure_exists(
            &connection,
            "conversations",
            &conversation_id,
            "conversation",
        )?;
        ensure_target_exists(&connection, &target_type, &target_id)?;
        let attachment = TemporaryAttachment {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            target_type,
            target_id,
            lifetime_mode,
            remaining_prompts,
            expires_at: input.expires_at,
            created_at: now(),
        };
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM temporary_attachments WHERE conversation_id = ?1 AND target_type = ?2 AND target_id = ?3",
            params![attachment.conversation_id, attachment.target_type, attachment.target_id],
        )?;
        transaction.execute(
            "INSERT INTO temporary_attachments (id, conversation_id, target_type, target_id, lifetime_mode, remaining_prompts, expires_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![attachment.id, attachment.conversation_id, attachment.target_type, attachment.target_id, attachment.lifetime_mode, attachment.remaining_prompts, attachment.expires_at, attachment.created_at],
        )?;
        transaction.commit()?;
        Ok(attachment)
    }

    pub fn consume_prompt(&self, conversation_id: &str) -> Result<Vec<TemporaryAttachment>> {
        let conversation_id = uuid("conversationId", conversation_id)?;
        let mut connection = self.connect()?;
        ensure_exists(
            &connection,
            "conversations",
            &conversation_id,
            "conversation",
        )?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "DELETE FROM temporary_attachments WHERE conversation_id = ?1 AND remaining_prompts = 1",
            [&conversation_id],
        )?;
        transaction.execute(
            "UPDATE temporary_attachments SET remaining_prompts = remaining_prompts - 1 WHERE conversation_id = ?1 AND remaining_prompts > 1",
            [&conversation_id],
        )?;
        let remaining = list_temporary_attachments(&transaction, &conversation_id)?;
        transaction.commit()?;
        Ok(remaining)
    }

    pub fn set_context_binding(
        &self,
        conversation_id: &str,
        target_type: &str,
        target_id: &str,
        enabled: bool,
        lifetime_mode: &str,
    ) -> Result<()> {
        let conversation_id = uuid("conversationId", conversation_id)?;
        let target_type = choice("targetType", target_type, TARGET_TYPES)?;
        let target_id = uuid("targetId", target_id)?;
        let lifetime_mode = choice("lifetimeMode", lifetime_mode, LIFETIMES)?;
        let connection = self.connect()?;
        ensure_exists(
            &connection,
            "conversations",
            &conversation_id,
            "conversation",
        )?;
        ensure_target_exists(&connection, &target_type, &target_id)?;
        connection.execute(
            "INSERT INTO chat_context_bindings (conversation_id, context_target_type, context_target_id, enabled, lifetime_mode) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(conversation_id, context_target_type, context_target_id) DO UPDATE SET enabled = excluded.enabled, lifetime_mode = excluded.lifetime_mode",
            params![conversation_id, target_type, target_id, enabled, lifetime_mode],
        )?;
        Ok(())
    }

    pub fn conversation_context_by_reference(
        &self,
        provider: &str,
        external_ref: &str,
    ) -> Result<Option<ConversationContextState>> {
        let provider = required_text("provider", provider, 80)?;
        let external_ref = required_text("externalRef", external_ref, 500)?;
        let connection = self.connect()?;
        let conversation_id = connection
            .query_row(
                "SELECT id FROM conversations WHERE provider = ?1 AND external_ref = ?2",
                params![provider, external_ref],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        conversation_id
            .map(|id| conversation_context_state(&connection, &id))
            .transpose()
    }

    pub fn conversation_context(&self, conversation_id: &str) -> Result<ConversationContextState> {
        let conversation_id = uuid("conversationId", conversation_id)?;
        let connection = self.connect()?;
        ensure_exists(
            &connection,
            "conversations",
            &conversation_id,
            "conversation",
        )?;
        conversation_context_state(&connection, &conversation_id)
    }

    pub fn clear_temporary_context(&self, conversation_id: &str) -> Result<usize> {
        let conversation_id = uuid("conversationId", conversation_id)?;
        let connection = self.connect()?;
        ensure_exists(
            &connection,
            "conversations",
            &conversation_id,
            "conversation",
        )?;
        connection
            .execute(
                "DELETE FROM temporary_attachments WHERE conversation_id = ?1",
                [&conversation_id],
            )
            .map_err(Into::into)
    }

    pub fn snapshot(&self) -> Result<DashboardSnapshot> {
        let connection = self.connect()?;
        Ok(DashboardSnapshot {
            projects: list_projects(&connection)?,
            memory_spaces: list_memory_spaces(&connection)?,
            memories: list_memories(&connection)?,
            context_packs: list_context_packs(&connection)?,
            memory_space_links: list_memory_space_links(&connection)?,
        })
    }

    pub fn get_memory(&self, id: &str) -> Result<Memory> {
        let id = uuid("memoryId", id)?;
        self.connect()?
            .query_row(
                "SELECT m.id, m.project_id, m.type, m.authority, m.status, m.title, m.current_version_id, v.content, m.created_at FROM memories m JOIN memory_versions v ON v.id = m.current_version_id WHERE m.id = ?1",
                [&id],
                map_memory,
            )
            .optional()?
            .ok_or(CoreError::NotFound { entity: "memory", id })
    }

    pub fn list_memory_versions(&self, memory_id: &str) -> Result<Vec<MemoryVersion>> {
        let memory_id = uuid("memoryId", memory_id)?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, memory_id, content, change_type, created_at, supersedes_version_id FROM memory_versions WHERE memory_id = ?1 ORDER BY created_at, rowid",
        )?;
        let rows = statement.query_map([memory_id], map_memory_version)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn detect_secrets(&self, text: &str) -> Vec<SecretWarning> {
        detect_secret_warnings(text)
    }

    pub fn preview_import(&self, input: ImportInput) -> Result<ImportPreview> {
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let memory_space_id = optional_uuid("memorySpaceId", input.memory_space_id.as_deref())?;
        let connection = self.connect()?;
        validate_import_destination(
            &connection,
            project_id.as_deref(),
            memory_space_id.as_deref(),
        )?;
        let (source_label, source_format, source_hash, mut candidates) =
            parse_import_file(Path::new(&input.source_path))?;
        mark_import_duplicates(&connection, &mut candidates)?;
        let duplicate_count = candidates.iter().filter(|item| item.duplicate).count();
        let secret_warning_count = candidates
            .iter()
            .map(|item| item.secret_warnings.len())
            .sum();
        Ok(ImportPreview {
            source_label,
            source_format,
            source_hash,
            importable_count: candidates.len() - duplicate_count,
            duplicate_count,
            secret_warning_count,
            candidates,
        })
    }

    pub fn import_file(&self, input: ImportInput) -> Result<ImportResult> {
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let memory_space_id = optional_uuid("memorySpaceId", input.memory_space_id.as_deref())?;
        let preview = self.preview_import(input)?;
        let mut connection = self.connect()?;
        validate_import_destination(
            &connection,
            project_id.as_deref(),
            memory_space_id.as_deref(),
        )?;
        let import_id = Uuid::new_v4().to_string();
        let timestamp = now();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO import_runs
             (id, source_label, source_format, source_hash, imported_count, skipped_count, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)",
            params![
                import_id,
                preview.source_label,
                preview.source_format,
                preview.source_hash,
                preview.duplicate_count as i64,
                timestamp,
            ],
        )?;
        let mut imported_count = 0_usize;
        for candidate in preview
            .candidates
            .into_iter()
            .filter(|candidate| !candidate.duplicate)
        {
            let memory_id = Uuid::new_v4().to_string();
            let version_id = Uuid::new_v4().to_string();
            transaction.execute(
                "INSERT INTO memories
                 (id, project_id, type, authority, status, title, current_version_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
                params![
                    memory_id,
                    project_id,
                    candidate.memory_type,
                    candidate.authority,
                    candidate.status,
                    candidate.title,
                    timestamp,
                ],
            )?;
            transaction.execute(
                "INSERT INTO memory_versions
                 (id, memory_id, content, change_type, created_at)
                 VALUES (?1, ?2, ?3, 'create', ?4)",
                params![version_id, memory_id, candidate.content, timestamp],
            )?;
            transaction.execute(
                "UPDATE memories SET current_version_id = ?1 WHERE id = ?2",
                params![version_id, memory_id],
            )?;
            if let Some(space_id) = memory_space_id.as_deref() {
                transaction.execute(
                    "INSERT INTO memory_space_links (memory_id, memory_space_id) VALUES (?1, ?2)",
                    params![memory_id, space_id],
                )?;
            }
            transaction.execute(
                "INSERT INTO import_items (import_id, content_hash, entity_type, entity_id)
                 VALUES (?1, ?2, 'memory', ?3)",
                params![import_id, candidate.content_hash, memory_id],
            )?;
            imported_count += 1;
        }
        transaction.execute(
            "UPDATE import_runs SET imported_count = ?1 WHERE id = ?2",
            params![imported_count as i64, import_id],
        )?;
        transaction.commit()?;
        Ok(ImportResult {
            import_id,
            source_label: preview.source_label,
            imported_count,
            skipped_count: preview.duplicate_count,
        })
    }

    pub fn backup_settings(&self) -> Result<BackupSettings> {
        read_backup_settings(&self.connect()?)
    }

    pub fn update_backup_settings(
        &self,
        input: UpdateBackupSettingsInput,
    ) -> Result<BackupSettings> {
        if !(1..=8760).contains(&input.interval_hours) {
            return Err(CoreError::Validation {
                field: "intervalHours",
                message: "must be between 1 and 8760".into(),
            });
        }
        let directory = optional_text("directory", &input.directory, 32_000)?;
        if input.enabled {
            let path = validate_absolute_path("directory", &directory)?;
            if !path.is_dir() {
                return Err(CoreError::Validation {
                    field: "directory",
                    message: "must be an existing directory".into(),
                });
            }
        }
        let connection = self.connect()?;
        connection.execute(
            "UPDATE backup_settings
             SET enabled = ?1, interval_hours = ?2, directory = ?3, updated_at = ?4
             WHERE singleton = 1",
            params![input.enabled, input.interval_hours, directory, now()],
        )?;
        read_backup_settings(&connection)
    }

    pub fn run_scheduled_backup(&self, force: bool) -> Result<ScheduledBackupResult> {
        let settings = self.backup_settings()?;
        if !settings.enabled && !force {
            return Ok(ScheduledBackupResult {
                created: false,
                path: None,
                reason: "scheduled backups are disabled".into(),
                settings,
            });
        }
        if settings.directory.is_empty() {
            return Err(CoreError::Validation {
                field: "directory",
                message: "choose a backup directory first".into(),
            });
        }
        if !force && !backup_is_due(&settings) {
            return Ok(ScheduledBackupResult {
                created: false,
                path: None,
                reason: "the next backup is not due yet".into(),
                settings,
            });
        }
        let directory = validate_absolute_path("directory", &settings.directory)?;
        if !directory.is_dir() {
            return Err(CoreError::Validation {
                field: "directory",
                message: "backup directory is unavailable".into(),
            });
        }
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let mut destination = directory.join(format!("tf0000-auto-{timestamp}.db"));
        if destination.exists() {
            destination = directory.join(format!(
                "tf0000-auto-{timestamp}-{}.db",
                &Uuid::new_v4().to_string()[..8]
            ));
        }
        self.backup(&destination)?;
        let completed_at = now();
        self.connect()?.execute(
            "UPDATE backup_settings SET last_backup_at = ?1, updated_at = ?1 WHERE singleton = 1",
            [&completed_at],
        )?;
        Ok(ScheduledBackupResult {
            created: true,
            path: Some(destination.display().to_string()),
            reason: "backup created".into(),
            settings: self.backup_settings()?,
        })
    }

    pub fn restore_backup(&self, source: impl AsRef<Path>) -> Result<RestoreResult> {
        let source = source.as_ref();
        validate_database_path(source)?;
        let source_canonical = source.canonicalize()?;
        if self.database_path.exists() && self.database_path.canonicalize()? == source_canonical {
            return Err(CoreError::Validation {
                field: "source",
                message: "cannot restore the active database from itself".into(),
            });
        }
        let parent = self
            .database_path
            .parent()
            .ok_or_else(|| CoreError::Validation {
                field: "databasePath",
                message: "has no parent directory".into(),
            })?;
        let staged = parent.join(format!("tf0000-restore-{}.db", Uuid::new_v4()));
        fs::copy(&source_canonical, &staged)?;
        let staged_store = match ContextStore::open(&staged) {
            Ok(store) => store,
            Err(error) => {
                let _ = fs::remove_file(&staged);
                return Err(error);
            }
        };
        let recovery = unique_database_path(parent, "tf0000-pre-restore")?;
        self.backup(&recovery)?;
        let copy_result = copy_database(&staged_store, self);
        let _ = fs::remove_file(&staged);
        if let Err(error) = copy_result {
            let recovery_store = ContextStore::open(&recovery)?;
            copy_database(&recovery_store, self)?;
            return Err(error);
        }
        let health = self.health()?;
        Ok(RestoreResult {
            restored_from: source_canonical.display().to_string(),
            recovery_backup_path: recovery.display().to_string(),
            schema_version: health.schema_version,
        })
    }

    pub fn diagnostics(&self) -> Result<DiagnosticReport> {
        let connection = self.connect()?;
        let integrity: String =
            connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        let sqlite_version: String =
            connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))?;
        let schema_version =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        Ok(DiagnosticReport {
            generated_at: now(),
            application: "TF0000".into(),
            status: if integrity == "ok" {
                "healthy"
            } else {
                "degraded"
            }
            .into(),
            schema_version,
            sqlite_version,
            database_path: self.database_path.display().to_string(),
            integrity,
            project_count: table_count(&connection, "projects")?,
            conversation_count: table_count(&connection, "conversations")?,
            message_count: table_count(&connection, "messages")?,
            memory_count: table_count(&connection, "memories")?,
            context_pack_count: table_count(&connection, "context_packs")?,
            unresolved_conflict_count: connection.query_row(
                "SELECT count(*) FROM memory_conflicts WHERE status = 'unresolved'",
                [],
                |row| row.get(0),
            )?,
            handoff_count: table_count(&connection, "conversation_handoffs")?,
            import_count: table_count(&connection, "import_runs")?,
            backup_settings: read_backup_settings(&connection)?,
        })
    }

    pub fn export_diagnostics(&self, destination: impl AsRef<Path>) -> Result<PathBuf> {
        let destination = prepare_destination_with_extension(destination, &["json"])?;
        fs::write(
            &destination,
            serde_json::to_vec_pretty(&self.diagnostics()?)?,
        )?;
        Ok(destination)
    }

    pub fn smart_settings(&self) -> Result<SmartSettings> {
        read_smart_settings(&self.connect()?)
    }

    pub fn update_smart_settings(&self, input: UpdateSmartSettingsInput) -> Result<SmartSettings> {
        let mode = choice("mode", &input.mode, &["off", "local", "provider"])?;
        let recommendation_mode = choice(
            "recommendationMode",
            &input.recommendation_mode,
            &["off", "ask", "automatic"],
        )?;
        let provider_name = optional_text("providerName", &input.provider_name, 80)?;
        let provider_endpoint = optional_text("providerEndpoint", &input.provider_endpoint, 2_000)?;
        let provider_model = optional_text("providerModel", &input.provider_model, 160)?;
        if mode == "provider" {
            if provider_name.is_empty() || provider_endpoint.is_empty() || provider_model.is_empty()
            {
                return Err(CoreError::Validation {
                    field: "provider",
                    message: "name, endpoint and model are required in provider mode".into(),
                });
            }
            let lower = provider_endpoint.to_ascii_lowercase();
            if !(lower.starts_with("https://")
                || lower.starts_with("http://localhost")
                || lower.starts_with("http://127.0.0.1"))
            {
                return Err(CoreError::Validation {
                    field: "providerEndpoint",
                    message: "must use HTTPS, localhost or 127.0.0.1".into(),
                });
            }
        }
        let connection = self.connect()?;
        connection.execute(
            "UPDATE smart_settings SET mode = ?1, provider_name = ?2,
             provider_endpoint = ?3, provider_model = ?4, recommendation_mode = ?5,
             updated_at = ?6 WHERE singleton = 1",
            params![
                mode,
                provider_name,
                provider_endpoint,
                provider_model,
                recommendation_mode,
                now()
            ],
        )?;
        if mode != "off" {
            sync_semantic_index(&connection)?;
        }
        read_smart_settings(&connection)
    }

    pub fn rebuild_semantic_index(&self) -> Result<usize> {
        let connection = self.connect()?;
        ensure_smart_enabled(&connection)?;
        connection.execute("DELETE FROM semantic_documents", [])?;
        sync_semantic_index(&connection)
    }

    pub fn generate_summary(&self, input: GenerateSummaryInput) -> Result<GeneratedArtifact> {
        let source_type = choice(
            "sourceType",
            &input.source_type,
            &["memory", "conversation"],
        )?;
        let source_id = uuid("sourceId", &input.source_id)?;
        let maximum_characters = input.maximum_characters.unwrap_or(700);
        if !(160..=4_000).contains(&maximum_characters) {
            return Err(CoreError::Validation {
                field: "maximumCharacters",
                message: "must be between 160 and 4000".into(),
            });
        }
        let connection = self.connect()?;
        ensure_smart_enabled(&connection)?;
        let content = smart_source_content(&connection, &source_type, &source_id)?;
        let summary = extractive_summary(&content, maximum_characters);
        insert_generated_artifact(
            &connection,
            &source_type,
            &source_id,
            "summary",
            &summary,
            "local",
        )
    }

    pub fn generated_artifacts(&self, source_id: Option<&str>) -> Result<Vec<GeneratedArtifact>> {
        let source_id = source_id.map(|value| uuid("sourceId", value)).transpose()?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, source_type, source_id, artifact_type, content, model_mode, created_at
             FROM generated_artifacts WHERE (?1 IS NULL OR source_id = ?1)
             ORDER BY created_at DESC, id DESC LIMIT 200",
        )?;
        let rows = statement.query_map([source_id], map_generated_artifact)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn extract_candidates(&self, input: ExtractCandidatesInput) -> Result<Vec<SmartCandidate>> {
        let source_type = choice(
            "sourceType",
            &input.source_type,
            &["memory", "conversation"],
        )?;
        let source_id = uuid("sourceId", &input.source_id)?;
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let connection = self.connect()?;
        ensure_smart_enabled(&connection)?;
        ensure_optional_exists(&connection, "projects", project_id.as_deref(), "project")?;
        let content = smart_source_content(&connection, &source_type, &source_id)?;
        let extracted = extract_candidate_statements(&content);
        let timestamp = now();
        let mut output = Vec::new();
        for (memory_type, statement, confidence) in extracted {
            let duplicate: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM smart_candidates
                 WHERE source_type = ?1 AND source_id = ?2 AND memory_type = ?3 AND content = ?4)",
                params![source_type, source_id, memory_type, statement],
                |row| row.get(0),
            )?;
            if duplicate {
                continue;
            }
            let candidate = SmartCandidate {
                id: Uuid::new_v4().to_string(),
                project_id: project_id.clone(),
                source_type: source_type.clone(),
                source_id: source_id.clone(),
                memory_type,
                title: candidate_title(&statement),
                content: statement,
                confidence,
                status: "pending".into(),
                created_at: timestamp.clone(),
                reviewed_at: None,
            };
            connection.execute(
                "INSERT INTO smart_candidates
                 (id, project_id, source_type, source_id, memory_type, title, content,
                  confidence, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9)",
                params![
                    candidate.id,
                    candidate.project_id,
                    candidate.source_type,
                    candidate.source_id,
                    candidate.memory_type,
                    candidate.title,
                    candidate.content,
                    candidate.confidence,
                    candidate.created_at,
                ],
            )?;
            output.push(candidate);
        }
        Ok(output)
    }

    pub fn smart_candidates(&self, status: Option<&str>) -> Result<Vec<SmartCandidate>> {
        let status = status
            .map(|value| choice("status", value, &["pending", "accepted", "dismissed"]))
            .transpose()?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, project_id, source_type, source_id, memory_type, title, content,
                    confidence, status, created_at, reviewed_at
             FROM smart_candidates WHERE (?1 IS NULL OR status = ?1)
             ORDER BY created_at DESC, id DESC LIMIT 500",
        )?;
        let rows = statement.query_map([status], map_smart_candidate)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn review_smart_candidate(
        &self,
        input: ReviewSmartCandidateInput,
    ) -> Result<SmartCandidateReview> {
        let candidate_id = uuid("candidateId", &input.candidate_id)?;
        let action = choice("action", &input.action, &["accept", "dismiss"])?;
        let memory_space_id = optional_uuid("memorySpaceId", input.memory_space_id.as_deref())?;
        let mut connection = self.connect()?;
        let candidate = get_smart_candidate(&connection, &candidate_id)?;
        if candidate.status != "pending" {
            return Err(CoreError::Validation {
                field: "candidateId",
                message: "candidate has already been reviewed".into(),
            });
        }
        let transaction = connection.transaction()?;
        let memory = if action == "accept" {
            validate_import_destination(
                &transaction,
                candidate.project_id.as_deref(),
                memory_space_id.as_deref(),
            )?;
            let memory_id = Uuid::new_v4().to_string();
            let version_id = Uuid::new_v4().to_string();
            let timestamp = now();
            transaction.execute(
                "INSERT INTO memories
                 (id, project_id, type, authority, status, title, current_version_id, created_at)
                 VALUES (?1, ?2, ?3, 'ai_suggestion', 'draft', ?4, ?5, ?6)",
                params![
                    memory_id,
                    candidate.project_id,
                    candidate.memory_type,
                    candidate.title,
                    version_id,
                    timestamp
                ],
            )?;
            transaction.execute(
                "INSERT INTO memory_versions
                 (id, memory_id, content, change_type, created_at)
                 VALUES (?1, ?2, ?3, 'add', ?4)",
                params![version_id, memory_id, candidate.content, timestamp],
            )?;
            if let Some(space_id) = memory_space_id {
                transaction.execute(
                    "INSERT INTO memory_space_links (memory_id, memory_space_id) VALUES (?1, ?2)",
                    params![memory_id, space_id],
                )?;
            }
            Some(Memory {
                id: memory_id,
                project_id: candidate.project_id.clone(),
                memory_type: candidate.memory_type.clone(),
                authority: "ai_suggestion".into(),
                status: "draft".into(),
                title: candidate.title.clone(),
                current_version_id: version_id,
                current_content: candidate.content.clone(),
                created_at: timestamp,
            })
        } else {
            None
        };
        let reviewed_at = now();
        transaction.execute(
            "UPDATE smart_candidates SET status = ?1, reviewed_at = ?2 WHERE id = ?3",
            params![
                if action == "accept" {
                    "accepted"
                } else {
                    "dismissed"
                },
                reviewed_at,
                candidate_id
            ],
        )?;
        transaction.commit()?;
        Ok(SmartCandidateReview {
            candidate: get_smart_candidate(&self.connect()?, &candidate_id)?,
            memory,
        })
    }

    pub fn explain_conflict(&self, conflict_id: &str) -> Result<ConflictExplanation> {
        let conflict_id = uuid("conflictId", conflict_id)?;
        let connection = self.connect()?;
        ensure_smart_enabled(&connection)?;
        let (key, first_id, first_title, first_content, second_id, second_title, second_content): (
            String, String, String, String, String, String, String,
        ) = connection
            .query_row(
                "SELECT c.conflict_key, left_memory.id, left_memory.title, left_version.content,
                        right_memory.id, right_memory.title, right_version.content
                 FROM memory_conflicts c
                 JOIN memories left_memory ON left_memory.id = c.memory_id
                 JOIN memory_versions left_version ON left_version.id = left_memory.current_version_id
                 JOIN memories right_memory ON right_memory.id = c.conflicting_memory_id
                 JOIN memory_versions right_version ON right_version.id = right_memory.current_version_id
                 WHERE c.id = ?1",
                [&conflict_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound { entity: "conflict", id: conflict_id.clone() })?;
        let content = format!(
            "These sources disagree about “{key}”. “{first_title}” records one value, while “{second_title}” records another. Review both originals before resolving; TF0000 has not chosen a winner."
        );
        let artifact = insert_generated_artifact(
            &connection,
            "conflict",
            &conflict_id,
            "conflict_explanation",
            &content,
            "local",
        )?;
        Ok(ConflictExplanation {
            artifact,
            conflict_key: key,
            sources: vec![
                ConflictExplanationSource {
                    memory_id: first_id,
                    title: first_title,
                    content: first_content,
                },
                ConflictExplanationSource {
                    memory_id: second_id,
                    title: second_title,
                    content: second_content,
                },
            ],
        })
    }

    pub fn recommend_context(
        &self,
        input: ContextRecommendationInput,
    ) -> Result<ContextRecommendation> {
        let prompt = required_text("prompt", &input.prompt, 4_000)?;
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let limit = input.limit.unwrap_or(6);
        if !(1..=20).contains(&limit) {
            return Err(CoreError::Validation {
                field: "limit",
                message: "must be between 1 and 20".into(),
            });
        }
        let settings = self.smart_settings()?;
        if settings.mode == "off" || settings.recommendation_mode == "off" {
            return Ok(ContextRecommendation {
                mode: "off".into(),
                requires_confirmation: false,
                message: "Smart context recommendations are off.".into(),
                results: Vec::new(),
            });
        }
        let results = self
            .search(SearchInput {
                query: prompt,
                scope: Some(
                    if project_id.is_some() {
                        "project"
                    } else {
                        "all"
                    }
                    .into(),
                ),
                project_id,
                provider: None,
                date_from: None,
                date_to: None,
                memory_type: None,
                status: Some("active".into()),
                limit: Some(limit),
                offset: None,
            })?
            .results;
        let requires_confirmation = settings.recommendation_mode == "ask";
        Ok(ContextRecommendation {
            mode: settings.recommendation_mode,
            requires_confirmation,
            message: if results.is_empty() {
                "No relevant active memories were found.".into()
            } else if requires_confirmation {
                format!(
                    "Review {} suggested item(s) before attaching.",
                    results.len()
                )
            } else {
                format!(
                    "{} relevant item(s) are ready for automatic attachment.",
                    results.len()
                )
            },
            results,
        })
    }

    pub fn benchmark_local_embeddings(&self) -> Result<SmartBenchmarkReport> {
        let connection = self.connect()?;
        ensure_smart_enabled(&connection)?;
        let mut statement = connection
            .prepare("SELECT content FROM semantic_documents ORDER BY created_at DESC LIMIT 250")?;
        let mut samples = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if samples.is_empty() {
            samples = vec![
                "TF0000 keeps durable context local and source grounded.".into(),
                "Decisions and requirements remain reviewable across providers.".into(),
                "Semantic retrieval is optional and lexical search remains available.".into(),
            ];
        }
        let profiles = [
            ("lite", 64_usize),
            ("balanced", 128_usize),
            ("quality", 256_usize),
        ];
        let mut benchmarks = Vec::new();
        for (profile, dimensions) in profiles {
            let started = Instant::now();
            for sample in &samples {
                let _ = local_embedding(sample, dimensions);
            }
            let elapsed = started.elapsed().as_secs_f64() * 1_000.0;
            benchmarks.push(EmbeddingBenchmark {
                profile: profile.into(),
                dimensions,
                documents: samples.len(),
                elapsed_milliseconds: elapsed,
                documents_per_second: samples.len() as f64 / (elapsed / 1_000.0).max(0.000_001),
                estimated_bytes_per_document: dimensions * std::mem::size_of::<f32>(),
            });
        }
        Ok(SmartBenchmarkReport {
            device: "CPU".into(),
            model_family: LOCAL_EMBEDDING_MODEL.into(),
            recommended_profile: "balanced".into(),
            benchmarks,
        })
    }

    pub fn map_workspace(&self, input: MapWorkspaceInput) -> Result<WorkspaceMapping> {
        let workspace_uri = validated_workspace_uri(&input.workspace_uri)?;
        let repository_root = required_text("repositoryRoot", &input.repository_root, 32_000)?;
        let project_id = uuid("projectId", &input.project_id)?;
        let connection = self.connect()?;
        ensure_exists(&connection, "projects", &project_id, "project")?;
        let timestamp = now();
        connection.execute(
            "INSERT INTO workspace_mappings
             (workspace_uri, repository_root, project_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(workspace_uri) DO UPDATE SET
               repository_root = excluded.repository_root,
               project_id = excluded.project_id,
               updated_at = excluded.updated_at",
            params![workspace_uri, repository_root, project_id, timestamp],
        )?;
        get_workspace_mapping(&connection, &workspace_uri)?.ok_or_else(|| CoreError::NotFound {
            entity: "workspace mapping",
            id: workspace_uri,
        })
    }

    pub fn workspace_mapping(&self, workspace_uri: &str) -> Result<Option<WorkspaceMapping>> {
        let workspace_uri = validated_workspace_uri(workspace_uri)?;
        get_workspace_mapping(&self.connect()?, &workspace_uri)
    }

    pub fn save_code_reference(&self, input: SaveCodeReferenceInput) -> Result<CodeReference> {
        let workspace_uri = validated_workspace_uri(&input.workspace_uri)?;
        let relative_path = validated_relative_path(&input.relative_path)?;
        let language = optional_text("language", &input.language, 80)?;
        let content = required_text("content", &input.content, 1_000_000)?;
        let (start_line, end_line) = validated_line_range(input.start_line, input.end_line)?;
        let connection = self.connect()?;
        let mapping = get_workspace_mapping(&connection, &workspace_uri)?.ok_or_else(|| {
            CoreError::Validation {
                field: "workspaceUri",
                message: "workspace must be mapped to a TF0000 project first".into(),
            }
        })?;
        let content_hash = stable_hash(&content);
        if let Some(existing) = connection
            .query_row(
                "SELECT id, project_id, workspace_uri, repository_root, relative_path, language,
                        start_line, end_line, content, content_hash, created_at
                 FROM code_references
                 WHERE project_id = ?1 AND relative_path = ?2 AND start_line IS ?3
                   AND end_line IS ?4 AND content_hash = ?5",
                params![
                    mapping.project_id,
                    relative_path,
                    start_line,
                    end_line,
                    content_hash
                ],
                map_code_reference,
            )
            .optional()?
        {
            return Ok(existing);
        }
        let reference = CodeReference {
            id: Uuid::new_v4().to_string(),
            project_id: mapping.project_id,
            workspace_uri,
            repository_root: mapping.repository_root,
            relative_path,
            language: if language.is_empty() {
                "text".into()
            } else {
                language
            },
            start_line,
            end_line,
            content,
            content_hash,
            created_at: now(),
        };
        connection.execute(
            "INSERT INTO code_references
             (id, project_id, workspace_uri, repository_root, relative_path, language,
              start_line, end_line, content, content_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                reference.id,
                reference.project_id,
                reference.workspace_uri,
                reference.repository_root,
                reference.relative_path,
                reference.language,
                reference.start_line,
                reference.end_line,
                reference.content,
                reference.content_hash,
                reference.created_at,
            ],
        )?;
        Ok(reference)
    }

    pub fn list_code_references(&self, project_id: &str) -> Result<Vec<CodeReference>> {
        let project_id = uuid("projectId", project_id)?;
        let connection = self.connect()?;
        ensure_exists(&connection, "projects", &project_id, "project")?;
        let mut statement = connection.prepare(
            "SELECT id, project_id, workspace_uri, repository_root, relative_path, language,
                    start_line, end_line, content, content_hash, created_at
             FROM code_references WHERE project_id = ?1
             ORDER BY created_at DESC, relative_path COLLATE NOCASE LIMIT 500",
        )?;
        let rows = statement.query_map([project_id], map_code_reference)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn get_project_context(&self, input: ProjectContextInput) -> Result<ProjectContext> {
        let project_id = uuid("projectId", &input.project_id)?;
        let maximum_characters = input.maximum_characters.unwrap_or(24_000);
        if !(1_000..=200_000).contains(&maximum_characters) {
            return Err(CoreError::Validation {
                field: "maximumCharacters",
                message: "must be between 1000 and 200000".into(),
            });
        }
        let connection = self.connect()?;
        let project_name: String = connection
            .query_row(
                "SELECT name FROM projects WHERE id = ?1 AND archived_at IS NULL",
                [&project_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound {
                entity: "active project",
                id: project_id.clone(),
            })?;
        let memories = list_active_project_memories(&connection, &project_id)?;
        let references = list_recent_code_references(&connection, &project_id, 100)?;
        let mut text = format!("# TF0000 project context: {project_name}\n\n");
        let mut memory_count = 0;
        let mut code_reference_count = 0;
        for memory in &memories {
            let block = format!(
                "## {} [{} · {}]\n{}\n\n",
                memory.title, memory.memory_type, memory.authority, memory.current_content
            );
            if text.chars().count() + block.chars().count() > maximum_characters {
                break;
            }
            text.push_str(&block);
            memory_count += 1;
        }
        if !references.is_empty() {
            let heading = "## Code references\n\n";
            if text.chars().count() + heading.len() <= maximum_characters {
                text.push_str(heading);
            }
        }
        for reference in &references {
            let lines = match (reference.start_line, reference.end_line) {
                (Some(start), Some(end)) => format!(" lines {start}-{end}"),
                _ => String::new(),
            };
            let block = format!(
                "### {}{}\n```{}\n{}\n```\n\n",
                reference.relative_path, lines, reference.language, reference.content
            );
            if text.chars().count() + block.chars().count() > maximum_characters {
                break;
            }
            text.push_str(&block);
            code_reference_count += 1;
        }
        let character_count = text.chars().count();
        Ok(ProjectContext {
            project_id,
            project_name,
            text,
            memory_count,
            code_reference_count,
            character_count,
            estimated_tokens: character_count.div_ceil(4),
        })
    }

    pub fn search_decisions(&self, input: SearchDecisionsInput) -> Result<SearchResponse> {
        let project_id = uuid("projectId", &input.project_id)?;
        self.search(SearchInput {
            query: input.query,
            scope: Some("project".into()),
            project_id: Some(project_id),
            provider: None,
            date_from: None,
            date_to: None,
            memory_type: Some("decision".into()),
            status: Some("active".into()),
            limit: Some(input.limit.unwrap_or(20).min(50)),
            offset: None,
        })
    }

    pub fn search_memories(&self, input: SearchMemoriesInput) -> Result<SearchResponse> {
        let project_id = uuid("projectId", &input.project_id)?;
        self.search(SearchInput {
            query: input.query,
            scope: Some("project".into()),
            project_id: Some(project_id),
            provider: None,
            date_from: None,
            date_to: None,
            memory_type: input.memory_type,
            status: Some(input.status.unwrap_or_else(|| "active".into())),
            limit: Some(input.limit.unwrap_or(20).min(50)),
            offset: None,
        })
    }

    pub fn search_chats(&self, input: SearchChatsInput) -> Result<SearchResponse> {
        let search = validate_search_input(SearchInput {
            query: input.query,
            scope: Some("project".into()),
            project_id: Some(uuid("projectId", &input.project_id)?),
            provider: input.provider,
            date_from: input.date_from,
            date_to: input.date_to,
            memory_type: None,
            status: None,
            limit: Some(input.limit.unwrap_or(20).min(50)),
            offset: None,
        })?;
        let connection = self.connect()?;
        ensure_optional_exists(
            &connection,
            "projects",
            search.project_id.as_deref(),
            "project",
        )?;
        let fetch_limit = (search.limit + search.offset).min(500) as i64;
        let mut results = Vec::new();
        search_messages(&connection, &search, fetch_limit, &mut results)?;
        search_fragments(&connection, &search, fetch_limit, &mut results)?;
        results.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| right.created_at.cmp(&left.created_at))
                .then_with(|| left.result_type.cmp(&right.result_type))
                .then_with(|| left.id.cmp(&right.id))
        });
        let results = results
            .into_iter()
            .skip(search.offset)
            .take(search.limit)
            .collect::<Vec<_>>();
        Ok(SearchResponse {
            query: search.query,
            result_count: results.len(),
            results,
        })
    }

    pub fn get_decisions(&self, input: GetDecisionsInput) -> Result<SearchResponse> {
        let project_id = uuid("projectId", &input.project_id)?;
        if let Some(query) = input.query.filter(|value| !value.trim().is_empty()) {
            return self.search_decisions(SearchDecisionsInput {
                project_id,
                query,
                limit: input.limit,
            });
        }
        let limit = input.limit.unwrap_or(20).clamp(1, 50);
        let connection = self.connect()?;
        ensure_exists(&connection, "projects", &project_id, "project")?;
        let memories = list_active_project_memories(&connection, &project_id)?;
        let results = memories
            .into_iter()
            .filter(|memory| memory.memory_type == "decision")
            .take(limit)
            .map(|memory| SearchResult {
                result_type: "memory".into(),
                id: memory.current_version_id.clone(),
                parent_id: Some(memory.id),
                project_id: memory.project_id,
                title: memory.title,
                excerpt: memory.current_content,
                provider: None,
                memory_type: Some(memory.memory_type),
                authority: Some(memory.authority),
                status: Some(memory.status),
                source_url: None,
                created_at: memory.created_at,
                is_current: true,
                score: 0.0,
            })
            .collect::<Vec<_>>();
        Ok(SearchResponse {
            query: String::new(),
            result_count: results.len(),
            results,
        })
    }

    pub fn get_context_pack(&self, pack_id: &str) -> Result<ContextPackDetail> {
        let pack_id = uuid("packId", pack_id)?;
        self.list_context_pack_details()?
            .into_iter()
            .find(|detail| detail.pack.id == pack_id)
            .ok_or(CoreError::NotFound {
                entity: "context pack",
                id: pack_id,
            })
    }

    pub fn memory_project_id(&self, memory_id: &str) -> Result<Option<String>> {
        Ok(self.get_memory(memory_id)?.project_id)
    }

    pub fn context_pack_project_id(&self, pack_id: &str) -> Result<Option<String>> {
        Ok(self.get_context_pack(pack_id)?.pack.project_id)
    }

    pub fn register_mcp_client(&self, input: RegisterMcpClientInput) -> Result<McpClient> {
        let client_id = validate_mcp_client_id(&input.client_id)?;
        let display_name = required_text("displayName", &input.display_name, 160)?;
        let timestamp = now();
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO mcp_clients (id, display_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(id) DO UPDATE SET display_name = excluded.display_name,
                                               updated_at = excluded.updated_at",
            params![client_id, display_name, timestamp],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO integration_permissions
             (integration_id, scope_type, scope_id, read_allowed, write_allowed)
             VALUES (?1, 'global', '*', 1, 0)",
            [&client_id],
        )?;
        transaction.commit()?;
        self.get_mcp_client(&client_id)
    }

    pub fn get_mcp_client(&self, client_id: &str) -> Result<McpClient> {
        let client_id = validate_mcp_client_id(client_id)?;
        self.connect()?
            .query_row(
                "SELECT id, display_name, created_at, updated_at FROM mcp_clients WHERE id = ?1",
                [&client_id],
                |row| {
                    Ok(McpClient {
                        id: row.get(0)?,
                        display_name: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                    })
                },
            )
            .optional()?
            .ok_or(CoreError::NotFound {
                entity: "MCP client",
                id: client_id,
            })
    }

    pub fn mcp_permission(
        &self,
        client_id: &str,
        project_id: Option<&str>,
    ) -> Result<McpPermission> {
        let client_id = validate_mcp_client_id(client_id)?;
        self.get_mcp_client(&client_id)?;
        let project_id = optional_uuid("projectId", project_id)?;
        let connection = self.connect()?;
        ensure_optional_exists(&connection, "projects", project_id.as_deref(), "project")?;
        let project_permission = if let Some(project) = project_id.as_deref() {
            connection
                .query_row(
                    "SELECT read_allowed, write_allowed FROM integration_permissions
                     WHERE integration_id = ?1 AND scope_type = 'project' AND scope_id = ?2",
                    params![client_id, project],
                    |row| Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?)),
                )
                .optional()?
        } else {
            None
        };
        let global_permission = if project_permission.is_none() {
            connection
                .query_row(
                    "SELECT read_allowed, write_allowed FROM integration_permissions
                     WHERE integration_id = ?1 AND scope_type = 'global' AND scope_id = '*'",
                    [&client_id],
                    |row| Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?)),
                )
                .optional()?
        } else {
            None
        };
        let permission = project_permission
            .or(global_permission)
            .unwrap_or((true, false));
        Ok(McpPermission {
            client_id,
            project_id,
            read_allowed: permission.0,
            candidate_write_allowed: permission.1,
        })
    }

    pub fn set_mcp_permission(&self, input: SetMcpPermissionInput) -> Result<McpPermission> {
        let client_id = validate_mcp_client_id(&input.client_id)?;
        self.get_mcp_client(&client_id)?;
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let connection = self.connect()?;
        ensure_optional_exists(&connection, "projects", project_id.as_deref(), "project")?;
        let (scope_type, scope_id) = project_id
            .as_deref()
            .map(|id| ("project", id))
            .unwrap_or(("global", "*"));
        connection.execute(
            "INSERT INTO integration_permissions
             (integration_id, scope_type, scope_id, read_allowed, write_allowed)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(integration_id, scope_type, scope_id)
             DO UPDATE SET read_allowed = excluded.read_allowed,
                           write_allowed = excluded.write_allowed",
            params![
                client_id,
                scope_type,
                scope_id,
                input.read_allowed,
                input.candidate_write_allowed
            ],
        )?;
        self.mcp_permission(&client_id, project_id.as_deref())
    }

    pub fn record_mcp_audit(&self, input: RecordMcpAuditInput) -> Result<McpAuditEntry> {
        let client_id = validate_mcp_client_id(&input.client_id)?;
        self.get_mcp_client(&client_id)?;
        let tool_name = choice(
            "toolName",
            &input.tool_name,
            &[
                "search_memory",
                "get_memory",
                "get_project_context",
                "get_decisions",
                "get_context_pack",
                "search_chats",
                "save_memory_candidate",
            ],
        )?;
        let access_type = choice(
            "accessType",
            &input.access_type,
            &["read", "candidate_write"],
        )?;
        let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
        let entity_type = input
            .entity_type
            .as_deref()
            .map(|value| optional_text("entityType", value, 80))
            .transpose()?;
        let entity_id = input
            .entity_id
            .as_deref()
            .map(|value| optional_text("entityId", value, 200))
            .transpose()?;
        let outcome = choice("outcome", &input.outcome, &["allowed", "denied", "error"])?;
        let entry = McpAuditEntry {
            id: Uuid::new_v4().to_string(),
            client_id,
            tool_name,
            access_type,
            project_id,
            entity_type,
            entity_id,
            outcome,
            occurred_at: now(),
        };
        self.connect()?.execute(
            "INSERT INTO mcp_audit_log
             (id, client_id, tool_name, access_type, project_id, entity_type, entity_id,
              outcome, occurred_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id,
                entry.client_id,
                entry.tool_name,
                entry.access_type,
                entry.project_id,
                entry.entity_type,
                entry.entity_id,
                entry.outcome,
                entry.occurred_at
            ],
        )?;
        Ok(entry)
    }

    pub fn mcp_audit_log(&self, client_id: &str, limit: usize) -> Result<Vec<McpAuditEntry>> {
        let client_id = validate_mcp_client_id(client_id)?;
        self.get_mcp_client(&client_id)?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, client_id, tool_name, access_type, project_id, entity_type, entity_id,
                    outcome, occurred_at
             FROM mcp_audit_log WHERE client_id = ?1
             ORDER BY occurred_at DESC, id DESC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![client_id, limit.clamp(1, 500) as i64], |row| {
            Ok(McpAuditEntry {
                id: row.get(0)?,
                client_id: row.get(1)?,
                tool_name: row.get(2)?,
                access_type: row.get(3)?,
                project_id: row.get(4)?,
                entity_type: row.get(5)?,
                entity_id: row.get(6)?,
                outcome: row.get(7)?,
                occurred_at: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_agent_write_candidate(
        &self,
        input: CreateAgentWriteCandidateInput,
    ) -> Result<AgentWriteCandidate> {
        let project_id = uuid("projectId", &input.project_id)?;
        let requested_by = required_text("requestedBy", &input.requested_by, 160)?;
        let memory_type = choice("memoryType", &input.memory_type, MEMORY_TYPES)?;
        let title = required_text("title", &input.title, 160)?;
        let content = required_text("content", &input.content, 1_000_000)?;
        let connection = self.connect()?;
        ensure_exists(&connection, "projects", &project_id, "project")?;
        let candidate = AgentWriteCandidate {
            id: Uuid::new_v4().to_string(),
            project_id,
            requested_by,
            memory_type,
            title,
            content,
            status: "pending".into(),
            created_at: now(),
            reviewed_at: None,
            memory_id: None,
        };
        connection.execute(
            "INSERT INTO agent_write_candidates
             (id, project_id, requested_by, memory_type, title, content, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7)",
            params![
                candidate.id,
                candidate.project_id,
                candidate.requested_by,
                candidate.memory_type,
                candidate.title,
                candidate.content,
                candidate.created_at,
            ],
        )?;
        Ok(candidate)
    }

    pub fn agent_write_candidates(&self, status: Option<&str>) -> Result<Vec<AgentWriteCandidate>> {
        let status = status
            .map(|value| choice("status", value, &["pending", "accepted", "rejected"]))
            .transpose()?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, project_id, requested_by, memory_type, title, content, status,
                    created_at, reviewed_at, memory_id
             FROM agent_write_candidates WHERE (?1 IS NULL OR status = ?1)
             ORDER BY created_at DESC, id DESC LIMIT 500",
        )?;
        let rows = statement.query_map([status], map_agent_write_candidate)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn review_agent_write_candidate(
        &self,
        input: ReviewAgentWriteCandidateInput,
    ) -> Result<AgentWriteCandidate> {
        let candidate_id = uuid("candidateId", &input.candidate_id)?;
        let action = choice("action", &input.action, &["accept", "reject"])?;
        if !input.confirmed {
            return Err(CoreError::Validation {
                field: "confirmed",
                message: "explicit user confirmation is required".into(),
            });
        }
        let memory_space_id = optional_uuid("memorySpaceId", input.memory_space_id.as_deref())?;
        let mut connection = self.connect()?;
        let candidate = get_agent_write_candidate(&connection, &candidate_id)?;
        if candidate.status != "pending" {
            return Err(CoreError::Validation {
                field: "candidateId",
                message: "candidate has already been reviewed".into(),
            });
        }
        let transaction = connection.transaction()?;
        let mut memory_id = None;
        if action == "accept" {
            validate_import_destination(
                &transaction,
                Some(&candidate.project_id),
                memory_space_id.as_deref(),
            )?;
            let next_memory_id = Uuid::new_v4().to_string();
            let version_id = Uuid::new_v4().to_string();
            let timestamp = now();
            transaction.execute(
                "INSERT INTO memories
                 (id, project_id, type, authority, status, title, current_version_id, created_at)
                 VALUES (?1, ?2, ?3, 'ai_suggestion', 'draft', ?4, NULL, ?5)",
                params![
                    next_memory_id,
                    candidate.project_id,
                    candidate.memory_type,
                    candidate.title,
                    timestamp
                ],
            )?;
            transaction.execute(
                "INSERT INTO memory_versions
                 (id, memory_id, content, change_type, created_at)
                 VALUES (?1, ?2, ?3, 'create', ?4)",
                params![version_id, next_memory_id, candidate.content, timestamp],
            )?;
            transaction.execute(
                "UPDATE memories SET current_version_id = ?1 WHERE id = ?2",
                params![version_id, next_memory_id],
            )?;
            if let Some(space_id) = memory_space_id {
                transaction.execute(
                    "INSERT INTO memory_space_links (memory_id, memory_space_id) VALUES (?1, ?2)",
                    params![next_memory_id, space_id],
                )?;
            }
            memory_id = Some(next_memory_id);
        }
        let reviewed_at = now();
        transaction.execute(
            "UPDATE agent_write_candidates
             SET status = ?1, reviewed_at = ?2, memory_id = ?3 WHERE id = ?4",
            params![
                if action == "accept" {
                    "accepted"
                } else {
                    "rejected"
                },
                reviewed_at,
                memory_id,
                candidate_id,
            ],
        )?;
        transaction.commit()?;
        get_agent_write_candidate(&self.connect()?, &candidate_id)
    }

    pub fn export_portable_workspace(
        &self,
        input: ExportPortableWorkspaceInput,
    ) -> Result<PortableWorkspace> {
        let source_device_id = required_text("sourceDeviceId", &input.source_device_id, 128)?;
        if input.project_ids.is_empty() && !input.include_global {
            return Err(CoreError::Validation {
                field: "projectIds",
                message: "select at least one project or include global context".into(),
            });
        }
        let selected = input
            .project_ids
            .iter()
            .map(|id| uuid("projectId", id))
            .collect::<Result<HashSet<_>>>()?;
        let snapshot = self.snapshot()?;
        let known = snapshot
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect::<HashSet<_>>();
        if let Some(missing) = selected.iter().find(|id| !known.contains(*id)) {
            return Err(CoreError::NotFound {
                entity: "project",
                id: missing.clone(),
            });
        }
        let projects = snapshot
            .projects
            .into_iter()
            .filter(|project| selected.contains(&project.id))
            .collect();
        let memory_spaces = snapshot
            .memory_spaces
            .into_iter()
            .filter(|space| {
                space
                    .project_id
                    .as_ref()
                    .is_some_and(|id| selected.contains(id))
                    || (input.include_global && space.project_id.is_none())
            })
            .collect::<Vec<_>>();
        let allowed_space_ids = memory_spaces
            .iter()
            .map(|space| space.id.clone())
            .collect::<HashSet<_>>();
        let connection = self.connect()?;
        let mut memories = Vec::new();
        for memory in snapshot.memories.into_iter().filter(|memory| {
            memory
                .project_id
                .as_ref()
                .is_some_and(|id| selected.contains(id))
                || (input.include_global && memory.project_id.is_none())
        }) {
            let history = self.memory_history(&memory.id)?;
            let mut statement = connection.prepare("SELECT memory_space_id FROM memory_space_links WHERE memory_id = ?1 ORDER BY memory_space_id")?;
            let memory_space_ids = statement
                .query_map([&memory.id], |row| row.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
                .into_iter()
                .filter(|id| allowed_space_ids.contains(id))
                .collect();
            memories.push(ExportMemory {
                memory,
                versions: history.versions,
                sources: history.sources,
                memory_space_ids,
            });
        }
        let allowed_memory_ids = memories
            .iter()
            .map(|record| record.memory.id.clone())
            .collect::<HashSet<_>>();
        let context_packs = self
            .list_context_pack_details()?
            .into_iter()
            .filter_map(|mut detail| {
                let included = detail
                    .pack
                    .project_id
                    .as_ref()
                    .is_some_and(|id| selected.contains(id))
                    || (input.include_global && detail.pack.project_id.is_none());
                if !included {
                    return None;
                }
                detail
                    .items
                    .retain(|item| allowed_memory_ids.contains(&item.target_id));
                Some(detail)
            })
            .collect();
        Ok(PortableWorkspace {
            format: "tf0000-project/v1".into(),
            schema_version: 1,
            exported_at: now(),
            source_device_id,
            projects,
            memory_spaces,
            memories,
            context_packs,
        })
    }

    pub fn apply_portable_workspace(
        &self,
        input: ApplyPortableWorkspaceInput,
    ) -> Result<SyncApplyResult> {
        validate_portable_workspace(&input.workspace)?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let workspace = input.workspace;
        for project in &workspace.projects {
            transaction.execute(
                "INSERT INTO projects (id, name, description, created_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name, description = excluded.description, archived_at = excluded.archived_at",
                params![project.id, project.name, project.description, project.created_at, project.archived_at],
            )?;
        }
        for space in &workspace.memory_spaces {
            transaction.execute(
                "INSERT INTO memory_spaces (id, project_id, parent_id, name, description, default_scope, created_at, archived_at)
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET project_id = excluded.project_id, parent_id = NULL, name = excluded.name, description = excluded.description, default_scope = excluded.default_scope, archived_at = excluded.archived_at",
                params![space.id, space.project_id, space.name, space.description, space.default_scope, space.created_at, space.archived_at],
            )?;
        }
        for space in &workspace.memory_spaces {
            transaction.execute(
                "UPDATE memory_spaces SET parent_id = ?1 WHERE id = ?2",
                params![space.parent_id, space.id],
            )?;
        }
        let mut result = SyncApplyResult {
            added: 0,
            fast_forwarded: 0,
            unchanged: 0,
            conflicts: 0,
        };
        for remote in &workspace.memories {
            let local_current: Option<String> = transaction
                .query_row(
                    "SELECT current_version_id FROM memories WHERE id = ?1",
                    [&remote.memory.id],
                    |row| row.get(0),
                )
                .optional()?;
            match local_current {
                None => {
                    upsert_portable_memory(&transaction, remote)?;
                    result.added += 1;
                }
                Some(local) if local == remote.memory.current_version_id => result.unchanged += 1,
                Some(local) => {
                    let remote_ids = remote
                        .versions
                        .iter()
                        .map(|version| version.id.as_str())
                        .collect::<HashSet<_>>();
                    let local_has_remote: bool = transaction.query_row(
                        "SELECT EXISTS(SELECT 1 FROM memory_versions WHERE memory_id = ?1 AND id = ?2)",
                        params![remote.memory.id, remote.memory.current_version_id], |row| row.get(0),
                    )?;
                    if remote_ids.contains(local.as_str()) {
                        upsert_portable_memory(&transaction, remote)?;
                        result.fast_forwarded += 1;
                    } else if local_has_remote {
                        result.unchanged += 1;
                    } else {
                        let local_value: String = transaction.query_row(
                            "SELECT v.content FROM memories m JOIN memory_versions v ON v.id = m.current_version_id WHERE m.id = ?1",
                            [&remote.memory.id], |row| row.get(0),
                        )?;
                        let inserted = transaction.execute(
                            "INSERT OR IGNORE INTO sync_conflicts (id, entity_type, entity_id, project_id, local_value, remote_value, remote_device_id, status, created_at)
                             VALUES (?1, 'memory', ?2, ?3, ?4, ?5, ?6, 'unresolved', ?7)",
                            params![Uuid::new_v4().to_string(), remote.memory.id, remote.memory.project_id, local_value, serde_json::to_string(remote)?, workspace.source_device_id, now()],
                        )?;
                        if inserted > 0 {
                            result.conflicts += 1;
                        } else {
                            result.unchanged += 1;
                        }
                    }
                }
            }
        }
        for detail in &workspace.context_packs {
            let local_updated: Option<String> = transaction
                .query_row(
                    "SELECT updated_at FROM context_packs WHERE id = ?1",
                    [&detail.pack.id],
                    |row| row.get(0),
                )
                .optional()?;
            if local_updated
                .as_deref()
                .is_none_or(|updated| updated <= detail.pack.updated_at.as_str())
            {
                transaction.execute(
                    "INSERT INTO context_packs (id, project_id, name, description, current_version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(id) DO UPDATE SET project_id = excluded.project_id, name = excluded.name, description = excluded.description, current_version = excluded.current_version, updated_at = excluded.updated_at",
                    params![detail.pack.id, detail.pack.project_id, detail.pack.name, detail.pack.description, detail.pack.current_version, detail.pack.created_at, detail.pack.updated_at],
                )?;
                transaction.execute(
                    "DELETE FROM context_pack_items WHERE pack_id = ?1",
                    [&detail.pack.id],
                )?;
                for item in &detail.items {
                    transaction.execute("INSERT INTO context_pack_items (pack_id, target_type, target_id, ordering, inclusion_mode) VALUES (?1, 'memory', ?2, ?3, ?4)", params![detail.pack.id, item.target_id, item.ordering, item.inclusion_mode])?;
                }
            }
        }
        transaction.commit()?;
        Ok(result)
    }

    pub fn list_sync_conflicts(&self, status: &str) -> Result<Vec<SyncConflict>> {
        let status = choice(
            "status",
            status,
            &["unresolved", "keep_local", "use_remote"],
        )?;
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id, entity_type, entity_id, project_id, local_value, remote_value, remote_device_id, status, created_at, resolved_at FROM sync_conflicts WHERE status = ?1 ORDER BY created_at DESC",
        )?;
        statement
            .query_map([status], |row| {
                Ok(SyncConflict {
                    id: row.get(0)?,
                    entity_type: row.get(1)?,
                    entity_id: row.get(2)?,
                    project_id: row.get(3)?,
                    local_value: row.get(4)?,
                    remote_value: row.get(5)?,
                    remote_device_id: row.get(6)?,
                    status: row.get(7)?,
                    created_at: row.get(8)?,
                    resolved_at: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn resolve_sync_conflict(&self, input: ResolveSyncConflictInput) -> Result<SyncConflict> {
        let conflict_id = uuid("conflictId", &input.conflict_id)?;
        let resolution = choice(
            "resolution",
            &input.resolution,
            &["keep_local", "use_remote"],
        )?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        let (status, remote): (String, String) = transaction
            .query_row(
                "SELECT status, remote_value FROM sync_conflicts WHERE id = ?1",
                [&conflict_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound {
                entity: "sync conflict",
                id: conflict_id.clone(),
            })?;
        if status != "unresolved" {
            return Err(CoreError::Validation {
                field: "conflictId",
                message: "conflict has already been resolved".into(),
            });
        }
        if resolution == "use_remote" {
            let record: ExportMemory = serde_json::from_str(&remote)?;
            upsert_portable_memory(&transaction, &record)?;
        }
        transaction.execute(
            "UPDATE sync_conflicts SET status = ?1, resolved_at = ?2 WHERE id = ?3",
            params![resolution, now(), conflict_id],
        )?;
        transaction.commit()?;
        let connection = self.connect()?;
        connection.query_row(
            "SELECT id, entity_type, entity_id, project_id, local_value, remote_value, remote_device_id, status, created_at, resolved_at FROM sync_conflicts WHERE id = ?1",
            [&conflict_id], |row| Ok(SyncConflict { id: row.get(0)?, entity_type: row.get(1)?, entity_id: row.get(2)?, project_id: row.get(3)?, local_value: row.get(4)?, remote_value: row.get(5)?, remote_device_id: row.get(6)?, status: row.get(7)?, created_at: row.get(8)?, resolved_at: row.get(9)? }),
        ).map_err(Into::into)
    }

    pub fn export_bundle(&self) -> Result<ExportBundle> {
        let snapshot = self.snapshot()?;
        let mut memories = Vec::with_capacity(snapshot.memories.len());
        let connection = self.connect()?;
        for memory in snapshot.memories {
            let history = self.memory_history(&memory.id)?;
            let mut statement = connection.prepare(
                "SELECT memory_space_id FROM memory_space_links WHERE memory_id = ?1 ORDER BY memory_space_id",
            )?;
            let space_rows = statement.query_map([&memory.id], |row| row.get(0))?;
            let memory_space_ids = space_rows.collect::<std::result::Result<Vec<String>, _>>()?;
            memories.push(ExportMemory {
                memory,
                versions: history.versions,
                sources: history.sources,
                memory_space_ids,
            });
        }
        Ok(ExportBundle {
            format: "tf0000-context/v1".into(),
            schema_version: SCHEMA_VERSION,
            exported_at: now(),
            projects: snapshot.projects,
            memory_spaces: snapshot.memory_spaces,
            memories,
            context_packs: snapshot.context_packs,
        })
    }

    pub fn export_json(&self, destination: impl AsRef<Path>) -> Result<PathBuf> {
        let destination = prepare_destination_with_extension(destination, &["json"])?;
        let bytes = serde_json::to_vec_pretty(&self.export_bundle()?)?;
        fs::write(&destination, bytes)?;
        Ok(destination)
    }

    pub fn export_markdown(&self, destination: impl AsRef<Path>) -> Result<PathBuf> {
        let destination = prepare_destination_with_extension(destination, &["md", "markdown"])?;
        let bundle = self.export_bundle()?;
        let mut output = format!(
            "# TF0000 Context Export\n\nExported: {}\n\n",
            bundle.exported_at
        );
        for project in &bundle.projects {
            output.push_str(&format!(
                "## Project: {}\n\n{}\n\n",
                project.name, project.description
            ));
            for memory in bundle
                .memories
                .iter()
                .filter(|item| item.memory.project_id.as_ref() == Some(&project.id))
            {
                output.push_str(&format!(
                    "### {}\n\n- Type: `{}`\n- Authority: `{}`\n- Status: `{}`\n\n{}\n\n",
                    memory.memory.title,
                    memory.memory.memory_type,
                    memory.memory.authority,
                    memory.memory.status,
                    memory.memory.current_content
                ));
            }
        }
        let global_memories = bundle
            .memories
            .iter()
            .filter(|item| item.memory.project_id.is_none())
            .collect::<Vec<_>>();
        if !global_memories.is_empty() {
            output.push_str("## Global memories\n\n");
            for memory in global_memories {
                output.push_str(&format!(
                    "### {}\n\n{}\n\n",
                    memory.memory.title, memory.memory.current_content
                ));
            }
        }
        fs::write(&destination, output)?;
        Ok(destination)
    }

    pub fn backup(&self, destination: impl AsRef<Path>) -> Result<PathBuf> {
        let destination = prepare_destination_with_extension(destination, &["db"])?;
        let source = self.connect()?;
        let mut target = Connection::open(&destination)?;
        let backup = Backup::new(&source, &mut target)?;
        backup.run_to_completion(128, Duration::from_millis(5), None)?;
        drop(backup);
        target.close().map_err(|(_, error)| error)?;
        Ok(destination)
    }
}

fn validate_portable_workspace(workspace: &PortableWorkspace) -> Result<()> {
    if workspace.format != "tf0000-project/v1" || workspace.schema_version != 1 {
        return Err(CoreError::Validation {
            field: "format",
            message: "unsupported portable workspace format".into(),
        });
    }
    required_text("sourceDeviceId", &workspace.source_device_id, 128)?;
    if workspace.projects.len() > 10_000
        || workspace.memory_spaces.len() > 50_000
        || workspace.memories.len() > 100_000
        || workspace.context_packs.len() > 50_000
    {
        return Err(CoreError::Validation {
            field: "workspace",
            message: "portable workspace exceeds item limits".into(),
        });
    }
    let mut project_ids = HashSet::new();
    for project in &workspace.projects {
        project_ids.insert(uuid("project.id", &project.id)?);
        required_text("project.name", &project.name, 160)?;
        optional_text("project.description", &project.description, 10_000)?;
    }
    let mut space_ids = HashSet::new();
    for space in &workspace.memory_spaces {
        space_ids.insert(uuid("memorySpace.id", &space.id)?);
        if let Some(project_id) = &space.project_id {
            let project_id = uuid("memorySpace.projectId", project_id)?;
            if !project_ids.contains(&project_id) {
                return Err(CoreError::Validation {
                    field: "memorySpace.projectId",
                    message: "references a project outside this portable workspace".into(),
                });
            }
        }
        if let Some(parent_id) = &space.parent_id {
            uuid("memorySpace.parentId", parent_id)?;
        }
        required_text("memorySpace.name", &space.name, 160)?;
        choice("memorySpace.defaultScope", &space.default_scope, SCOPES)?;
    }
    for space in &workspace.memory_spaces {
        if let Some(parent_id) = &space.parent_id
            && !space_ids.contains(parent_id)
        {
            return Err(CoreError::Validation {
                field: "memorySpace.parentId",
                message: "references an unknown space".into(),
            });
        }
    }
    let mut memory_ids = HashSet::new();
    for record in &workspace.memories {
        let memory = &record.memory;
        memory_ids.insert(uuid("memory.id", &memory.id)?);
        if let Some(project_id) = &memory.project_id {
            let project_id = uuid("memory.projectId", project_id)?;
            if !project_ids.contains(&project_id) {
                return Err(CoreError::Validation {
                    field: "memory.projectId",
                    message: "references a project outside this portable workspace".into(),
                });
            }
        }
        choice("memory.type", &memory.memory_type, MEMORY_TYPES)?;
        choice("memory.authority", &memory.authority, AUTHORITIES)?;
        choice("memory.status", &memory.status, STATUSES)?;
        required_text("memory.title", &memory.title, 160)?;
        let current_version_id = uuid("memory.currentVersionId", &memory.current_version_id)?;
        if record.versions.len() > 100_000 {
            return Err(CoreError::Validation {
                field: "memory.versions",
                message: "too many versions".into(),
            });
        }
        let mut version_ids = HashSet::new();
        for version in &record.versions {
            version_ids.insert(uuid("version.id", &version.id)?);
            if version.memory_id != memory.id {
                return Err(CoreError::Validation {
                    field: "version.memoryId",
                    message: "does not match its memory".into(),
                });
            }
            required_text("version.content", &version.content, 1_000_000)?;
            choice(
                "version.changeType",
                &version.change_type,
                &["create", "add", "merge", "replace", "supersede", "restore"],
            )?;
        }
        for source in &record.sources {
            if !version_ids.contains(&source.memory_version_id) {
                return Err(CoreError::Validation {
                    field: "memory.source.memoryVersionId",
                    message: "references an unknown version".into(),
                });
            }
            required_text("memory.source.sourceType", &source.source_type, 64)?;
            required_text("memory.source.sourceId", &source.source_id, 500)?;
            required_text("memory.source.sourceHash", &source.source_hash, 256)?;
            optional_text("memory.source.sourceLabel", &source.source_label, 1_000)?;
            optional_text(
                "memory.source.sourceExcerpt",
                &source.source_excerpt,
                10_000,
            )?;
        }
        let current = record
            .versions
            .iter()
            .find(|version| version.id == current_version_id)
            .ok_or_else(|| CoreError::Validation {
                field: "memory.currentVersionId",
                message: "current version is missing".into(),
            })?;
        if current.content != memory.current_content {
            return Err(CoreError::Validation {
                field: "memory.currentContent",
                message: "does not match the current version".into(),
            });
        }
        for version in &record.versions {
            if let Some(parent) = &version.supersedes_version_id
                && !version_ids.contains(parent)
            {
                return Err(CoreError::Validation {
                    field: "version.supersedesVersionId",
                    message: "references an unknown version".into(),
                });
            }
        }
        for space_id in &record.memory_space_ids {
            let space_id = uuid("memory.memorySpaceId", space_id)?;
            if !space_ids.contains(&space_id) {
                return Err(CoreError::Validation {
                    field: "memory.memorySpaceId",
                    message: "references an unknown space".into(),
                });
            }
        }
    }
    for detail in &workspace.context_packs {
        uuid("contextPack.id", &detail.pack.id)?;
        if let Some(project_id) = &detail.pack.project_id {
            let project_id = uuid("contextPack.projectId", project_id)?;
            if !project_ids.contains(&project_id) {
                return Err(CoreError::Validation {
                    field: "contextPack.projectId",
                    message: "references a project outside this portable workspace".into(),
                });
            }
        }
        required_text("contextPack.name", &detail.pack.name, 160)?;
        let mut orderings = HashSet::new();
        let mut targets = HashSet::new();
        for item in &detail.items {
            let target_id = uuid("contextPack.item.targetId", &item.target_id)?;
            if !memory_ids.contains(&target_id) {
                return Err(CoreError::Validation {
                    field: "contextPack.item.targetId",
                    message: "references an unknown memory".into(),
                });
            }
            choice(
                "contextPack.item.inclusionMode",
                &item.inclusion_mode,
                &["full", "summary", "reference"],
            )?;
            if item.ordering < 0 || !orderings.insert(item.ordering) || !targets.insert(target_id) {
                return Err(CoreError::Validation {
                    field: "contextPack.items",
                    message: "item ordering and target IDs must be unique".into(),
                });
            }
        }
    }
    Ok(())
}

fn upsert_portable_memory(
    transaction: &rusqlite::Transaction<'_>,
    record: &ExportMemory,
) -> Result<()> {
    let memory = &record.memory;
    transaction.execute(
        "INSERT INTO memories (id, project_id, type, authority, status, title, current_version_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)
         ON CONFLICT(id) DO UPDATE SET project_id = excluded.project_id, type = excluded.type, authority = excluded.authority, status = excluded.status, title = excluded.title",
        params![memory.id, memory.project_id, memory.memory_type, memory.authority, memory.status, memory.title, memory.created_at],
    )?;
    for version in &record.versions {
        let existing: Option<(String, String)> = transaction
            .query_row(
                "SELECT memory_id, content FROM memory_versions WHERE id = ?1",
                [&version.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if existing.as_ref().is_some_and(|(memory_id, content)| {
            memory_id != &memory.id || content != &version.content
        }) {
            return Err(CoreError::Validation {
                field: "version.id",
                message: "an existing immutable version has different content".into(),
            });
        }
        transaction.execute(
            "INSERT OR IGNORE INTO memory_versions (id, memory_id, content, change_type, created_at, supersedes_version_id) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![version.id, memory.id, version.content, version.change_type, version.created_at],
        )?;
    }
    for version in &record.versions {
        transaction.execute(
            "UPDATE memory_versions SET supersedes_version_id = ?1 WHERE id = ?2",
            params![version.supersedes_version_id, version.id],
        )?;
    }
    for source in &record.sources {
        transaction.execute(
            "INSERT OR IGNORE INTO memory_sources (memory_version_id, file_ref, source_role, source_type, source_id, source_hash, source_label, source_excerpt)
             VALUES (?1, ?2, 'supporting', ?3, ?4, ?5, ?6, ?7)",
            params![source.memory_version_id, format!("tf0000-sync:{}:{}", source.source_type, source.source_id), source.source_type, source.source_id, source.source_hash, source.source_label, source.source_excerpt],
        )?;
    }
    transaction.execute(
        "DELETE FROM memory_space_links WHERE memory_id = ?1",
        [&memory.id],
    )?;
    for space_id in &record.memory_space_ids {
        transaction.execute(
            "INSERT OR IGNORE INTO memory_space_links (memory_id, memory_space_id) VALUES (?1, ?2)",
            params![memory.id, space_id],
        )?;
    }
    transaction.execute(
        "UPDATE memories SET current_version_id = ?1 WHERE id = ?2",
        params![memory.current_version_id, memory.id],
    )?;
    Ok(())
}

struct ValidatedSearch {
    query: String,
    fts_query: String,
    scope: String,
    project_id: Option<String>,
    provider: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    memory_type: Option<String>,
    status: Option<String>,
    limit: usize,
    offset: usize,
}

#[derive(Debug)]
struct SemanticDocument {
    entity_type: String,
    entity_id: String,
    parent_id: Option<String>,
    project_id: Option<String>,
    title: String,
    content: String,
    provider: Option<String>,
    memory_type: Option<String>,
    authority: Option<String>,
    status: Option<String>,
    source_url: Option<String>,
    created_at: String,
    is_current: bool,
}

fn read_smart_settings(connection: &Connection) -> Result<SmartSettings> {
    connection
        .query_row(
            "SELECT mode, provider_name, provider_endpoint, provider_model,
                    recommendation_mode, updated_at
             FROM smart_settings WHERE singleton = 1",
            [],
            |row| {
                Ok(SmartSettings {
                    mode: row.get(0)?,
                    provider_name: row.get(1)?,
                    provider_endpoint: row.get(2)?,
                    provider_model: row.get(3)?,
                    recommendation_mode: row.get(4)?,
                    embedding_model: LOCAL_EMBEDDING_MODEL.into(),
                    updated_at: row.get(5)?,
                })
            },
        )
        .map_err(Into::into)
}

fn ensure_smart_enabled(connection: &Connection) -> Result<SmartSettings> {
    let settings = read_smart_settings(connection)?;
    if settings.mode == "off" {
        return Err(CoreError::Validation {
            field: "smartFeatures",
            message: "must be enabled in Local or Provider mode".into(),
        });
    }
    Ok(settings)
}

fn live_semantic_documents(connection: &Connection) -> Result<Vec<SemanticDocument>> {
    let mut statement = connection.prepare(
        "SELECT 'memory', v.id, m.id, m.project_id, m.title, v.content, NULL, m.type,
                m.authority, m.status, NULL, v.created_at, 1
         FROM memories m JOIN memory_versions v ON v.id = m.current_version_id
         UNION ALL
         SELECT 'message', msg.id, msg.conversation_id, c.project_id, c.title, msg.body,
                c.provider, NULL, NULL, NULL, c.url, COALESCE(msg.sent_at, c.captured_at), 1
         FROM messages msg JOIN conversations c ON c.id = msg.conversation_id
         UNION ALL
         SELECT 'fragment', f.id, msg.conversation_id, c.project_id, c.title, f.selected_text,
                c.provider, NULL, NULL, NULL, c.url, c.captured_at, 1
         FROM fragments f
         JOIN messages msg ON msg.id = f.message_id
         JOIN conversations c ON c.id = msg.conversation_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(SemanticDocument {
            entity_type: row.get(0)?,
            entity_id: row.get(1)?,
            parent_id: row.get(2)?,
            project_id: row.get(3)?,
            title: row.get(4)?,
            content: row.get(5)?,
            provider: row.get(6)?,
            memory_type: row.get(7)?,
            authority: row.get(8)?,
            status: row.get(9)?,
            source_url: row.get(10)?,
            created_at: row.get(11)?,
            is_current: row.get(12)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn sync_semantic_index(connection: &Connection) -> Result<usize> {
    let documents = live_semantic_documents(connection)?;
    let mut live_keys = HashSet::with_capacity(documents.len());
    for document in &documents {
        live_keys.insert((document.entity_type.clone(), document.entity_id.clone()));
        let content_hash = stable_hash(&format!("{}\n{}", document.title, document.content));
        let existing_hash = connection
            .query_row(
                "SELECT content_hash FROM semantic_documents
                 WHERE entity_type = ?1 AND entity_id = ?2",
                params![document.entity_type, document.entity_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if existing_hash.as_deref() == Some(&content_hash) {
            continue;
        }
        let embedding = serde_json::to_string(&local_embedding(
            &format!("{}\n{}", document.title, document.content),
            LOCAL_EMBEDDING_DIMENSIONS,
        ))?;
        connection.execute(
            "INSERT INTO semantic_documents
             (entity_type, entity_id, parent_id, project_id, title, content, provider,
              memory_type, authority, status, source_url, created_at, is_current,
              content_hash, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(entity_type, entity_id) DO UPDATE SET
               parent_id = excluded.parent_id, project_id = excluded.project_id,
               title = excluded.title, content = excluded.content, provider = excluded.provider,
               memory_type = excluded.memory_type, authority = excluded.authority,
               status = excluded.status, source_url = excluded.source_url,
               created_at = excluded.created_at, is_current = excluded.is_current,
               content_hash = excluded.content_hash, embedding = excluded.embedding",
            params![
                document.entity_type,
                document.entity_id,
                document.parent_id,
                document.project_id,
                document.title,
                document.content,
                document.provider,
                document.memory_type,
                document.authority,
                document.status,
                document.source_url,
                document.created_at,
                document.is_current,
                content_hash,
                embedding,
            ],
        )?;
    }
    let mut statement =
        connection.prepare("SELECT entity_type, entity_id FROM semantic_documents")?;
    let stored = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for (entity_type, entity_id) in stored {
        if !live_keys.contains(&(entity_type.clone(), entity_id.clone())) {
            connection.execute(
                "DELETE FROM semantic_documents WHERE entity_type = ?1 AND entity_id = ?2",
                params![entity_type, entity_id],
            )?;
        }
    }
    Ok(documents.len())
}

fn semantic_search(
    connection: &Connection,
    search: &ValidatedSearch,
    fetch_limit: usize,
) -> Result<Vec<SearchResult>> {
    let query_embedding = local_embedding(&search.query, LOCAL_EMBEDDING_DIMENSIONS);
    let mut statement = connection.prepare(
        "SELECT entity_type, entity_id, parent_id, project_id, title, content, provider,
                memory_type, authority, status, source_url, created_at, is_current, embedding
         FROM semantic_documents ORDER BY created_at DESC LIMIT 5000",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            SemanticDocument {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                parent_id: row.get(2)?,
                project_id: row.get(3)?,
                title: row.get(4)?,
                content: row.get(5)?,
                provider: row.get(6)?,
                memory_type: row.get(7)?,
                authority: row.get(8)?,
                status: row.get(9)?,
                source_url: row.get(10)?,
                created_at: row.get(11)?,
                is_current: row.get(12)?,
            },
            row.get::<_, String>(13)?,
        ))
    })?;
    let mut results = Vec::new();
    for row in rows {
        let (document, embedding_json) = row?;
        if search.scope == "global" && document.project_id.is_some()
            || search.scope == "project" && document.project_id != search.project_id
            || search.provider.as_ref().is_some_and(|provider| {
                document
                    .provider
                    .as_ref()
                    .is_none_or(|value| !value.eq_ignore_ascii_case(provider))
            })
            || search
                .memory_type
                .as_ref()
                .is_some_and(|value| document.memory_type.as_ref() != Some(value))
            || search
                .status
                .as_ref()
                .is_some_and(|value| document.status.as_ref() != Some(value))
            || search
                .date_from
                .as_ref()
                .is_some_and(|value| &document.created_at < value)
            || search
                .date_to
                .as_ref()
                .is_some_and(|value| &document.created_at > value)
        {
            continue;
        }
        if (search.memory_type.is_some() || search.status.is_some())
            && document.entity_type != "memory"
        {
            continue;
        }
        let Ok(embedding) = serde_json::from_str::<Vec<f32>>(&embedding_json) else {
            continue;
        };
        let similarity = cosine_similarity(&query_embedding, &embedding);
        if similarity < 0.12 {
            continue;
        }
        let authority_boost = match document.authority.as_deref() {
            Some("user_confirmed") => 360.0,
            Some("external_fact") => 280.0,
            Some("ai_suggestion") => 100.0,
            Some(_) => 50.0,
            None => 0.0,
        };
        let status_boost = match document.status.as_deref() {
            Some("active") => 180.0,
            Some("draft") => 40.0,
            Some("superseded" | "rejected") => -160.0,
            Some(_) => -60.0,
            None => 0.0,
        };
        results.push(SearchResult {
            result_type: document.entity_type,
            id: document.entity_id,
            parent_id: document.parent_id,
            project_id: document.project_id,
            title: document.title,
            excerpt: deterministic_summary(&document.content, 320),
            provider: document.provider,
            memory_type: document.memory_type,
            authority: document.authority,
            status: document.status,
            source_url: document.source_url,
            created_at: document.created_at,
            is_current: document.is_current,
            score: similarity * 600.0
                + authority_boost
                + status_boost
                + if document.is_current { 120.0 } else { 0.0 },
        });
    }
    results.sort_by(|left, right| right.score.total_cmp(&left.score));
    results.truncate(fetch_limit.max(20));
    Ok(results)
}

fn local_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut vector = vec![0.0_f32; dimensions];
    let tokens = semantic_tokens(text);
    for (position, token) in tokens.iter().enumerate() {
        add_embedding_feature(&mut vector, token, 1.0);
        if let Some(next) = tokens.get(position + 1) {
            add_embedding_feature(&mut vector, &format!("{token}_{next}"), 0.55);
        }
        let padded = format!("^{token}$");
        for window in padded.as_bytes().windows(3) {
            if let Ok(trigram) = std::str::from_utf8(window) {
                add_embedding_feature(&mut vector, trigram, 0.18);
            }
        }
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

fn semantic_tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.len() > 1)
        .filter(|token| {
            !matches!(
                *token,
                "the"
                    | "and"
                    | "that"
                    | "with"
                    | "from"
                    | "this"
                    | "for"
                    | "are"
                    | "was"
                    | "were"
                    | "have"
                    | "has"
            )
        })
        .take(4_000)
        .map(|token| {
            let stem = token
                .strip_suffix("ing")
                .or_else(|| token.strip_suffix("ed"))
                .or_else(|| token.strip_suffix("es"))
                .or_else(|| token.strip_suffix('s'))
                .filter(|stem| stem.len() >= 3)
                .unwrap_or(token);
            match stem {
                "automobile" | "vehicle" => "car",
                "purchase" | "acquire" => "buy",
                "error" | "defect" | "issue" => "bug",
                "require" | "mandatory" => "must",
                "choice" | "selected" => "decision",
                "client" => "customer",
                "fast" | "rapid" => "quick",
                other => other,
            }
            .to_string()
        })
        .collect()
}

fn add_embedding_feature(vector: &mut [f32], feature: &str, weight: f32) {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in feature.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let index = hash as usize % vector.len();
    let sign = if hash & (1 << 63) == 0 { 1.0 } else { -1.0 };
    vector[index] += sign * weight;
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    if left.len() != right.len() {
        return 0.0;
    }
    left.iter()
        .zip(right)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum::<f64>()
        .clamp(-1.0, 1.0)
}

fn validate_search_input(input: SearchInput) -> Result<ValidatedSearch> {
    let query = required_text("query", &input.query, 500)?;
    let fts_query = fts_query(&query)?;
    let scope = choice(
        "scope",
        input.scope.as_deref().unwrap_or("all"),
        &["all", "global", "project"],
    )?;
    let project_id = optional_uuid("projectId", input.project_id.as_deref())?;
    if scope == "project" && project_id.is_none() {
        return Err(CoreError::Validation {
            field: "projectId",
            message: "is required for project scope".into(),
        });
    }
    let provider = input
        .provider
        .as_deref()
        .map(|value| required_text("provider", value, 80))
        .transpose()?;
    let date_from = input
        .date_from
        .as_deref()
        .map(|value| search_date("dateFrom", value, false))
        .transpose()?;
    let date_to = input
        .date_to
        .as_deref()
        .map(|value| search_date("dateTo", value, true))
        .transpose()?;
    if date_from
        .as_ref()
        .zip(date_to.as_ref())
        .is_some_and(|(from, to)| from > to)
    {
        return Err(CoreError::Validation {
            field: "dateFrom",
            message: "must not be after dateTo".into(),
        });
    }
    let memory_type = input
        .memory_type
        .as_deref()
        .map(|value| choice("memoryType", value, MEMORY_TYPES))
        .transpose()?;
    let status = input
        .status
        .as_deref()
        .map(|value| choice("status", value, STATUSES))
        .transpose()?;
    let limit = input.limit.unwrap_or(40);
    if !(1..=100).contains(&limit) {
        return Err(CoreError::Validation {
            field: "limit",
            message: "must be between 1 and 100".into(),
        });
    }
    let offset = input.offset.unwrap_or(0);
    if offset > 1_000 {
        return Err(CoreError::Validation {
            field: "offset",
            message: "must be 1000 or less".into(),
        });
    }
    Ok(ValidatedSearch {
        query,
        fts_query,
        scope,
        project_id,
        provider,
        date_from,
        date_to,
        memory_type,
        status,
        limit,
        offset,
    })
}

fn search_date(field: &'static str, value: &str, end_of_day: bool) -> Result<String> {
    let value = required_text(field, value, 80)?;
    if value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return Ok(if end_of_day {
            format!("{value}T23:59:59.999999999Z")
        } else {
            format!("{value}T00:00:00Z")
        });
    }
    Ok(value)
}

fn fts_query(query: &str) -> Result<String> {
    let mut seen = HashSet::new();
    let tokens = query
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .filter(|token| seen.insert(token.clone()))
        .take(20)
        .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return Err(CoreError::Validation {
            field: "query",
            message: "must contain at least one letter or number".into(),
        });
    }
    Ok(tokens.join(" AND "))
}

fn scope_matches_sql() -> &'static str {
    "(?2 = 'all' OR (?2 = 'global' AND project_id IS NULL) OR (?2 = 'project' AND project_id = ?3))"
}

fn lexical_score(rank: f64) -> f64 {
    100.0 / (1.0 + rank.abs() * 1_000.0)
}

fn search_messages(
    connection: &Connection,
    search: &ValidatedSearch,
    fetch_limit: i64,
    results: &mut Vec<SearchResult>,
) -> Result<()> {
    let sql = format!(
        "SELECT message_id, conversation_id, project_id, title,
                snippet(messages_fts, 6, '[', ']', ' … ', 28), provider, source_url,
                captured_at, bm25(messages_fts, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 5.0, 0.0, 0.0)
         FROM messages_fts
         WHERE messages_fts MATCH ?1 AND {}
           AND (?4 IS NULL OR provider = ?4 COLLATE NOCASE)
           AND (?5 IS NULL OR captured_at >= ?5)
           AND (?6 IS NULL OR captured_at <= ?6)
         ORDER BY bm25(messages_fts) LIMIT ?7",
        scope_matches_sql()
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params![
            search.fts_query,
            search.scope,
            search.project_id,
            search.provider,
            search.date_from,
            search.date_to,
            fetch_limit
        ],
        |row| {
            let rank: f64 = row.get(8)?;
            Ok(SearchResult {
                result_type: "message".into(),
                id: row.get(0)?,
                parent_id: Some(row.get(1)?),
                project_id: row.get(2)?,
                title: row.get(3)?,
                excerpt: row.get(4)?,
                provider: Some(row.get(5)?),
                memory_type: None,
                authority: None,
                status: None,
                source_url: row.get(6)?,
                created_at: row.get(7)?,
                is_current: true,
                score: 120.0 + lexical_score(rank),
            })
        },
    )?;
    results.extend(rows.collect::<std::result::Result<Vec<_>, _>>()?);
    Ok(())
}

fn search_fragments(
    connection: &Connection,
    search: &ValidatedSearch,
    fetch_limit: i64,
    results: &mut Vec<SearchResult>,
) -> Result<()> {
    let sql = format!(
        "SELECT fragment_id, conversation_id, project_id, title,
                snippet(fragments_fts, 7, '[', ']', ' … ', 28), provider, source_url,
                captured_at, bm25(fragments_fts, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 6.0, 0.0, 0.0)
         FROM fragments_fts
         WHERE fragments_fts MATCH ?1 AND {}
           AND (?4 IS NULL OR provider = ?4 COLLATE NOCASE)
           AND (?5 IS NULL OR captured_at >= ?5)
           AND (?6 IS NULL OR captured_at <= ?6)
         ORDER BY bm25(fragments_fts) LIMIT ?7",
        scope_matches_sql()
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params![
            search.fts_query,
            search.scope,
            search.project_id,
            search.provider,
            search.date_from,
            search.date_to,
            fetch_limit
        ],
        |row| {
            let rank: f64 = row.get(8)?;
            Ok(SearchResult {
                result_type: "fragment".into(),
                id: row.get(0)?,
                parent_id: Some(row.get(1)?),
                project_id: row.get(2)?,
                title: row.get(3)?,
                excerpt: row.get(4)?,
                provider: Some(row.get(5)?),
                memory_type: None,
                authority: None,
                status: None,
                source_url: row.get(6)?,
                created_at: row.get(7)?,
                is_current: true,
                score: 140.0 + lexical_score(rank),
            })
        },
    )?;
    results.extend(rows.collect::<std::result::Result<Vec<_>, _>>()?);
    Ok(())
}

fn search_memory_versions(
    connection: &Connection,
    search: &ValidatedSearch,
    fetch_limit: i64,
    results: &mut Vec<SearchResult>,
) -> Result<()> {
    let sql = format!(
        "SELECT memory_version_id, memory_id, project_id, title,
                snippet(memory_versions_fts, 4, '[', ']', ' … ', 28), memory_type,
                authority, status, created_at, is_current,
                bm25(memory_versions_fts, 0.0, 0.0, 0.0, 3.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0)
         FROM memory_versions_fts
         WHERE memory_versions_fts MATCH ?1 AND {}
           AND (?4 IS NULL OR memory_type = ?4)
           AND (?5 IS NULL OR status = ?5)
           AND (?6 IS NULL OR created_at >= ?6)
           AND (?7 IS NULL OR created_at <= ?7)
         ORDER BY bm25(memory_versions_fts) LIMIT ?8",
        scope_matches_sql()
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params![
            search.fts_query,
            search.scope,
            search.project_id,
            search.memory_type,
            search.status,
            search.date_from,
            search.date_to,
            fetch_limit
        ],
        |row| {
            let authority: String = row.get(6)?;
            let status: String = row.get(7)?;
            let is_current = row.get::<_, String>(9)? == "1";
            let rank: f64 = row.get(10)?;
            let authority_boost = match authority.as_str() {
                "user_confirmed" => 600.0,
                "external_fact" => 480.0,
                "ai_suggestion" => 180.0,
                _ => 80.0,
            };
            let status_boost = match status.as_str() {
                "active" => 300.0,
                "draft" => 80.0,
                "superseded" => -200.0,
                "rejected" => -300.0,
                _ => -100.0,
            };
            Ok(SearchResult {
                result_type: "memory".into(),
                id: row.get(0)?,
                parent_id: Some(row.get(1)?),
                project_id: row.get(2)?,
                title: row.get(3)?,
                excerpt: row.get(4)?,
                provider: None,
                memory_type: Some(row.get(5)?),
                authority: Some(authority),
                status: Some(status),
                source_url: None,
                created_at: row.get(8)?,
                is_current,
                score: lexical_score(rank)
                    + authority_boost
                    + status_boost
                    + if is_current { 250.0 } else { 0.0 },
            })
        },
    )?;
    results.extend(rows.collect::<std::result::Result<Vec<_>, _>>()?);
    Ok(())
}

fn search_context_packs(
    connection: &Connection,
    search: &ValidatedSearch,
    fetch_limit: i64,
    results: &mut Vec<SearchResult>,
) -> Result<()> {
    let sql = format!(
        "SELECT pack_id, project_id, name,
                COALESCE(NULLIF(snippet(context_packs_fts, 4, '[', ']', ' … ', 28), ''), description),
                updated_at, bm25(context_packs_fts, 0.0, 0.0, 3.0, 2.0, 5.0, 0.0)
         FROM context_packs_fts
         WHERE context_packs_fts MATCH ?1 AND {}
           AND (?4 IS NULL OR updated_at >= ?4)
           AND (?5 IS NULL OR updated_at <= ?5)
         ORDER BY bm25(context_packs_fts) LIMIT ?6",
        scope_matches_sql()
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        params![
            search.fts_query,
            search.scope,
            search.project_id,
            search.date_from,
            search.date_to,
            fetch_limit
        ],
        |row| {
            let rank: f64 = row.get(5)?;
            Ok(SearchResult {
                result_type: "context_pack".into(),
                id: row.get(0)?,
                parent_id: None,
                project_id: row.get(1)?,
                title: row.get(2)?,
                excerpt: row.get(3)?,
                provider: None,
                memory_type: None,
                authority: None,
                status: Some("active".into()),
                source_url: None,
                created_at: row.get(4)?,
                is_current: true,
                score: 110.0 + lexical_score(rank),
            })
        },
    )?;
    results.extend(rows.collect::<std::result::Result<Vec<_>, _>>()?);
    Ok(())
}

fn search_code_references(
    connection: &Connection,
    search: &ValidatedSearch,
    fetch_limit: i64,
    results: &mut Vec<SearchResult>,
) -> Result<()> {
    if search.scope == "global" {
        return Ok(());
    }
    let mut statement = connection.prepare(
        "SELECT reference_id, project_id, relative_path,
                snippet(code_references_fts, 4, '[', ']', ' … ', 28), language,
                created_at, bm25(code_references_fts, 0.0, 0.0, 4.0, 0.0, 6.0, 0.0)
         FROM code_references_fts
         WHERE code_references_fts MATCH ?1
           AND (?2 = 'all' OR project_id = ?3)
           AND (?4 IS NULL OR created_at >= ?4)
           AND (?5 IS NULL OR created_at <= ?5)
         ORDER BY bm25(code_references_fts) LIMIT ?6",
    )?;
    let rows = statement.query_map(
        params![
            search.fts_query,
            search.scope,
            search.project_id,
            search.date_from,
            search.date_to,
            fetch_limit,
        ],
        |row| {
            let rank: f64 = row.get(6)?;
            Ok(SearchResult {
                result_type: "code_reference".into(),
                id: row.get(0)?,
                parent_id: None,
                project_id: Some(row.get(1)?),
                title: row.get(2)?,
                excerpt: row.get(3)?,
                provider: Some("vscode".into()),
                memory_type: Some("reference".into()),
                authority: Some("user_confirmed".into()),
                status: Some("active".into()),
                source_url: None,
                created_at: row.get(5)?,
                is_current: true,
                score: 220.0 + lexical_score(rank),
            })
        },
    )?;
    results.extend(rows.collect::<std::result::Result<Vec<_>, _>>()?);
    Ok(())
}

fn resolve_merge_sources(
    connection: &Connection,
    inputs: Vec<MergeSourceInput>,
) -> Result<Vec<ResolvedMergeSource>> {
    if inputs.is_empty() {
        return Err(CoreError::Validation {
            field: "sources",
            message: "at least one source is required".into(),
        });
    }
    inputs
        .into_iter()
        .map(|input| {
            let source_type = choice("sourceType", &input.source_type, MERGE_SOURCE_TYPES)?;
            if source_type == "manual" {
                let content = required_text(
                    "content",
                    input.content.as_deref().unwrap_or(""),
                    1_000_000,
                )?;
                let source_hash = stable_hash(&content);
                let label = optional_text(
                    "label",
                    input.label.as_deref().unwrap_or("Manual note"),
                    160,
                )?;
                return Ok(ResolvedMergeSource {
                    source_type,
                    source_id: format!("manual:{source_hash}"),
                    label: if label.is_empty() {
                        "Manual note".into()
                    } else {
                        label
                    },
                    content,
                    source_hash,
                    duplicate_of: None,
                });
            }

            let source_id = uuid("sourceId", input.source_id.as_deref().unwrap_or(""))?;
            let (label, content, source_hash) = match source_type.as_str() {
                "conversation" => {
                    let (provider, title): (String, String) = connection
                        .query_row(
                            "SELECT provider, title FROM conversations WHERE id = ?1",
                            [&source_id],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .optional()?
                        .ok_or_else(|| CoreError::NotFound {
                            entity: "conversation",
                            id: source_id.clone(),
                        })?;
                    let mut statement = connection.prepare(
                        "SELECT role, body FROM messages WHERE conversation_id = ?1 ORDER BY ordinal",
                    )?;
                    let messages = statement
                        .query_map([&source_id], |row| {
                            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                        })?
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    let content = messages
                        .into_iter()
                        .map(|(role, body)| format!("[{}] {}", role.to_uppercase(), body.trim()))
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    (
                        format!("{} conversation: {title}", provider.to_uppercase()),
                        content.clone(),
                        stable_hash(&content),
                    )
                }
                "message" => connection
                    .query_row(
                        "SELECT c.provider || ' message in ' || c.title, m.body, m.source_hash FROM messages m JOIN conversations c ON c.id = m.conversation_id WHERE m.id = ?1",
                        [&source_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()?
                    .ok_or_else(|| CoreError::NotFound {
                        entity: "message",
                        id: source_id.clone(),
                    })?,
                "fragment" => connection
                    .query_row(
                        "SELECT c.provider || ' fragment in ' || c.title, f.selected_text, f.source_hash FROM fragments f JOIN messages m ON m.id = f.message_id JOIN conversations c ON c.id = m.conversation_id WHERE f.id = ?1",
                        [&source_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()?
                    .ok_or_else(|| CoreError::NotFound {
                        entity: "fragment",
                        id: source_id.clone(),
                    })?,
                "memory" => connection
                    .query_row(
                        "SELECT 'Memory: ' || m.title, v.content, v.id FROM memories m JOIN memory_versions v ON v.id = m.current_version_id WHERE m.id = ?1",
                        [&source_id],
                        |row| {
                            let label: String = row.get(0)?;
                            let content: String = row.get(1)?;
                            Ok((label, content.clone(), stable_hash(&content)))
                        },
                    )
                    .optional()?
                    .ok_or_else(|| CoreError::NotFound {
                        entity: "memory",
                        id: source_id.clone(),
                    })?,
                _ => unreachable!(),
            };
            let content = required_text("sourceContent", &content, 1_000_000)?;
            Ok(ResolvedMergeSource {
                source_type,
                source_id,
                label,
                content,
                source_hash,
                duplicate_of: None,
            })
        })
        .collect()
}

fn mark_duplicate_sources(
    connection: &Connection,
    memory_id: Option<&str>,
    sources: &mut [ResolvedMergeSource],
    before: &str,
) -> Result<()> {
    let mut identities = HashMap::<String, String>::new();
    let mut hashes = HashMap::<String, String>::new();
    if let Some(memory_id) = memory_id {
        let mut statement = connection.prepare(
            "SELECT source_type, source_id, source_hash
             FROM memory_sources
             WHERE memory_version_id IN (SELECT id FROM memory_versions WHERE memory_id = ?1)
               AND source_type IS NOT NULL",
        )?;
        let history = statement
            .query_map([memory_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (source_type, source_id, source_hash) in history {
            identities.insert(
                format!("{source_type}:{source_id}"),
                "memory-history".into(),
            );
            hashes.insert(source_hash, "memory-history".into());
        }
    }
    if !before.trim().is_empty() {
        hashes.insert(stable_hash(before), "current-version".into());
    }
    for source in sources {
        let identity = format!("{}:{}", source.source_type, source.source_id);
        let duplicate = identities
            .get(&identity)
            .or_else(|| hashes.get(&source.source_hash))
            .cloned();
        if let Some(duplicate_of) = duplicate {
            source.duplicate_of = Some(duplicate_of);
        } else {
            identities.insert(identity, source.source_id.clone());
            hashes.insert(source.source_hash.clone(), source.source_id.clone());
        }
    }
    Ok(())
}

fn compose_memory_update(
    action: &str,
    before: &str,
    sources: &[ResolvedMergeSource],
) -> Result<String> {
    let unique = sources
        .iter()
        .filter(|source| source.duplicate_of.is_none())
        .map(|source| source.content.as_str())
        .collect::<Vec<_>>();
    if unique.is_empty() {
        return Err(CoreError::Validation {
            field: "sources",
            message: "all selected sources are duplicates of existing content".into(),
        });
    }
    let output = match action {
        "add" => std::iter::once(before)
            .filter(|value| !value.trim().is_empty())
            .chain(unique)
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n\n"),
        "merge" => merge_paragraphs(before, unique),
        "replace" | "supersede" => unique
            .into_iter()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => unreachable!(),
    };
    required_text("content", &output, 1_000_000)
}

fn merge_paragraphs<'a>(base: &'a str, additions: impl IntoIterator<Item = &'a str>) -> String {
    let mut seen = HashSet::new();
    std::iter::once(base)
        .chain(additions)
        .flat_map(|content| content.split("\n\n"))
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .filter(|paragraph| seen.insert(stable_hash(paragraph)))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn diff_summary(before: &str, after: &str) -> UpdateDiff {
    let mut before_counts = HashMap::<&str, usize>::new();
    let mut after_counts = HashMap::<&str, usize>::new();
    for line in before.lines() {
        *before_counts.entry(line).or_default() += 1;
    }
    for line in after.lines() {
        *after_counts.entry(line).or_default() += 1;
    }
    let added_lines = after_counts
        .iter()
        .map(|(line, count)| count.saturating_sub(*before_counts.get(line).unwrap_or(&0)))
        .sum();
    let removed_lines = before_counts
        .iter()
        .map(|(line, count)| count.saturating_sub(*after_counts.get(line).unwrap_or(&0)))
        .sum();
    UpdateDiff {
        before: before.into(),
        after: after.into(),
        added_lines,
        removed_lines,
    }
}

fn insert_memory_source(
    connection: &Connection,
    version_id: &str,
    source: &ResolvedMergeSource,
) -> Result<()> {
    let (conversation_id, message_id, fragment_id, file_ref): (
        Option<&str>,
        Option<&str>,
        Option<&str>,
        Option<String>,
    ) = match source.source_type.as_str() {
        "conversation" => (Some(&source.source_id), None, None, None),
        "message" => (None, Some(&source.source_id), None, None),
        "fragment" => (None, None, Some(&source.source_id), None),
        _ => (
            None,
            None,
            None,
            Some(format!(
                "tf0000://{}/{}",
                source.source_type, source.source_id
            )),
        ),
    };
    connection.execute(
        "INSERT OR IGNORE INTO memory_sources
         (memory_version_id, conversation_id, message_id, fragment_id, file_ref, source_role,
          source_type, source_id, source_hash, source_label, source_excerpt)
         VALUES (?1, ?2, ?3, ?4, ?5, 'supporting', ?6, ?7, ?8, ?9, ?10)",
        params![
            version_id,
            conversation_id,
            message_id,
            fragment_id,
            file_ref,
            source.source_type,
            source.source_id,
            source.source_hash,
            source.label,
            source.content,
        ],
    )?;
    Ok(())
}

fn copy_memory_sources(
    connection: &Connection,
    from_version: &str,
    to_version: &str,
) -> Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO memory_sources
         (memory_version_id, conversation_id, message_id, fragment_id, file_ref, source_role,
          source_type, source_id, source_hash, source_label, source_excerpt)
         SELECT ?1, conversation_id, message_id, fragment_id, file_ref, source_role,
                source_type, source_id, source_hash, source_label, source_excerpt
         FROM memory_sources WHERE memory_version_id = ?2",
        params![to_version, from_version],
    )?;
    Ok(())
}

fn build_handoff_preview(connection: &Connection, input: &HandoffInput) -> Result<HandoffPreview> {
    let supported_providers = &["chatgpt", "claude", "gemini"];
    let source_provider = choice(
        "sourceProvider",
        &input.source_provider,
        supported_providers,
    )?;
    let destination_provider = choice(
        "destinationProvider",
        &input.destination_provider,
        supported_providers,
    )?;
    if source_provider == destination_provider {
        return Err(CoreError::Validation {
            field: "destinationProvider",
            message: "must differ from the source provider".into(),
        });
    }
    let mode = choice("mode", &input.mode, &["full", "minimal", "custom"])?;
    let external_ref = required_text("sourceExternalRef", &input.source_external_ref, 4_000)?;
    let (conversation_id, project_id, title, source_url): (
        String,
        Option<String>,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT id, project_id, title, url FROM conversations
             WHERE provider = ?1 AND external_ref = ?2",
            params![source_provider, external_ref],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| CoreError::NotFound {
            entity: "source conversation",
            id: format!("{}:{}", source_provider, external_ref),
        })?;

    let (message_limit, include_current_task, include_active_decisions) = match mode.as_str() {
        "full" => (None, true, true),
        "minimal" => (Some(4), true, false),
        "custom" => {
            let count = input.recent_message_count.unwrap_or(8);
            if !(1..=100).contains(&count) {
                return Err(CoreError::Validation {
                    field: "recentMessageCount",
                    message: "must be between 1 and 100".into(),
                });
            }
            (
                Some(count),
                input.include_current_task,
                input.include_active_decisions,
            )
        }
        _ => unreachable!(),
    };

    let messages = list_handoff_messages(connection, &conversation_id, message_limit)?;
    if messages.is_empty() {
        return Err(CoreError::Validation {
            field: "sourceConversation",
            message: "has no captured messages".into(),
        });
    }

    let mut items = Vec::new();
    let message_text = messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            items.push(HandoffItem {
                item_type: "message".into(),
                item_id: message.id.clone(),
                label: format!("{} message {}", message.role, message.ordinal),
            });
            format!(
                "### {}. {}\n\n{}",
                index + 1,
                message.role.to_uppercase(),
                message.body.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let tasks = if include_current_task {
        list_active_memories_by_type(connection, project_id.as_deref(), "task")?
    } else {
        Vec::new()
    };
    let decisions = if include_active_decisions {
        list_active_memories_by_type(connection, project_id.as_deref(), "decision")?
    } else {
        Vec::new()
    };
    let selected_memories = unique_uuid_values("memoryId", &input.memory_ids)?
        .into_iter()
        .map(|id| fetch_memory(connection, &id))
        .collect::<Result<Vec<_>>>()?;
    let pack_ids = unique_uuid_values("contextPackId", &input.context_pack_ids)?;

    let mut sections = vec![format!("## Recent conversation\n\n{message_text}")];
    push_memory_handoff_section(
        &mut sections,
        &mut items,
        "Current task",
        "current_task",
        tasks,
    );
    push_memory_handoff_section(
        &mut sections,
        &mut items,
        "Active decisions",
        "active_decision",
        decisions,
    );
    push_memory_handoff_section(
        &mut sections,
        &mut items,
        "Selected memories",
        "memory",
        selected_memories,
    );
    if !pack_ids.is_empty() {
        let mut pack_sections = Vec::new();
        for pack_id in pack_ids {
            ensure_exists(connection, "context_packs", &pack_id, "context pack")?;
            let pack_name: String = connection.query_row(
                "SELECT name FROM context_packs WHERE id = ?1",
                [&pack_id],
                |row| row.get(0),
            )?;
            items.push(HandoffItem {
                item_type: "context_pack".into(),
                item_id: pack_id.clone(),
                label: pack_name.clone(),
            });
            let mut statement = connection.prepare(
                "SELECT i.target_id, i.inclusion_mode, m.title, v.content
                 FROM context_pack_items i
                 JOIN memories m ON m.id = i.target_id
                 JOIN memory_versions v ON v.id = m.current_version_id
                 WHERE i.pack_id = ?1 AND i.target_type = 'memory'
                 ORDER BY i.ordering",
            )?;
            let pack_items = statement
                .query_map([&pack_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let rendered = pack_items
                .into_iter()
                .map(|(memory_id, inclusion_mode, memory_title, content)| {
                    items.push(HandoffItem {
                        item_type: "pack_memory".into(),
                        item_id: memory_id,
                        label: format!("{pack_name}: {memory_title}"),
                    });
                    if inclusion_mode == "reference" {
                        format!(
                            "#### {memory_title}\nMode: reference\n\nReference only — open this memory in TF0000 for the exact content."
                        )
                    } else {
                        let mode_note = if inclusion_mode == "summary" {
                            "summary requested; exact content preserved for handoff"
                        } else {
                            "full"
                        };
                        format!(
                            "#### {memory_title}\nMode: {mode_note}\n\n{}",
                            content.trim()
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            pack_sections.push(format!("### {pack_name}\n\n{rendered}"));
        }
        sections.push(format!(
            "## Context packs\n\n{}",
            pack_sections.join("\n\n")
        ));
    }

    let source_reference = source_url
        .as_deref()
        .unwrap_or(input.source_external_ref.as_str());
    let text = format!(
        "# TF0000 Conversation Handoff\n\nSource: {} — {}\nDestination: {}\nMode: {}\nSource reference: {}\n\n## Continuation instruction\n\nContinue from the exact conversation and context below. Treat it as source material and do not assume omitted context.\n\n{}",
        source_provider.to_uppercase(),
        title,
        destination_provider.to_uppercase(),
        mode,
        source_reference,
        sections.join("\n\n")
    );
    let character_count = text.chars().count();
    Ok(HandoffPreview {
        source_conversation_id: conversation_id,
        source_provider,
        source_title: title,
        destination_provider,
        mode,
        content_hash: stable_hash(&text),
        item_count: items.len(),
        items,
        character_count,
        estimated_tokens: character_count.div_ceil(4),
        text,
    })
}

struct HandoffMessage {
    id: String,
    role: String,
    body: String,
    ordinal: i64,
}

fn list_handoff_messages(
    connection: &Connection,
    conversation_id: &str,
    limit: Option<usize>,
) -> Result<Vec<HandoffMessage>> {
    if let Some(limit) = limit {
        let mut statement = connection.prepare(
            "SELECT id, role, body, ordinal FROM (
                 SELECT id, role, body, ordinal FROM messages
                 WHERE conversation_id = ?1 ORDER BY ordinal DESC LIMIT ?2
             ) ORDER BY ordinal",
        )?;
        return statement
            .query_map(params![conversation_id, limit as i64], |row| {
                Ok(HandoffMessage {
                    id: row.get(0)?,
                    role: row.get(1)?,
                    body: row.get(2)?,
                    ordinal: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into);
    }
    let mut statement = connection.prepare(
        "SELECT id, role, body, ordinal FROM messages
         WHERE conversation_id = ?1 ORDER BY ordinal",
    )?;
    statement
        .query_map([conversation_id], |row| {
            Ok(HandoffMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                body: row.get(2)?,
                ordinal: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn list_active_memories_by_type(
    connection: &Connection,
    project_id: Option<&str>,
    memory_type: &str,
) -> Result<Vec<Memory>> {
    let mut statement = connection.prepare(
        "SELECT m.id, m.project_id, m.type, m.authority, m.status, m.title,
                m.current_version_id, v.content, m.created_at
         FROM memories m JOIN memory_versions v ON v.id = m.current_version_id
         WHERE m.status = 'active' AND m.type = ?1
           AND (m.project_id IS NULL OR m.project_id = ?2)
         ORDER BY m.title COLLATE NOCASE, m.id",
    )?;
    statement
        .query_map(params![memory_type, project_id], map_memory)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn fetch_memory(connection: &Connection, id: &str) -> Result<Memory> {
    connection
        .query_row(
            "SELECT m.id, m.project_id, m.type, m.authority, m.status, m.title,
                    m.current_version_id, v.content, m.created_at
             FROM memories m JOIN memory_versions v ON v.id = m.current_version_id
             WHERE m.id = ?1",
            [id],
            map_memory,
        )
        .optional()?
        .ok_or_else(|| CoreError::NotFound {
            entity: "memory",
            id: id.to_owned(),
        })
}

fn unique_uuid_values(field: &'static str, values: &[String]) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    values
        .iter()
        .map(|value| uuid(field, value))
        .filter_map(|result| match result {
            Ok(value) if seen.insert(value.clone()) => Some(Ok(value)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn push_memory_handoff_section(
    sections: &mut Vec<String>,
    items: &mut Vec<HandoffItem>,
    heading: &str,
    item_type: &str,
    memories: Vec<Memory>,
) {
    if memories.is_empty() {
        return;
    }
    let rendered = memories
        .into_iter()
        .map(|memory| {
            items.push(HandoffItem {
                item_type: item_type.into(),
                item_id: memory.id,
                label: memory.title.clone(),
            });
            format!("### {}\n\n{}", memory.title, memory.current_content.trim())
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    sections.push(format!("## {heading}\n\n{rendered}"));
}

#[derive(Debug)]
struct RawImportCandidate {
    title: String,
    content: String,
    memory_type: String,
    authority: String,
    status: String,
}

fn parse_import_file(source: &Path) -> Result<(String, String, String, Vec<ImportCandidate>)> {
    if !source.is_absolute() || !source.is_file() {
        return Err(CoreError::Validation {
            field: "sourcePath",
            message: "must be an existing absolute file path".into(),
        });
    }
    let metadata = fs::metadata(source)?;
    if metadata.len() > 16 * 1024 * 1024 {
        return Err(CoreError::Validation {
            field: "sourcePath",
            message: "import files must be 16 MB or smaller".into(),
        });
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let source_format = match extension.as_str() {
        "json" => "json",
        "md" | "markdown" => "markdown",
        "txt" => "text",
        _ => {
            return Err(CoreError::Validation {
                field: "sourcePath",
                message: "must be a .json, .md, .markdown or .txt file".into(),
            });
        }
    };
    let source_label = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Imported file")
        .to_string();
    let text = fs::read_to_string(source)?;
    let source_hash = stable_hash(&text);
    let fallback_title = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Imported context");
    let raw = match source_format {
        "json" => parse_json_import(&text, fallback_title)?,
        "markdown" => parse_markdown_import(&text, fallback_title),
        "text" => vec![RawImportCandidate {
            title: fallback_title.into(),
            content: text,
            memory_type: "reference".into(),
            authority: "external_fact".into(),
            status: "active".into(),
        }],
        _ => unreachable!(),
    };
    if raw.is_empty() {
        return Err(CoreError::Validation {
            field: "sourcePath",
            message: "contains no importable text".into(),
        });
    }
    if raw.len() > 2_000 {
        return Err(CoreError::Validation {
            field: "sourcePath",
            message: "contains more than 2000 importable items".into(),
        });
    }
    let candidates = raw
        .into_iter()
        .map(|item| {
            let title = required_text("title", &truncate_title(&item.title), 160)?;
            let content = required_text("content", &item.content, 1_000_000)?;
            let memory_type = choice("memoryType", &item.memory_type, MEMORY_TYPES)?;
            let authority = choice("authority", &item.authority, AUTHORITIES)?;
            let status = choice("status", &item.status, STATUSES)?;
            Ok(ImportCandidate {
                title,
                content_hash: stable_hash(&content),
                secret_warnings: detect_secret_warnings(&content),
                content,
                memory_type,
                authority,
                status,
                duplicate: false,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((source_label, source_format.into(), source_hash, candidates))
}

fn parse_json_import(text: &str, fallback_title: &str) -> Result<Vec<RawImportCandidate>> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    if value.get("format").and_then(|item| item.as_str()) == Some("tf0000-context/v1") {
        let memories = value
            .get("memories")
            .and_then(|item| item.as_array())
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.get("memory"))
            .filter_map(|memory| {
                let title = memory.get("title")?.as_str()?;
                let content = memory.get("currentContent")?.as_str()?;
                Some(RawImportCandidate {
                    title: title.into(),
                    content: content.into(),
                    memory_type: memory
                        .get("memoryType")
                        .and_then(|item| item.as_str())
                        .unwrap_or("reference")
                        .into(),
                    authority: memory
                        .get("authority")
                        .and_then(|item| item.as_str())
                        .unwrap_or("external_fact")
                        .into(),
                    status: memory
                        .get("status")
                        .and_then(|item| item.as_str())
                        .unwrap_or("active")
                        .into(),
                })
            })
            .collect::<Vec<_>>();
        return Ok(memories);
    }

    let mut candidates = Vec::new();
    collect_json_conversations(&value, fallback_title, 0, &mut candidates);
    if candidates.is_empty() {
        collect_json_records(&value, fallback_title, 0, &mut candidates);
    }
    if candidates.is_empty() {
        candidates.push(RawImportCandidate {
            title: fallback_title.into(),
            content: serde_json::to_string_pretty(&value)?,
            memory_type: "reference".into(),
            authority: "external_fact".into(),
            status: "active".into(),
        });
    }
    Ok(candidates)
}

fn collect_json_conversations(
    value: &serde_json::Value,
    fallback_title: &str,
    depth: usize,
    output: &mut Vec<RawImportCandidate>,
) {
    if depth > 20 || output.len() >= 2_001 {
        return;
    }
    match value {
        serde_json::Value::Object(object) => {
            let message_array = object
                .get("messages")
                .or_else(|| object.get("chat_messages"))
                .and_then(|item| item.as_array());
            if let Some(messages) = message_array {
                let body = render_json_messages(messages.iter());
                if !body.is_empty() {
                    output.push(RawImportCandidate {
                        title: json_title(object).unwrap_or_else(|| fallback_title.into()),
                        content: body,
                        memory_type: "reference".into(),
                        authority: "external_fact".into(),
                        status: "active".into(),
                    });
                    return;
                }
            }
            if let Some(mapping) = object.get("mapping").and_then(|item| item.as_object()) {
                let mut messages = mapping
                    .values()
                    .filter_map(|node| node.get("message"))
                    .filter_map(|message| {
                        json_content(message).map(|content| {
                            (
                                message
                                    .get("create_time")
                                    .and_then(|item| item.as_f64())
                                    .unwrap_or(f64::MAX),
                                message_role(message),
                                content,
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                messages.sort_by(|left, right| left.0.total_cmp(&right.0));
                let body = messages
                    .into_iter()
                    .map(|(_, role, content)| {
                        format!("[{}] {}", role.to_uppercase(), content.trim())
                    })
                    .filter(|item| !item.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if !body.is_empty() {
                    output.push(RawImportCandidate {
                        title: json_title(object).unwrap_or_else(|| fallback_title.into()),
                        content: body,
                        memory_type: "reference".into(),
                        authority: "external_fact".into(),
                        status: "active".into(),
                    });
                    return;
                }
            }
            for nested in object.values() {
                collect_json_conversations(nested, fallback_title, depth + 1, output);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_json_conversations(item, fallback_title, depth + 1, output);
            }
        }
        _ => {}
    }
}

fn collect_json_records(
    value: &serde_json::Value,
    fallback_title: &str,
    depth: usize,
    output: &mut Vec<RawImportCandidate>,
) {
    if depth > 20 || output.len() >= 2_001 {
        return;
    }
    match value {
        serde_json::Value::Object(object) => {
            if let Some(content) = json_content(value) {
                output.push(RawImportCandidate {
                    title: json_title(object).unwrap_or_else(|| fallback_title.into()),
                    content,
                    memory_type: "reference".into(),
                    authority: "external_fact".into(),
                    status: "active".into(),
                });
                return;
            }
            for nested in object.values() {
                collect_json_records(nested, fallback_title, depth + 1, output);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_json_records(item, fallback_title, depth + 1, output);
            }
        }
        _ => {}
    }
}

fn json_title(object: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    ["title", "name", "subject"]
        .iter()
        .find_map(|key| object.get(*key).and_then(|item| item.as_str()))
        .map(str::to_string)
}

fn render_json_messages<'a>(messages: impl Iterator<Item = &'a serde_json::Value>) -> String {
    messages
        .filter_map(|message| {
            json_content(message).map(|content| {
                format!(
                    "[{}] {}",
                    message_role(message).to_uppercase(),
                    content.trim()
                )
            })
        })
        .filter(|item| !item.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn message_role(message: &serde_json::Value) -> &str {
    message
        .get("role")
        .or_else(|| message.get("sender"))
        .and_then(|item| item.as_str())
        .or_else(|| {
            message
                .get("author")
                .and_then(|author| author.get("role"))
                .and_then(|item| item.as_str())
        })
        .unwrap_or("unknown")
}

fn json_content(value: &serde_json::Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.into());
    }
    let object = value.as_object()?;
    for key in ["content", "body", "text"] {
        if let Some(content) = object.get(key) {
            if let Some(text) = content.as_str() {
                return Some(text.into());
            }
            if let Some(parts) = content.as_array() {
                let joined = parts
                    .iter()
                    .filter_map(|part| {
                        part.as_str().map(str::to_string).or_else(|| {
                            part.get("text")
                                .and_then(|item| item.as_str())
                                .map(str::to_string)
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !joined.is_empty() {
                    return Some(joined);
                }
            }
            if let Some(parts) = content.get("parts").and_then(|item| item.as_array()) {
                let joined = parts
                    .iter()
                    .filter_map(|part| part.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                if !joined.is_empty() {
                    return Some(joined);
                }
            }
        }
    }
    None
}

fn parse_markdown_import(text: &str, fallback_title: &str) -> Vec<RawImportCandidate> {
    let mut candidates = Vec::new();
    let mut title = fallback_title.to_string();
    let mut body = Vec::new();
    for line in text.lines() {
        if let Some(heading) = line.trim_start().strip_prefix('#') {
            let heading = heading.trim_start_matches('#').trim();
            if !body.join("\n").trim().is_empty() {
                candidates.push(RawImportCandidate {
                    title: title.clone(),
                    content: body.join("\n"),
                    memory_type: "reference".into(),
                    authority: "external_fact".into(),
                    status: "active".into(),
                });
            }
            title = if heading.is_empty() {
                fallback_title.into()
            } else {
                heading.into()
            };
            body.clear();
        } else {
            body.push(line);
        }
    }
    if !body.join("\n").trim().is_empty() {
        candidates.push(RawImportCandidate {
            title,
            content: body.join("\n"),
            memory_type: "reference".into(),
            authority: "external_fact".into(),
            status: "active".into(),
        });
    }
    candidates
}

fn truncate_title(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= 160 {
        trimmed.into()
    } else {
        trimmed.chars().take(159).collect::<String>() + "…"
    }
}

fn validate_import_destination(
    connection: &Connection,
    project_id: Option<&str>,
    memory_space_id: Option<&str>,
) -> Result<()> {
    ensure_optional_exists(connection, "projects", project_id, "project")?;
    ensure_optional_exists(connection, "memory_spaces", memory_space_id, "memory space")?;
    if let Some(space_id) = memory_space_id {
        let space_project: Option<String> = connection.query_row(
            "SELECT project_id FROM memory_spaces WHERE id = ?1",
            [space_id],
            |row| row.get(0),
        )?;
        if space_project.as_deref() != project_id {
            return Err(CoreError::Validation {
                field: "memorySpaceId",
                message: "memory space must belong to the selected project".into(),
            });
        }
    }
    Ok(())
}

fn mark_import_duplicates(
    connection: &Connection,
    candidates: &mut [ImportCandidate],
) -> Result<()> {
    let mut existing = HashSet::new();
    let mut statement = connection.prepare(
        "SELECT v.content FROM memories m JOIN memory_versions v ON v.id = m.current_version_id",
    )?;
    for content in statement.query_map([], |row| row.get::<_, String>(0))? {
        existing.insert(stable_hash(&content?));
    }
    let mut statement = connection.prepare("SELECT content_hash FROM import_items")?;
    for hash in statement.query_map([], |row| row.get::<_, String>(0))? {
        existing.insert(hash?);
    }
    for candidate in candidates {
        candidate.duplicate = !existing.insert(candidate.content_hash.clone());
    }
    Ok(())
}

fn detect_secret_warnings(text: &str) -> Vec<SecretWarning> {
    let mut warnings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        let kind = if lower.contains("-----begin") && lower.contains("private key-----") {
            Some("private key")
        } else if trimmed
            .split(|character: char| {
                character.is_whitespace() || matches!(character, '"' | '\'' | ',' | ';')
            })
            .any(looks_like_secret_token)
        {
            Some("credential token")
        } else if [
            "api_key", "apikey", "api-key", "secret", "password", "token",
        ]
        .iter()
        .any(|keyword| lower.contains(keyword))
            && (trimmed.contains('=') || trimmed.contains(':'))
            && trimmed
                .split_once(['=', ':'])
                .is_some_and(|(_, value)| value.trim().trim_matches(['\'', '"']).len() >= 12)
        {
            Some("credential assignment")
        } else {
            None
        };
        if let Some(kind) = kind {
            let label = trimmed
                .split_once(['=', ':'])
                .map(|(label, _)| label.trim())
                .filter(|label| !label.is_empty())
                .unwrap_or("sensitive value");
            warnings.push(SecretWarning {
                kind: kind.into(),
                line: index + 1,
                redacted_excerpt: format!("{}: [redacted]", truncate_title(label)),
            });
        }
    }
    warnings
}

fn looks_like_secret_token(value: &str) -> bool {
    let token = value.trim_matches(|character: char| {
        !character.is_ascii_alphanumeric() && !matches!(character, '-' | '_')
    });
    (token.starts_with("sk-") && token.len() >= 20)
        || (token.starts_with("ghp_") && token.len() >= 20)
        || (token.starts_with("github_pat_") && token.len() >= 24)
        || (token.starts_with("xoxb-") && token.len() >= 20)
        || (token.starts_with("xoxp-") && token.len() >= 20)
        || (token.starts_with("AKIA")
            && token.len() == 20
            && token
                .chars()
                .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit()))
}

fn read_backup_settings(connection: &Connection) -> Result<BackupSettings> {
    connection
        .query_row(
            "SELECT enabled, interval_hours, directory, last_backup_at, updated_at
             FROM backup_settings WHERE singleton = 1",
            [],
            |row| {
                Ok(BackupSettings {
                    enabled: row.get(0)?,
                    interval_hours: row.get(1)?,
                    directory: row.get(2)?,
                    last_backup_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .map_err(Into::into)
}

fn backup_is_due(settings: &BackupSettings) -> bool {
    let Some(last_backup_at) = settings.last_backup_at.as_deref() else {
        return true;
    };
    DateTime::parse_from_rfc3339(last_backup_at)
        .map(|last| {
            Utc::now()
                .signed_duration_since(last.with_timezone(&Utc))
                .num_hours()
                >= settings.interval_hours
        })
        .unwrap_or(true)
}

fn validate_absolute_path(field: &'static str, value: &str) -> Result<PathBuf> {
    let path = PathBuf::from(required_text(field, value, 32_000)?);
    if !path.is_absolute() {
        return Err(CoreError::Validation {
            field,
            message: "must be an absolute path".into(),
        });
    }
    Ok(path)
}

fn validate_database_path(path: &Path) -> Result<()> {
    if !path.is_absolute() || !path.is_file() {
        return Err(CoreError::Validation {
            field: "source",
            message: "must be an existing absolute database path".into(),
        });
    }
    if path.extension().and_then(|value| value.to_str()) != Some("db") {
        return Err(CoreError::Validation {
            field: "source",
            message: "must use the .db extension".into(),
        });
    }
    Ok(())
}

fn unique_database_path(directory: &Path, prefix: &str) -> Result<PathBuf> {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let base = directory.join(format!("{prefix}-{timestamp}.db"));
    if !base.exists() {
        return Ok(base);
    }
    Ok(directory.join(format!(
        "{prefix}-{timestamp}-{}.db",
        &Uuid::new_v4().to_string()[..8]
    )))
}

fn copy_database(source: &ContextStore, target: &ContextStore) -> Result<()> {
    let source_connection = source.connect()?;
    let mut target_connection = target.connect()?;
    let backup = Backup::new(&source_connection, &mut target_connection)?;
    backup.run_to_completion(128, Duration::from_millis(5), None)?;
    drop(backup);
    target.verify_integrity_with(&target_connection)
}

fn table_count(connection: &Connection, table: &str) -> Result<i64> {
    let sql = match table {
        "projects" => "SELECT count(*) FROM projects",
        "conversations" => "SELECT count(*) FROM conversations",
        "messages" => "SELECT count(*) FROM messages",
        "memories" => "SELECT count(*) FROM memories",
        "context_packs" => "SELECT count(*) FROM context_packs",
        "conversation_handoffs" => "SELECT count(*) FROM conversation_handoffs",
        "import_runs" => "SELECT count(*) FROM import_runs",
        _ => unreachable!("table count names are internal constants"),
    };
    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn stable_hash(content: &str) -> String {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in normalized.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}:{}", normalized.len())
}

fn normalized_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_key_values(title: &str, content: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for line in content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let pair = line.split_once(':').or_else(|| line.split_once('='));
        if let Some((key, value)) = pair {
            let key = normalized_key(key.trim_start_matches(['-', '*', '#', ' ']));
            let value = value.trim();
            if !key.is_empty() && !value.is_empty() {
                values.insert(key, value.into());
            }
        }
    }
    if values.is_empty() {
        values.insert(normalized_key(title), content.trim().into());
    }
    values
}

fn find_conflict_candidates(
    connection: &Connection,
    memory_id: Option<&str>,
    project_id: Option<&str>,
    memory_type: &str,
    title: &str,
    content: &str,
) -> Result<Vec<ConflictCandidate>> {
    if !matches!(
        memory_type,
        "decision" | "requirement" | "fact" | "preference"
    ) {
        return Ok(Vec::new());
    }
    let current = extract_key_values(title, content);
    let mut statement = connection.prepare(
        "SELECT m.id, m.title, v.content
         FROM memories m JOIN memory_versions v ON v.id = m.current_version_id
         WHERE m.status = 'active' AND m.type = ?1 AND m.project_id IS ?2
           AND (?3 IS NULL OR m.id <> ?3)
         ORDER BY m.title COLLATE NOCASE, m.id",
    )?;
    let others = statement
        .query_map(params![memory_type, project_id, memory_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut conflicts = Vec::new();
    for (other_id, other_title, other_content) in others {
        let other = extract_key_values(&other_title, &other_content);
        for (key, value) in &current {
            if let Some(other_value) = other.get(key)
                && normalized_key(value) != normalized_key(other_value)
            {
                conflicts.push(ConflictCandidate {
                    conflicting_memory_id: other_id.clone(),
                    conflicting_memory_title: other_title.clone(),
                    conflict_key: key.clone(),
                    current_value: value.clone(),
                    conflicting_value: other_value.clone(),
                });
            }
        }
    }
    Ok(conflicts)
}

fn refresh_memory_conflicts(connection: &Connection, memory_id: &str) -> Result<()> {
    let (project_id, memory_type, title, content): (
        Option<String>,
        String,
        String,
        String,
    ) = connection.query_row(
        "SELECT m.project_id, m.type, m.title, v.content FROM memories m JOIN memory_versions v ON v.id = m.current_version_id WHERE m.id = ?1",
        [memory_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let candidates = find_conflict_candidates(
        connection,
        Some(memory_id),
        project_id.as_deref(),
        &memory_type,
        &title,
        &content,
    )?;
    let timestamp = now();
    connection.execute(
        "UPDATE memory_conflicts SET status = 'resolved', resolved_at = ?1 WHERE memory_id = ?2 AND status = 'unresolved'",
        params![timestamp, memory_id],
    )?;
    for conflict in candidates {
        connection.execute(
            "INSERT OR IGNORE INTO memory_conflicts
             (id, memory_id, conflicting_memory_id, conflict_key, current_value, conflicting_value, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unresolved', ?7)",
            params![
                Uuid::new_v4().to_string(),
                memory_id,
                conflict.conflicting_memory_id,
                conflict.conflict_key,
                conflict.current_value,
                conflict.conflicting_value,
                timestamp,
            ],
        )?;
    }
    Ok(())
}

fn map_memory_branch(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryBranch> {
    Ok(MemoryBranch {
        id: row.get(0)?,
        memory_id: row.get(1)?,
        base_version_id: row.get(2)?,
        name: row.get(3)?,
        status: row.get(4)?,
        current_version_id: row.get(5)?,
        current_content: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn map_memory_conflict(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryConflict> {
    Ok(MemoryConflict {
        id: row.get(0)?,
        memory_id: row.get(1)?,
        memory_title: row.get(2)?,
        conflicting_memory_id: row.get(3)?,
        conflicting_memory_title: row.get(4)?,
        conflict_key: row.get(5)?,
        current_value: row.get(6)?,
        conflicting_value: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
        resolved_at: row.get(10)?,
    })
}

fn validated_workspace_uri(value: &str) -> Result<String> {
    let value = required_text("workspaceUri", value, 4_000)?;
    if value.chars().any(char::is_control)
        || !(value.starts_with("file://") || value.starts_with("vscode-remote://"))
    {
        return Err(CoreError::Validation {
            field: "workspaceUri",
            message: "must be a file or vscode-remote URI".into(),
        });
    }
    Ok(value)
}

fn validated_relative_path(value: &str) -> Result<String> {
    let value = required_text("relativePath", value, 4_000)?.replace('\\', "/");
    let path = Path::new(&value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(CoreError::Validation {
            field: "relativePath",
            message: "must stay inside the mapped repository".into(),
        });
    }
    Ok(value)
}

fn validated_line_range(
    start_line: Option<i64>,
    end_line: Option<i64>,
) -> Result<(Option<i64>, Option<i64>)> {
    match (start_line, end_line) {
        (None, None) => Ok((None, None)),
        (Some(start), Some(end)) if start >= 1 && end >= start && end - start <= 20_000 => {
            Ok((Some(start), Some(end)))
        }
        _ => Err(CoreError::Validation {
            field: "lineRange",
            message: "start and end lines must be supplied together and span at most 20001 lines"
                .into(),
        }),
    }
}

fn get_workspace_mapping(
    connection: &Connection,
    workspace_uri: &str,
) -> Result<Option<WorkspaceMapping>> {
    connection
        .query_row(
            "SELECT w.workspace_uri, w.repository_root, w.project_id, p.name,
                    w.created_at, w.updated_at
             FROM workspace_mappings w JOIN projects p ON p.id = w.project_id
             WHERE w.workspace_uri = ?1",
            [workspace_uri],
            |row| {
                Ok(WorkspaceMapping {
                    workspace_uri: row.get(0)?,
                    repository_root: row.get(1)?,
                    project_id: row.get(2)?,
                    project_name: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn map_code_reference(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodeReference> {
    Ok(CodeReference {
        id: row.get(0)?,
        project_id: row.get(1)?,
        workspace_uri: row.get(2)?,
        repository_root: row.get(3)?,
        relative_path: row.get(4)?,
        language: row.get(5)?,
        start_line: row.get(6)?,
        end_line: row.get(7)?,
        content: row.get(8)?,
        content_hash: row.get(9)?,
        created_at: row.get(10)?,
    })
}

fn list_recent_code_references(
    connection: &Connection,
    project_id: &str,
    limit: usize,
) -> Result<Vec<CodeReference>> {
    let mut statement = connection.prepare(
        "SELECT id, project_id, workspace_uri, repository_root, relative_path, language,
                start_line, end_line, content, content_hash, created_at
         FROM code_references WHERE project_id = ?1
         ORDER BY created_at DESC, relative_path COLLATE NOCASE LIMIT ?2",
    )?;
    let rows = statement.query_map(params![project_id, limit as i64], map_code_reference)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn list_active_project_memories(connection: &Connection, project_id: &str) -> Result<Vec<Memory>> {
    let mut statement = connection.prepare(
        "SELECT m.id, m.project_id, m.type, m.authority, m.status, m.title,
                m.current_version_id, v.content, m.created_at
         FROM memories m JOIN memory_versions v ON v.id = m.current_version_id
         WHERE m.project_id = ?1 AND m.status = 'active'
         ORDER BY CASE m.type
                    WHEN 'decision' THEN 0 WHEN 'requirement' THEN 1 WHEN 'fact' THEN 2
                    WHEN 'task' THEN 3 ELSE 4 END,
                  CASE m.authority
                    WHEN 'user_confirmed' THEN 0 WHEN 'external_fact' THEN 1 ELSE 2 END,
                  m.created_at DESC, m.title COLLATE NOCASE",
    )?;
    let rows = statement.query_map([project_id], map_memory)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn map_agent_write_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentWriteCandidate> {
    Ok(AgentWriteCandidate {
        id: row.get(0)?,
        project_id: row.get(1)?,
        requested_by: row.get(2)?,
        memory_type: row.get(3)?,
        title: row.get(4)?,
        content: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        reviewed_at: row.get(8)?,
        memory_id: row.get(9)?,
    })
}

fn get_agent_write_candidate(
    connection: &Connection,
    candidate_id: &str,
) -> Result<AgentWriteCandidate> {
    connection
        .query_row(
            "SELECT id, project_id, requested_by, memory_type, title, content, status,
                    created_at, reviewed_at, memory_id
             FROM agent_write_candidates WHERE id = ?1",
            [candidate_id],
            map_agent_write_candidate,
        )
        .optional()?
        .ok_or_else(|| CoreError::NotFound {
            entity: "agent write candidate",
            id: candidate_id.into(),
        })
}

fn validate_mcp_client_id(value: &str) -> Result<String> {
    let client_id = required_text("clientId", value, 100)?;
    if !client_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(CoreError::Validation {
            field: "clientId",
            message: "may contain only letters, numbers, dot, underscore, colon and hyphen".into(),
        });
    }
    Ok(client_id)
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn prepare_destination(destination: impl AsRef<Path>) -> Result<PathBuf> {
    let destination = destination.as_ref().to_path_buf();
    if destination.exists() {
        return Err(CoreError::DestinationExists(destination));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(destination)
}

fn prepare_destination_with_extension(
    destination: impl AsRef<Path>,
    allowed_extensions: &[&str],
) -> Result<PathBuf> {
    let destination = destination.as_ref();
    if !destination.is_absolute() {
        return Err(CoreError::Validation {
            field: "destination",
            message: "must be an absolute path".into(),
        });
    }
    let extension = destination
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !allowed_extensions.contains(&extension.as_str()) {
        return Err(CoreError::Validation {
            field: "destination",
            message: format!(
                "must use one of these extensions: {}",
                allowed_extensions.join(", ")
            ),
        });
    }
    prepare_destination(destination)
}

fn ensure_optional_exists(
    connection: &Connection,
    table: &'static str,
    id: Option<&str>,
    entity: &'static str,
) -> Result<()> {
    if let Some(id) = id {
        ensure_exists(connection, table, id, entity)?;
    }
    Ok(())
}

fn ensure_exists(
    connection: &Connection,
    table: &'static str,
    id: &str,
    entity: &'static str,
) -> Result<()> {
    let sql = match table {
        "projects" => "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1)",
        "memory_spaces" => "SELECT EXISTS(SELECT 1 FROM memory_spaces WHERE id = ?1)",
        "conversations" => "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = ?1)",
        "memories" => "SELECT EXISTS(SELECT 1 FROM memories WHERE id = ?1)",
        "context_packs" => "SELECT EXISTS(SELECT 1 FROM context_packs WHERE id = ?1)",
        _ => unreachable!("table names are internal constants"),
    };
    let exists: bool = connection.query_row(sql, [id], |row| row.get(0))?;
    if !exists {
        return Err(CoreError::NotFound {
            entity,
            id: id.to_owned(),
        });
    }
    Ok(())
}

fn ensure_target_exists(connection: &Connection, target_type: &str, target_id: &str) -> Result<()> {
    match target_type {
        "memory" => ensure_exists(connection, "memories", target_id, "memory"),
        "memory_space" => ensure_exists(connection, "memory_spaces", target_id, "memory space"),
        "context_pack" => ensure_exists(connection, "context_packs", target_id, "context pack"),
        _ => unreachable!("target type is validated"),
    }
}

fn collect_target_memories(
    connection: &Connection,
    target_type: &str,
    target_id: &str,
    output: &mut Vec<(String, String)>,
) -> Result<()> {
    ensure_target_exists(connection, target_type, target_id)?;
    match target_type {
        "memory" => output.push((target_id.to_string(), "full".into())),
        "memory_space" => {
            let mut statement = connection.prepare(
                "WITH RECURSIVE spaces(id) AS (
                    SELECT ?1
                    UNION ALL
                    SELECT s.id FROM memory_spaces s JOIN spaces p ON s.parent_id = p.id
                 )
                 SELECT l.memory_id FROM memory_space_links l
                 JOIN spaces s ON s.id = l.memory_space_id
                 JOIN memories m ON m.id = l.memory_id
                 WHERE m.status = 'active'
                 ORDER BY m.title COLLATE NOCASE",
            )?;
            let ids = statement.query_map([target_id], |row| row.get::<_, String>(0))?;
            for id in ids {
                output.push((id?, "full".into()));
            }
        }
        "context_pack" => {
            let mut statement = connection.prepare(
                "SELECT target_id, inclusion_mode FROM context_pack_items WHERE pack_id = ?1 AND target_type = 'memory' ORDER BY ordering",
            )?;
            let items = statement.query_map([target_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for item in items {
                output.push(item?);
            }
        }
        _ => unreachable!("target type is validated"),
    }
    Ok(())
}

fn deterministic_summary(content: &str, maximum_characters: usize) -> String {
    let clean = content.trim();
    if clean.chars().count() <= maximum_characters {
        return clean.to_string();
    }
    let mut summary = clean.chars().take(maximum_characters).collect::<String>();
    summary.push('…');
    summary
}

fn smart_source_content(
    connection: &Connection,
    source_type: &str,
    source_id: &str,
) -> Result<String> {
    let content = match source_type {
        "memory" => connection
            .query_row(
                "SELECT v.content FROM memories m
                 JOIN memory_versions v ON v.id = m.current_version_id WHERE m.id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .optional()?,
        "conversation" => connection
            .query_row(
                "SELECT group_concat(role || ': ' || body, char(10) || char(10))
                 FROM (SELECT role, body FROM messages WHERE conversation_id = ?1 ORDER BY ordinal)",
                [source_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten(),
        _ => unreachable!("smart source type is validated"),
    };
    content.ok_or_else(|| CoreError::NotFound {
        entity: if source_type == "memory" {
            "memory"
        } else {
            "conversation"
        },
        id: source_id.into(),
    })
}

fn extractive_summary(content: &str, maximum_characters: usize) -> String {
    let sentences = split_sentences(content);
    if sentences.is_empty() {
        return deterministic_summary(content, maximum_characters);
    }
    let tokens = semantic_tokens(content);
    let mut frequencies = HashMap::<String, usize>::new();
    for token in tokens {
        *frequencies.entry(token).or_default() += 1;
    }
    let mut ranked = sentences
        .iter()
        .enumerate()
        .map(|(index, sentence)| {
            let score = semantic_tokens(sentence)
                .iter()
                .map(|token| frequencies.get(token).copied().unwrap_or_default() as f64)
                .sum::<f64>()
                / sentence.split_whitespace().count().max(1) as f64
                + if index == 0 { 1.0 } else { 0.0 };
            (index, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let mut selected = Vec::new();
    let mut length = 0;
    for (index, _) in ranked {
        let sentence_length = sentences[index].chars().count();
        if !selected.is_empty() && length + sentence_length + 1 > maximum_characters {
            continue;
        }
        selected.push(index);
        length += sentence_length + usize::from(length > 0);
        if length >= maximum_characters.saturating_mul(3) / 4 || selected.len() >= 6 {
            break;
        }
    }
    selected.sort_unstable();
    deterministic_summary(
        &selected
            .into_iter()
            .map(|index| sentences[index].as_str())
            .collect::<Vec<_>>()
            .join(" "),
        maximum_characters,
    )
}

fn split_sentences(content: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for character in content.chars() {
        current.push(character);
        if matches!(character, '.' | '!' | '?' | '\n') {
            let sentence = current.trim();
            if sentence.chars().count() >= 12 {
                sentences.push(sentence.to_string());
            }
            current.clear();
        }
    }
    if current.trim().chars().count() >= 12 {
        sentences.push(current.trim().to_string());
    }
    sentences
}

fn extract_candidate_statements(content: &str) -> Vec<(String, String, f64)> {
    let mut seen = HashSet::new();
    split_sentences(content)
        .into_iter()
        .filter_map(|sentence| {
            let clean = sentence
                .trim()
                .trim_start_matches(['-', '*', '#'])
                .trim()
                .to_string();
            if clean.chars().count() > 1_000 || !seen.insert(normalized_key(&clean)) {
                return None;
            }
            let lower = clean.to_lowercase();
            let (memory_type, confidence) = if [
                "we decided",
                "decision:",
                "agreed to",
                "will use",
                "selected ",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
            {
                ("decision", 0.9)
            } else if ["must ", "required", "requirement:", "needs to", "shall "]
                .iter()
                .any(|marker| lower.contains(marker))
            {
                ("requirement", 0.86)
            } else if ["suggest", "recommend", "could ", "consider ", "might "]
                .iter()
                .any(|marker| lower.contains(marker))
            {
                ("suggestion", 0.72)
            } else {
                return None;
            };
            Some((memory_type.into(), clean, confidence))
        })
        .take(25)
        .collect()
}

fn candidate_title(statement: &str) -> String {
    let words = statement
        .split_whitespace()
        .take(10)
        .collect::<Vec<_>>()
        .join(" ");
    truncate_title(words.trim_end_matches(['.', '!', '?']))
}

fn insert_generated_artifact(
    connection: &Connection,
    source_type: &str,
    source_id: &str,
    artifact_type: &str,
    content: &str,
    model_mode: &str,
) -> Result<GeneratedArtifact> {
    let artifact = GeneratedArtifact {
        id: Uuid::new_v4().to_string(),
        source_type: source_type.into(),
        source_id: source_id.into(),
        artifact_type: artifact_type.into(),
        content: content.into(),
        model_mode: model_mode.into(),
        created_at: now(),
    };
    connection.execute(
        "INSERT INTO generated_artifacts
         (id, source_type, source_id, artifact_type, content, model_mode, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            artifact.id,
            artifact.source_type,
            artifact.source_id,
            artifact.artifact_type,
            artifact.content,
            artifact.model_mode,
            artifact.created_at,
        ],
    )?;
    Ok(artifact)
}

fn map_generated_artifact(row: &rusqlite::Row<'_>) -> rusqlite::Result<GeneratedArtifact> {
    Ok(GeneratedArtifact {
        id: row.get(0)?,
        source_type: row.get(1)?,
        source_id: row.get(2)?,
        artifact_type: row.get(3)?,
        content: row.get(4)?,
        model_mode: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn map_smart_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<SmartCandidate> {
    Ok(SmartCandidate {
        id: row.get(0)?,
        project_id: row.get(1)?,
        source_type: row.get(2)?,
        source_id: row.get(3)?,
        memory_type: row.get(4)?,
        title: row.get(5)?,
        content: row.get(6)?,
        confidence: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
        reviewed_at: row.get(10)?,
    })
}

fn get_smart_candidate(connection: &Connection, candidate_id: &str) -> Result<SmartCandidate> {
    connection
        .query_row(
            "SELECT id, project_id, source_type, source_id, memory_type, title, content,
                    confidence, status, created_at, reviewed_at
             FROM smart_candidates WHERE id = ?1",
            [candidate_id],
            map_smart_candidate,
        )
        .optional()?
        .ok_or_else(|| CoreError::NotFound {
            entity: "smart candidate",
            id: candidate_id.into(),
        })
}

fn list_context_bindings(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Vec<ContextBinding>> {
    let mut statement = connection.prepare(
        "SELECT context_target_type, context_target_id, enabled, lifetime_mode FROM chat_context_bindings WHERE conversation_id = ?1 ORDER BY context_target_type, context_target_id",
    )?;
    let rows = statement.query_map([conversation_id], |row| {
        Ok(ContextBinding {
            target_type: row.get(0)?,
            target_id: row.get(1)?,
            enabled: row.get(2)?,
            lifetime_mode: row.get(3)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn conversation_context_state(
    connection: &Connection,
    conversation_id: &str,
) -> Result<ConversationContextState> {
    Ok(ConversationContextState {
        conversation_id: conversation_id.to_string(),
        bindings: list_context_bindings(connection, conversation_id)?,
        temporary_attachments: list_temporary_attachments(connection, conversation_id)?,
    })
}

fn list_projects(connection: &Connection) -> Result<Vec<Project>> {
    let mut statement = connection.prepare(
        "SELECT id, name, description, created_at, archived_at FROM projects ORDER BY archived_at IS NOT NULL, name COLLATE NOCASE",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            created_at: row.get(3)?,
            archived_at: row.get(4)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn list_memory_spaces(connection: &Connection) -> Result<Vec<MemorySpace>> {
    let mut statement = connection.prepare(
        "SELECT id, project_id, parent_id, name, description, default_scope, created_at, archived_at FROM memory_spaces ORDER BY name COLLATE NOCASE",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(MemorySpace {
            id: row.get(0)?,
            project_id: row.get(1)?,
            parent_id: row.get(2)?,
            name: row.get(3)?,
            description: row.get(4)?,
            default_scope: row.get(5)?,
            created_at: row.get(6)?,
            archived_at: row.get(7)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn list_memory_space_links(connection: &Connection) -> Result<Vec<MemorySpaceLink>> {
    let mut statement = connection.prepare(
        "SELECT memory_id, memory_space_id FROM memory_space_links ORDER BY memory_space_id, memory_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(MemorySpaceLink {
            memory_id: row.get(0)?,
            memory_space_id: row.get(1)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn map_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: row.get(0)?,
        project_id: row.get(1)?,
        memory_type: row.get(2)?,
        authority: row.get(3)?,
        status: row.get(4)?,
        title: row.get(5)?,
        current_version_id: row.get(6)?,
        current_content: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn list_memories(connection: &Connection) -> Result<Vec<Memory>> {
    let mut statement = connection.prepare(
        "SELECT m.id, m.project_id, m.type, m.authority, m.status, m.title, m.current_version_id, v.content, m.created_at FROM memories m JOIN memory_versions v ON v.id = m.current_version_id ORDER BY m.status != 'active', m.title COLLATE NOCASE",
    )?;
    let rows = statement.query_map([], map_memory)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn map_memory_version(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryVersion> {
    Ok(MemoryVersion {
        id: row.get(0)?,
        memory_id: row.get(1)?,
        content: row.get(2)?,
        change_type: row.get(3)?,
        created_at: row.get(4)?,
        supersedes_version_id: row.get(5)?,
    })
}

fn list_context_packs(connection: &Connection) -> Result<Vec<ContextPack>> {
    let mut statement = connection.prepare(
        "SELECT id, project_id, name, description, current_version, created_at, updated_at FROM context_packs ORDER BY name COLLATE NOCASE",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(ContextPack {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            current_version: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn list_temporary_attachments(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Vec<TemporaryAttachment>> {
    let mut statement = connection.prepare(
        "SELECT id, conversation_id, target_type, target_id, lifetime_mode, remaining_prompts, expires_at, created_at FROM temporary_attachments WHERE conversation_id = ?1 ORDER BY created_at",
    )?;
    let rows = statement.query_map([conversation_id], |row| {
        Ok(TemporaryAttachment {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            target_type: row.get(2)?,
            target_id: row.get(3)?,
            lifetime_mode: row.get(4)?,
            remaining_prompts: row.get(5)?,
            expires_at: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

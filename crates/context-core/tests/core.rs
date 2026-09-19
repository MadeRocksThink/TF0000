use std::fs;

use context_core::{
    AppendBranchVersionInput, AppendVersionInput, ApplyMemoryUpdateInput,
    ApplyPortableWorkspaceInput, AskMemoryInput, CaptureConversationInput, CapturedFragmentInput,
    CapturedMessageInput, ComposeContextInput, ContextPackItem, ContextRecommendationInput,
    ContextStore, CreateAgentWriteCandidateInput, CreateContextPackInput, CreateMemoryBranchInput,
    CreateMemoryInput, CreateMemorySpaceInput, CreateProjectInput, ExportPortableWorkspaceInput,
    ExtractCandidatesInput, GenerateSummaryInput, GetDecisionsInput, HandoffInput, ImportInput,
    MapWorkspaceInput, MergeSourceInput, PreviewMemoryUpdateInput, ProjectContextInput,
    RecordMcpAuditInput, RegisterMcpClientInput, ResolveSyncConflictInput,
    ReviewAgentWriteCandidateInput, ReviewSmartCandidateInput, SaveCodeReferenceInput,
    SaveContextPackInput, SearchChatsInput, SearchDecisionsInput, SearchInput, SearchMemoriesInput,
    SetMcpPermissionInput, SourceReference, UpdateBackupSettingsInput, UpdateSmartSettingsInput,
};
use rusqlite::{Connection, params};
use tempfile::TempDir;
use uuid::Uuid;

fn store() -> (TempDir, ContextStore) {
    let directory = tempfile::tempdir().expect("temp directory");
    let store = ContextStore::open(directory.path().join("context.db")).expect("open store");
    (directory, store)
}

fn project_and_space(store: &ContextStore) -> (String, String) {
    let project = store
        .create_project(CreateProjectInput {
            name: "Test project".into(),
            description: "Integration test".into(),
        })
        .expect("create project");
    let space = store
        .create_memory_space(CreateMemorySpaceInput {
            project_id: Some(project.id.clone()),
            parent_id: None,
            name: "Decisions".into(),
            description: String::new(),
            default_scope: "project".into(),
        })
        .expect("create space");
    (project.id, space.id)
}

fn search_input(query: &str) -> SearchInput {
    SearchInput {
        query: query.into(),
        scope: Some("all".into()),
        project_id: None,
        provider: None,
        date_from: None,
        date_to: None,
        memory_type: None,
        status: None,
        limit: Some(50),
        offset: None,
    }
}

#[test]
fn migrates_and_reports_healthy_database() {
    let (_directory, store) = store();
    let health = store.health().expect("health report");
    assert_eq!(health.status, "healthy");
    assert_eq!(health.schema_version, 10);
}

#[test]
fn migrates_a_legacy_database_through_phase_ten() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("phase-four.db");
    let connection = Connection::open(&path).expect("open legacy sqlite");
    connection
        .execute_batch(include_str!("../migrations/0001_initial.sql"))
        .expect("phase zero schema");
    connection
        .execute_batch(include_str!("../migrations/0002_browser_capture.sql"))
        .expect("phase two schema");
    drop(connection);

    let migrated = ContextStore::open(&path).expect("migrate phase four database");
    assert_eq!(migrated.health().unwrap().schema_version, 10);
    let connection = Connection::open(path).expect("inspect migrated sqlite");
    let tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('branch_versions', 'memory_conflicts')",
            [],
            |row| row.get(0),
        )
        .expect("count phase five tables");
    assert_eq!(tables, 2);
    let search_tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('messages_fts', 'fragments_fts', 'memory_versions_fts', 'context_packs_fts')",
            [],
            |row| row.get(0),
        )
        .expect("count phase six search tables");
    assert_eq!(search_tables, 4);
    let handoff_tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'conversation_handoffs'",
            [],
            |row| row.get(0),
        )
        .expect("count phase seven handoff tables");
    assert_eq!(handoff_tables, 1);
    let hardening_tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('import_runs', 'import_items', 'backup_settings')",
            [],
            |row| row.get(0),
        )
        .expect("count phase eight hardening tables");
    assert_eq!(hardening_tables, 3);
    let smart_tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN
             ('smart_settings', 'semantic_documents', 'generated_artifacts', 'smart_candidates')",
            [],
            |row| row.get(0),
        )
        .expect("count phase nine smart tables");
    assert_eq!(smart_tables, 4);
    let vscode_tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN
             ('workspace_mappings', 'code_references', 'code_references_fts',
              'agent_write_candidates')",
            [],
            |row| row.get(0),
        )
        .expect("count phase ten VS Code tables");
    assert_eq!(vscode_tables, 4);
    let mcp_tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN
             ('mcp_clients', 'mcp_audit_log')",
            [],
            |row| row.get(0),
        )
        .expect("count phase eleven MCP tables");
    assert_eq!(mcp_tables, 2);
    let sync_tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'sync_conflicts'",
            [],
            |row| row.get(0),
        )
        .expect("count phase twelve sync tables");
    assert_eq!(sync_tables, 1);
}

#[test]
fn phase_six_migration_backfills_existing_content() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("phase-five.db");
    let mut connection = Connection::open(&path).expect("open phase five sqlite");
    connection
        .execute_batch(include_str!("../migrations/0001_initial.sql"))
        .expect("initial schema");
    connection
        .execute_batch(include_str!("../migrations/0002_browser_capture.sql"))
        .expect("capture schema");
    connection
        .execute_batch(include_str!("../migrations/0003_merge_versioning.sql"))
        .expect("phase five schema");
    let memory_id = Uuid::new_v4().to_string();
    let version_id = Uuid::new_v4().to_string();
    let transaction = connection.transaction().expect("legacy transaction");
    transaction
        .execute(
            "INSERT INTO memories (id, type, authority, status, title, created_at) VALUES (?1, 'fact', 'user_confirmed', 'active', 'Legacy indexed fact', '2026-01-01T00:00:00Z')",
            [&memory_id],
        )
        .expect("legacy memory");
    transaction
        .execute(
            "INSERT INTO memory_versions (id, memory_id, content, change_type, created_at) VALUES (?1, ?2, 'Backfilled searchable evidence', 'create', '2026-01-01T00:00:00Z')",
            params![version_id, memory_id],
        )
        .expect("legacy version");
    transaction
        .execute(
            "UPDATE memories SET current_version_id = ?1 WHERE id = ?2",
            params![version_id, memory_id],
        )
        .expect("legacy current version");
    transaction.commit().expect("commit legacy data");
    drop(connection);

    let migrated = ContextStore::open(path).expect("migrate and backfill search");
    let results = migrated
        .search(search_input("backfilled evidence"))
        .expect("search backfilled memory");
    assert_eq!(results.results.len(), 1);
    assert_eq!(
        results.results[0].parent_id.as_deref(),
        Some(memory_id.as_str())
    );
}

#[test]
fn phase_six_search_is_filtered_ranked_synchronized_and_evidence_first() {
    let (_directory, store) = store();
    let (project_id, _) = project_and_space(&store);
    let confirmed = store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id.clone()),
            memory_space_id: None,
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Database architecture".into(),
            content: "Use encrypted SQLite for durable local storage.".into(),
        })
        .expect("confirmed memory");
    store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id.clone()),
            memory_space_id: None,
            memory_type: "suggestion".into(),
            authority: "ai_suggestion".into(),
            status: "superseded".into(),
            title: "Old database idea".into(),
            content: "Use encrypted SQLite for local storage.".into(),
        })
        .expect("superseded suggestion");
    store
        .capture_conversation(CaptureConversationInput {
            provider: "claude".into(),
            external_ref: Some("phase-six-search".into()),
            title: "WAL checkpoint discussion".into(),
            url: Some("https://claude.ai/chat/phase-six-search".into()),
            captured_at: "2026-09-19T12:00:00Z".into(),
            messages: vec![CapturedMessageInput {
                external_ref: Some("checkpoint-message".into()),
                role: "assistant".into(),
                speaker: None,
                body: "Use a predictable WAL checkpoint schedule.".into(),
                ordinal: 0,
                sent_at: None,
                source_hash: "checkpoint-message-hash".into(),
            }],
            fragments: vec![CapturedFragmentInput {
                message_external_ref: Some("checkpoint-message".into()),
                message_source_hash: None,
                start_offset: Some(18),
                end_offset: Some(32),
                selected_text: "WAL checkpoint".into(),
                source_hash: "checkpoint-fragment-hash".into(),
            }],
        })
        .expect("capture searchable conversation");

    let ranked = store
        .search(search_input("encrypted SQLite"))
        .expect("ranked search");
    assert_eq!(ranked.results[0].result_type, "memory");
    assert_eq!(
        ranked.results[0].parent_id.as_deref(),
        Some(confirmed.id.as_str())
    );
    assert!(ranked.results[0].is_current);

    let mut provider_search = search_input("checkpoint");
    provider_search.provider = Some("claude".into());
    provider_search.date_from = Some("2026-09-19".into());
    provider_search.date_to = Some("2026-09-19".into());
    let provider_results = store.search(provider_search).expect("provider search");
    assert!(!provider_results.results.is_empty());
    assert!(
        provider_results
            .results
            .iter()
            .all(|result| result.provider.as_deref() == Some("claude"))
    );
    assert!(
        provider_results
            .results
            .iter()
            .all(|result| result.source_url.as_deref()
                == Some("https://claude.ai/chat/phase-six-search"))
    );

    store
        .append_memory_version(AppendVersionInput {
            memory_id: confirmed.id.clone(),
            content: "Use encrypted SQLite with WAL checkpoint automation.".into(),
            change_type: "add".into(),
            next_status: None,
        })
        .expect("append searchable version");
    let wal_results = store
        .search(search_input("automation"))
        .expect("updated search");
    assert_eq!(
        wal_results.results[0].parent_id.as_deref(),
        Some(confirmed.id.as_str())
    );
    assert!(wal_results.results[0].is_current);

    let pack = store
        .save_context_pack(SaveContextPackInput {
            id: None,
            project_id: Some(project_id.clone()),
            name: "Storage operations".into(),
            description: "Durable database context".into(),
            items: vec![ContextPackItem {
                target_id: confirmed.id.clone(),
                ordering: 0,
                inclusion_mode: "full".into(),
            }],
        })
        .expect("searchable pack");
    assert!(
        store
            .search(search_input("automation"))
            .unwrap()
            .results
            .iter()
            .any(|result| result.result_type == "context_pack" && result.id == pack.pack.id)
    );
    store
        .append_memory_version(AppendVersionInput {
            memory_id: confirmed.id.clone(),
            content: "Use encrypted SQLite with a scheduled vacuum cadence.".into(),
            change_type: "replace".into(),
            next_status: None,
        })
        .expect("update packed memory");
    assert!(
        store
            .search(search_input("vacuum cadence"))
            .unwrap()
            .results
            .iter()
            .any(|result| result.result_type == "context_pack" && result.id == pack.pack.id)
    );

    let answer = store
        .ask_memory(AskMemoryInput {
            query: "encrypted SQLite".into(),
            scope: Some("project".into()),
            project_id: Some(project_id),
            provider: None,
        })
        .expect("ask memory evidence");
    assert_eq!(answer.status, "evidence");
    assert!(!answer.current_decisions.is_empty());
    let missing = store
        .ask_memory(AskMemoryInput {
            query: "quantum banana protocol".into(),
            scope: Some("all".into()),
            project_id: None,
            provider: None,
        })
        .expect("not recorded answer");
    assert_eq!(missing.status, "not_recorded");
    assert!(missing.evidence.is_empty());
}

#[test]
fn phase_five_merges_sources_versions_branches_and_conflicts_without_data_loss() {
    let (_directory, store) = store();
    let (project_id, space_id) = project_and_space(&store);
    for (provider, reference, body) in [
        ("chatgpt", "gpt-phase-five", "GPT architecture evidence"),
        (
            "claude",
            "claude-phase-five",
            "Claude architecture evidence",
        ),
        (
            "gemini",
            "gemini-phase-five",
            "Gemini architecture evidence",
        ),
    ] {
        store
            .capture_conversation(CaptureConversationInput {
                provider: provider.into(),
                external_ref: Some(reference.into()),
                title: format!("{provider} design"),
                url: None,
                captured_at: "2026-09-19T10:00:00Z".into(),
                messages: vec![CapturedMessageInput {
                    external_ref: None,
                    role: "assistant".into(),
                    speaker: None,
                    body: body.into(),
                    ordinal: 0,
                    sent_at: None,
                    source_hash: format!("{provider}-hash"),
                }],
                fragments: vec![],
            })
            .expect("capture provider conversation");
    }
    let conversation_sources = store
        .list_captured_sources()
        .expect("captured sources")
        .into_iter()
        .filter(|source| source.source_type == "conversation")
        .map(|source| MergeSourceInput {
            source_type: source.source_type,
            source_id: Some(source.id),
            content: None,
            label: None,
        })
        .collect::<Vec<_>>();
    let merged = store
        .apply_memory_update(ApplyMemoryUpdateInput {
            memory_id: None,
            expected_current_version_id: None,
            project_id: Some(project_id.clone()),
            memory_space_id: Some(space_id),
            memory_type: Some("summary".into()),
            authority: Some("user_confirmed".into()),
            title: Some("Cross-provider architecture".into()),
            action: "merge".into(),
            sources: conversation_sources,
        })
        .expect("create merged memory");
    assert!(merged.memory.current_content.contains("GPT architecture"));
    assert!(
        merged
            .memory
            .current_content
            .contains("Claude architecture")
    );
    assert!(
        merged
            .memory
            .current_content
            .contains("Gemini architecture")
    );
    assert_eq!(
        store
            .memory_history(&merged.memory.id)
            .unwrap()
            .sources
            .len(),
        3
    );

    let current = store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id.clone()),
            memory_space_id: None,
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Primary database".into(),
            content: "Database: SQLite".into(),
        })
        .expect("current decision");
    store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id),
            memory_space_id: None,
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Alternative database".into(),
            content: "Database: PostgreSQL".into(),
        })
        .expect("conflicting decision");
    let manual = MergeSourceInput {
        source_type: "manual".into(),
        source_id: None,
        content: Some("Database: MySQL".into()),
        label: Some("Architecture review".into()),
    };
    let preview = store
        .preview_memory_update(PreviewMemoryUpdateInput {
            memory_id: Some(current.id.clone()),
            action: "supersede".into(),
            sources: vec![manual.clone(), manual.clone()],
        })
        .expect("preview update");
    assert_eq!(preview.unique_source_count, 1);
    assert_eq!(preview.duplicate_count, 1);
    assert_eq!(preview.conflicts.len(), 1);
    assert_eq!(preview.diff.before, "Database: SQLite");
    assert_eq!(preview.diff.after, "Database: MySQL");
    let updated = store
        .apply_memory_update(ApplyMemoryUpdateInput {
            memory_id: Some(current.id.clone()),
            expected_current_version_id: preview.current_version_id,
            project_id: None,
            memory_space_id: None,
            memory_type: None,
            authority: None,
            title: None,
            action: "supersede".into(),
            sources: vec![manual.clone(), manual],
        })
        .expect("apply update");
    assert_eq!(updated.version.change_type, "supersede");
    assert_eq!(updated.conflicts.len(), 1);
    let history = store.memory_history(&current.id).expect("history");
    assert_eq!(history.versions.len(), 2);
    assert_eq!(history.sources.len(), 1);
    store
        .restore_memory_version(&current.id, &current.current_version_id)
        .expect("restore original");
    assert_eq!(store.memory_history(&current.id).unwrap().versions.len(), 3);

    let branch = store
        .create_memory_branch(CreateMemoryBranchInput {
            memory_id: current.id.clone(),
            name: "Cloud alternative".into(),
        })
        .expect("create branch");
    let branch = store
        .append_branch_version(AppendBranchVersionInput {
            branch_id: branch.id,
            content: "Database: CockroachDB".into(),
            change_type: "replace".into(),
        })
        .expect("edit branch");
    let finalized = store
        .finalize_memory_branch(&branch.id, "merge")
        .expect("merge branch");
    assert_eq!(finalized.status, "merged");
    assert!(
        store
            .get_memory(&current.id)
            .unwrap()
            .current_content
            .contains("CockroachDB")
    );
    let conflicts = store
        .list_memory_conflicts(Some(&current.id), "unresolved")
        .expect("conflict inbox");
    assert!(!conflicts.is_empty());
    store
        .resolve_memory_conflict(&conflicts[0].id, "resolved")
        .expect("resolve conflict");
}

#[test]
fn browser_capture_is_transactional_and_idempotent() {
    let (_directory, store) = store();
    let input = CaptureConversationInput {
        provider: "chatgpt".into(),
        external_ref: Some("conversation-1".into()),
        title: "Architecture".into(),
        url: Some("https://chatgpt.com/c/conversation-1".into()),
        captured_at: "2026-09-19T10:00:00Z".into(),
        messages: vec![CapturedMessageInput {
            external_ref: Some("message-1".into()),
            role: "user".into(),
            speaker: None,
            body: "Use local storage".into(),
            ordinal: 0,
            sent_at: None,
            source_hash: "message-hash".into(),
        }],
        fragments: vec![CapturedFragmentInput {
            message_external_ref: Some("message-1".into()),
            message_source_hash: None,
            start_offset: Some(4),
            end_offset: Some(9),
            selected_text: "local".into(),
            source_hash: "fragment-hash".into(),
        }],
    };
    let first = store.capture_conversation(input.clone()).expect("capture");
    let second = store.capture_conversation(input).expect("capture again");
    assert_eq!(first.conversation_id, second.conversation_id);
    let connection = Connection::open(store.database_path()).expect("open sqlite");
    let messages: i64 = connection
        .query_row("SELECT count(*) FROM messages", [], |row| row.get(0))
        .expect("count messages");
    let fragments: i64 = connection
        .query_row("SELECT count(*) FROM fragments", [], |row| row.get(0))
        .expect("count fragments");
    assert_eq!((messages, fragments), (1, 1));

    let sources = store.list_captured_sources().expect("list sources");
    assert_eq!(sources.len(), 3);
    let source = sources
        .iter()
        .find(|source| source.source_type == "fragment")
        .expect("fragment source");
    assert_eq!(source.provider, "chatgpt");
    assert_eq!(source.content, "local");
    let composed = store
        .compose_context(ComposeContextInput {
            memory_ids: vec![],
            memory_space_ids: vec![],
            context_pack_ids: vec![],
            sources: vec![SourceReference {
                id: source.id.clone(),
                source_type: source.source_type.clone(),
            }],
            conversation_id: None,
        })
        .expect("compose captured source");
    assert!(composed.text.contains("Architecture — CHATGPT (user)"));
    assert!(
        composed
            .text
            .contains("https://chatgpt.com/c/conversation-1")
    );
}

#[test]
fn manual_capture_supports_unsupported_pages() {
    let (_directory, store) = store();
    store
        .capture_conversation(CaptureConversationInput {
            provider: "manual".into(),
            external_ref: Some("https://example.com/reference".into()),
            title: "External reference".into(),
            url: Some("https://example.com/reference".into()),
            captured_at: "2026-09-19T10:00:00Z".into(),
            messages: vec![CapturedMessageInput {
                external_ref: Some("manual-reference".into()),
                role: "unknown".into(),
                speaker: None,
                body: "Exact manually imported material".into(),
                ordinal: 0,
                sent_at: None,
                source_hash: "manual-source-hash".into(),
            }],
            fragments: vec![],
        })
        .expect("manual capture");
    let sources = store.list_captured_sources().expect("list manual source");
    assert_eq!(sources.len(), 2);
    let source = sources
        .iter()
        .find(|source| source.source_type == "message")
        .expect("message source");
    assert_eq!(source.provider, "manual");
    assert_eq!(
        source.source_url.as_deref(),
        Some("https://example.com/reference")
    );
}

#[test]
fn phase_seven_handoffs_are_exact_deterministic_and_append_only() {
    let (_directory, store) = store();
    let task = store
        .create_memory(CreateMemoryInput {
            project_id: None,
            memory_space_id: None,
            memory_type: "task".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Ship handoffs".into(),
            content: "Finish the exact cross-provider handoff flow.".into(),
        })
        .expect("task memory");
    let decision = store
        .create_memory(CreateMemoryInput {
            project_id: None,
            memory_space_id: None,
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "No auto-send".into(),
            content: "Never submit a provider prompt automatically.".into(),
        })
        .expect("decision memory");
    let selected = store
        .create_memory(CreateMemoryInput {
            project_id: None,
            memory_space_id: None,
            memory_type: "fact".into(),
            authority: "external_fact".into(),
            status: "active".into(),
            title: "Selected fact".into(),
            content: "This fact was selected explicitly.".into(),
        })
        .expect("selected memory");
    let packed = store
        .create_memory(CreateMemoryInput {
            project_id: None,
            memory_space_id: None,
            memory_type: "requirement".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Packed requirement".into(),
            content: "Preserve this complete pack content without summarizing it.".into(),
        })
        .expect("packed memory");
    let pack = store
        .save_context_pack(SaveContextPackInput {
            id: None,
            project_id: None,
            name: "Launch context".into(),
            description: String::new(),
            items: vec![ContextPackItem {
                target_id: packed.id,
                ordering: 0,
                inclusion_mode: "summary".into(),
            }],
        })
        .expect("context pack");
    let capture = store
        .capture_conversation(CaptureConversationInput {
            provider: "chatgpt".into(),
            external_ref: Some("phase-seven-source".into()),
            title: "Phase seven source".into(),
            url: Some("https://chatgpt.com/c/phase-seven-source".into()),
            captured_at: "2026-09-19T12:00:00Z".into(),
            messages: (0..6)
                .map(|ordinal| CapturedMessageInput {
                    external_ref: Some(format!("message-{ordinal}")),
                    role: if ordinal % 2 == 0 {
                        "user"
                    } else {
                        "assistant"
                    }
                    .into(),
                    speaker: None,
                    body: format!("Exact message {ordinal}"),
                    ordinal,
                    sent_at: None,
                    source_hash: format!("phase-seven-hash-{ordinal}"),
                })
                .collect(),
            fragments: vec![],
        })
        .expect("source capture");

    let full_input = HandoffInput {
        source_provider: "chatgpt".into(),
        source_external_ref: "phase-seven-source".into(),
        destination_provider: "claude".into(),
        mode: "full".into(),
        recent_message_count: None,
        include_current_task: false,
        include_active_decisions: false,
        memory_ids: vec![selected.id.clone()],
        context_pack_ids: vec![pack.pack.id.clone()],
    };
    let full = store
        .preview_handoff(full_input.clone())
        .expect("full preview");
    let repeated = store
        .preview_handoff(full_input.clone())
        .expect("repeated preview");
    assert_eq!(full, repeated);
    assert!(full.text.contains("Exact message 0"));
    assert!(full.text.contains("Exact message 5"));
    assert!(full.text.contains(&task.current_content));
    assert!(full.text.contains(&decision.current_content));
    assert!(full.text.contains(&selected.current_content));
    assert!(
        full.text
            .contains("summary requested; exact content preserved")
    );
    assert!(full.text.contains("Preserve this complete pack content"));

    let minimal = store
        .preview_handoff(HandoffInput {
            mode: "minimal".into(),
            ..full_input.clone()
        })
        .expect("minimal preview");
    assert!(!minimal.text.contains("Exact message 0"));
    assert!(!minimal.text.contains("Exact message 1"));
    assert!(minimal.text.contains("Exact message 2"));
    assert!(minimal.text.contains(&task.current_content));
    assert!(!minimal.text.contains(&decision.current_content));

    let custom = store
        .preview_handoff(HandoffInput {
            mode: "custom".into(),
            recent_message_count: Some(2),
            include_current_task: false,
            include_active_decisions: false,
            ..full_input.clone()
        })
        .expect("custom preview");
    assert!(!custom.text.contains("Exact message 3"));
    assert!(custom.text.contains("Exact message 4"));
    assert!(custom.text.contains("Exact message 5"));
    assert!(!custom.text.contains(&task.current_content));
    assert!(!custom.text.contains(&decision.current_content));
    assert!(custom.text.contains(&selected.current_content));

    let connection = Connection::open(store.database_path()).expect("open database");
    let before: (i64, String) = connection
        .query_row(
            "SELECT count(*), group_concat(body, '|') FROM messages WHERE conversation_id = ?1",
            [&capture.conversation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("source before record");
    let recorded = store.record_handoff(full_input).expect("record handoff");
    let after: (i64, String) = connection
        .query_row(
            "SELECT count(*), group_concat(body, '|') FROM messages WHERE conversation_id = ?1",
            [&capture.conversation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("source after record");
    assert_eq!(before, after);
    assert_eq!(recorded.content_hash, full.content_hash);
    let handoffs = store
        .list_conversation_handoffs(Some(&capture.conversation_id))
        .expect("list handoffs");
    assert_eq!(handoffs, vec![recorded]);

    assert!(
        store
            .preview_handoff(HandoffInput {
                source_provider: "chatgpt".into(),
                source_external_ref: "phase-seven-source".into(),
                destination_provider: "chatgpt".into(),
                mode: "minimal".into(),
                recent_message_count: None,
                include_current_task: true,
                include_active_decisions: false,
                memory_ids: vec![],
                context_pack_ids: vec![],
            })
            .is_err()
    );
}

#[test]
fn phase_eight_import_backup_restore_diagnostics_and_secret_warnings_work() {
    let (directory, store) = store();
    let imports = directory.path().join("imports");
    fs::create_dir_all(&imports).expect("import directory");
    let text_path = imports.join("reference.txt");
    fs::write(
        &text_path,
        "Deployment reference\napi_key = sk-1234567890abcdefghijklmnop",
    )
    .expect("text fixture");
    let text_input = ImportInput {
        source_path: text_path.display().to_string(),
        project_id: None,
        memory_space_id: None,
    };
    let preview = store
        .preview_import(text_input.clone())
        .expect("preview text import");
    assert_eq!(preview.importable_count, 1);
    assert_eq!(preview.duplicate_count, 0);
    assert_eq!(preview.secret_warning_count, 1);
    assert_eq!(preview.candidates[0].secret_warnings[0].line, 2);
    assert!(
        !preview.candidates[0].secret_warnings[0]
            .redacted_excerpt
            .contains("sk-")
    );
    let imported = store
        .import_file(text_input.clone())
        .expect("apply text import");
    assert_eq!((imported.imported_count, imported.skipped_count), (1, 0));
    let duplicate = store
        .preview_import(text_input.clone())
        .expect("preview duplicate import");
    assert_eq!(
        (duplicate.importable_count, duplicate.duplicate_count),
        (0, 1)
    );
    let repeated = store
        .import_file(text_input)
        .expect("skip duplicate import");
    assert_eq!((repeated.imported_count, repeated.skipped_count), (0, 1));

    let markdown_path = imports.join("notes.md");
    fs::write(
        &markdown_path,
        "# First decision\nUse local storage.\n\n## Second note\nKeep raw sources.",
    )
    .expect("markdown fixture");
    let markdown = store
        .preview_import(ImportInput {
            source_path: markdown_path.display().to_string(),
            project_id: None,
            memory_space_id: None,
        })
        .expect("markdown preview");
    assert_eq!(markdown.candidates.len(), 2);

    let provider_path = imports.join("provider-export.json");
    fs::write(
        &provider_path,
        r#"{"title":"Provider chat","messages":[{"role":"user","content":"Exact question"},{"role":"assistant","content":"Exact answer"}]}"#,
    )
    .expect("provider fixture");
    let provider = store
        .preview_import(ImportInput {
            source_path: provider_path.display().to_string(),
            project_id: None,
            memory_space_id: None,
        })
        .expect("provider preview");
    assert_eq!(provider.candidates.len(), 1);
    assert!(
        provider.candidates[0]
            .content
            .contains("[USER] Exact question")
    );
    assert!(
        provider.candidates[0]
            .content
            .contains("[ASSISTANT] Exact answer")
    );

    let chatgpt_path = imports.join("chatgpt-conversations.json");
    fs::write(
        &chatgpt_path,
        r#"[{"title":"Mapped provider chat","mapping":{"one":{"message":{"author":{"role":"user"},"create_time":1,"content":{"parts":["Mapped question"]}}},"two":{"message":{"author":{"role":"assistant"},"create_time":2,"content":{"parts":["Mapped answer"]}}}}}]"#,
    )
    .expect("mapped provider fixture");
    let chatgpt = store
        .preview_import(ImportInput {
            source_path: chatgpt_path.display().to_string(),
            project_id: None,
            memory_space_id: None,
        })
        .expect("mapped provider preview");
    assert_eq!(chatgpt.candidates.len(), 1);
    assert!(
        chatgpt.candidates[0]
            .content
            .contains("[USER] Mapped question")
    );
    assert!(
        chatgpt.candidates[0]
            .content
            .contains("[ASSISTANT] Mapped answer")
    );

    let backup_directory = directory.path().join("backups");
    fs::create_dir_all(&backup_directory).expect("backup directory");
    let settings = store
        .update_backup_settings(UpdateBackupSettingsInput {
            enabled: true,
            interval_hours: 24,
            directory: backup_directory.display().to_string(),
        })
        .expect("backup settings");
    assert!(settings.enabled);
    let scheduled = store.run_scheduled_backup(false).expect("scheduled backup");
    assert!(scheduled.created);
    let backup_path = scheduled.path.expect("backup path");
    assert!(std::path::Path::new(&backup_path).is_file());
    assert!(!store.run_scheduled_backup(false).expect("not due").created);

    let disposable = store
        .create_memory(CreateMemoryInput {
            project_id: None,
            memory_space_id: None,
            memory_type: "fact".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Created after backup".into(),
            content: "This should disappear after restore.".into(),
        })
        .expect("post-backup memory");
    assert!(store.get_memory(&disposable.id).is_ok());
    let restored = store.restore_backup(&backup_path).expect("restore backup");
    assert_eq!(restored.schema_version, 10);
    assert!(std::path::Path::new(&restored.recovery_backup_path).is_file());
    assert!(store.get_memory(&disposable.id).is_err());

    let diagnostics = store.diagnostics().expect("diagnostics");
    assert_eq!(diagnostics.integrity, "ok");
    assert_eq!(diagnostics.schema_version, 10);
    assert!(diagnostics.import_count >= 2);
    let diagnostic_path = directory.path().join("diagnostics.json");
    store
        .export_diagnostics(&diagnostic_path)
        .expect("diagnostic export");
    let diagnostic_text = fs::read_to_string(diagnostic_path).expect("read diagnostics");
    assert!(diagnostic_text.contains("\"integrity\": \"ok\""));
    assert!(!diagnostic_text.contains("sk-1234567890"));
}

#[test]
fn phase_nine_smart_layer_is_optional_grounded_and_reviewable() {
    let (_directory, store) = store();
    let (project_id, space_id) = project_and_space(&store);
    let source = store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id.clone()),
            memory_space_id: Some(space_id.clone()),
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Customer transport policy".into(),
            content: "We decided the customer will buy a car. The service must keep local backups. We recommend reviewing original sources before release.".into(),
        })
        .expect("smart source memory");

    assert_eq!(store.smart_settings().unwrap().mode, "off");
    assert!(
        store
            .search(search_input("automobile purchase"))
            .unwrap()
            .results
            .is_empty()
    );
    let settings = store
        .update_smart_settings(UpdateSmartSettingsInput {
            mode: "local".into(),
            provider_name: String::new(),
            provider_endpoint: String::new(),
            provider_model: String::new(),
            recommendation_mode: "ask".into(),
        })
        .expect("enable local smart features");
    assert_eq!(settings.embedding_model, "tf0000-mini-embed-v1");
    assert!(store.rebuild_semantic_index().unwrap() >= 1);
    let hybrid = store
        .search(SearchInput {
            query: "automobile purchase".into(),
            scope: Some("project".into()),
            project_id: Some(project_id.clone()),
            provider: None,
            date_from: None,
            date_to: None,
            memory_type: None,
            status: None,
            limit: Some(20),
            offset: None,
        })
        .expect("hybrid semantic search");
    assert!(
        hybrid
            .results
            .iter()
            .any(|result| result.parent_id.as_deref() == Some(&source.id))
    );

    let summary = store
        .generate_summary(GenerateSummaryInput {
            source_type: "memory".into(),
            source_id: source.id.clone(),
            maximum_characters: Some(220),
        })
        .expect("separate summary artifact");
    assert_eq!(summary.artifact_type, "summary");
    assert_eq!(
        store.get_memory(&source.id).unwrap().current_content,
        source.current_content
    );
    assert_eq!(
        store.generated_artifacts(Some(&source.id)).unwrap().len(),
        1
    );

    let candidates = store
        .extract_candidates(ExtractCandidatesInput {
            source_type: "memory".into(),
            source_id: source.id.clone(),
            project_id: Some(project_id.clone()),
        })
        .expect("extract review candidates");
    assert_eq!(candidates.len(), 3);
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.status == "pending")
    );
    let accepted = store
        .review_smart_candidate(ReviewSmartCandidateInput {
            candidate_id: candidates[0].id.clone(),
            action: "accept".into(),
            memory_space_id: Some(space_id),
        })
        .expect("accept candidate");
    let accepted_memory = accepted.memory.expect("draft candidate memory");
    assert_eq!(accepted_memory.authority, "ai_suggestion");
    assert_eq!(accepted_memory.status, "draft");

    let recommendations = store
        .recommend_context(ContextRecommendationInput {
            prompt: "Which vehicle should the customer purchase?".into(),
            project_id: Some(project_id.clone()),
            limit: Some(5),
        })
        .expect("smart context recommendations");
    assert_eq!(recommendations.mode, "ask");
    assert!(recommendations.requires_confirmation);
    assert!(!recommendations.results.is_empty());

    let competing = store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id),
            memory_space_id: None,
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Competing transport policy".into(),
            content: "vehicle: train".into(),
        })
        .expect("competing decision");
    let preview = store
        .preview_memory_update(PreviewMemoryUpdateInput {
            memory_id: Some(competing.id.clone()),
            action: "replace".into(),
            sources: vec![MergeSourceInput {
                source_type: "manual".into(),
                source_id: None,
                content: Some("customer transport policy: train".into()),
                label: Some("review".into()),
            }],
        })
        .expect("preview conflicting change");
    let updated = store
        .apply_memory_update(ApplyMemoryUpdateInput {
            memory_id: Some(competing.id),
            expected_current_version_id: preview.current_version_id,
            project_id: None,
            memory_space_id: None,
            memory_type: None,
            authority: None,
            title: None,
            action: "replace".into(),
            sources: vec![MergeSourceInput {
                source_type: "manual".into(),
                source_id: None,
                content: Some("customer transport policy: train".into()),
                label: Some("review".into()),
            }],
        })
        .expect("apply conflicting change");
    if let Some(conflict) = updated.conflicts.first() {
        let explanation = store
            .explain_conflict(&conflict.id)
            .expect("explain conflict");
        assert_eq!(explanation.sources.len(), 2);
        assert!(
            explanation
                .artifact
                .content
                .contains("has not chosen a winner")
        );
    }

    let benchmark = store.benchmark_local_embeddings().expect("CPU benchmark");
    assert_eq!(benchmark.device, "CPU");
    assert_eq!(benchmark.recommended_profile, "balanced");
    assert_eq!(benchmark.benchmarks.len(), 3);

    store
        .update_smart_settings(UpdateSmartSettingsInput {
            mode: "off".into(),
            provider_name: String::new(),
            provider_endpoint: String::new(),
            provider_model: String::new(),
            recommendation_mode: "off".into(),
        })
        .expect("disable smart features");
    assert!(store.search(search_input("customer")).is_ok());
}

#[test]
fn phase_ten_vscode_context_is_mapped_provenanced_and_confirmation_gated() {
    let (_directory, store) = store();
    let (project_id, space_id) = project_and_space(&store);
    store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id.clone()),
            memory_space_id: Some(space_id.clone()),
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Payment storage".into(),
            content: "Use SQLite for local payment state.".into(),
        })
        .expect("project decision");
    let workspace_uri = "file:///E:/work/payment-service";
    let mapping = store
        .map_workspace(MapWorkspaceInput {
            workspace_uri: workspace_uri.into(),
            repository_root: "E:\\work\\payment-service".into(),
            project_id: project_id.clone(),
        })
        .expect("workspace mapping");
    assert_eq!(mapping.project_name, "Test project");
    assert_eq!(
        store
            .workspace_mapping(workspace_uri)
            .expect("lookup mapping")
            .expect("mapped workspace")
            .project_id,
        project_id
    );

    let input = SaveCodeReferenceInput {
        workspace_uri: workspace_uri.into(),
        relative_path: "src/payment.ts".into(),
        language: "typescript".into(),
        start_line: Some(12),
        end_line: Some(18),
        content: "export function persistPayment() { return sqlite.save(); }".into(),
    };
    let reference = store
        .save_code_reference(input.clone())
        .expect("save selected code");
    let duplicate = store
        .save_code_reference(input)
        .expect("deduplicate selected code");
    assert_eq!(reference.id, duplicate.id);
    assert!(
        store
            .save_code_reference(SaveCodeReferenceInput {
                workspace_uri: workspace_uri.into(),
                relative_path: "../secret.txt".into(),
                language: "text".into(),
                start_line: None,
                end_line: None,
                content: "outside repository".into(),
            })
            .is_err()
    );
    let found = store
        .search(SearchInput {
            query: "persistPayment".into(),
            scope: Some("project".into()),
            project_id: Some(project_id.clone()),
            provider: None,
            date_from: None,
            date_to: None,
            memory_type: None,
            status: None,
            limit: Some(20),
            offset: None,
        })
        .expect("search code reference");
    assert!(
        found
            .results
            .iter()
            .any(|result| result.result_type == "code_reference")
    );

    let context = store
        .get_project_context(ProjectContextInput {
            project_id: project_id.clone(),
            maximum_characters: Some(8_000),
        })
        .expect("compose project context");
    assert_eq!(context.memory_count, 1);
    assert_eq!(context.code_reference_count, 1);
    assert!(context.text.contains("Payment storage"));
    assert!(context.text.contains("src/payment.ts lines 12-18"));
    let decisions = store
        .search_decisions(SearchDecisionsInput {
            project_id: project_id.clone(),
            query: "SQLite payment".into(),
            limit: Some(10),
        })
        .expect("search project decisions");
    assert_eq!(decisions.results.len(), 1);
    assert_eq!(
        decisions.results[0].memory_type.as_deref(),
        Some("decision")
    );

    let candidate = store
        .create_agent_write_candidate(CreateAgentWriteCandidateInput {
            project_id,
            requested_by: "vscode-language-model-tool".into(),
            memory_type: "requirement".into(),
            title: "Payment retry limit".into(),
            content: "Payment retries must stop after three attempts.".into(),
        })
        .expect("create pending write candidate");
    assert_eq!(candidate.status, "pending");
    assert!(
        store
            .review_agent_write_candidate(ReviewAgentWriteCandidateInput {
                candidate_id: candidate.id.clone(),
                action: "accept".into(),
                confirmed: false,
                memory_space_id: Some(space_id.clone()),
            })
            .is_err()
    );
    let accepted = store
        .review_agent_write_candidate(ReviewAgentWriteCandidateInput {
            candidate_id: candidate.id,
            action: "accept".into(),
            confirmed: true,
            memory_space_id: Some(space_id),
        })
        .expect("accept confirmed candidate");
    assert_eq!(accepted.status, "accepted");
    let memory = store
        .get_memory(accepted.memory_id.as_deref().expect("created draft memory"))
        .expect("candidate memory");
    assert_eq!(memory.authority, "ai_suggestion");
    assert_eq!(memory.status, "draft");
}

#[test]
fn phase_eleven_mcp_queries_permissions_and_audit_share_the_core() {
    let (_directory, store) = store();
    let (project_id, space_id) = project_and_space(&store);
    let memory = store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id.clone()),
            memory_space_id: Some(space_id),
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "MCP transport".into(),
            content: "Use local stdio transport for MCP clients.".into(),
        })
        .expect("MCP decision");
    let pack = store
        .save_context_pack(SaveContextPackInput {
            id: None,
            project_id: Some(project_id.clone()),
            name: "Agent essentials".into(),
            description: "MCP test pack".into(),
            items: vec![ContextPackItem {
                target_id: memory.id.clone(),
                ordering: 0,
                inclusion_mode: "full".into(),
            }],
        })
        .expect("MCP context pack");
    let conversation = store
        .capture_conversation(CaptureConversationInput {
            provider: "chatgpt".into(),
            external_ref: Some("mcp-search-chat".into()),
            title: "Agent integration".into(),
            url: None,
            captured_at: "2026-09-19T18:00:00Z".into(),
            messages: vec![CapturedMessageInput {
                external_ref: Some("mcp-message".into()),
                role: "user".into(),
                speaker: None,
                body: "The agent needs source grounded project context.".into(),
                ordinal: 0,
                sent_at: None,
                source_hash: "mcp-message-hash".into(),
            }],
            fragments: Vec::new(),
        })
        .expect("captured MCP chat");
    Connection::open(store.database_path())
        .expect("open database")
        .execute(
            "UPDATE conversations SET project_id = ?1 WHERE id = ?2",
            params![project_id, conversation.conversation_id],
        )
        .expect("scope captured chat");

    assert_eq!(
        store
            .search_memories(SearchMemoriesInput {
                project_id: project_id.clone(),
                query: "stdio transport".into(),
                memory_type: None,
                status: None,
                limit: Some(10),
            })
            .expect("MCP memory search")
            .result_count,
        1
    );
    assert_eq!(
        store
            .get_decisions(GetDecisionsInput {
                project_id: project_id.clone(),
                query: None,
                limit: Some(10),
            })
            .expect("MCP decisions")
            .result_count,
        1
    );
    assert_eq!(
        store
            .get_context_pack(&pack.pack.id)
            .expect("MCP pack")
            .items
            .len(),
        1
    );
    assert_eq!(
        store
            .search_chats(SearchChatsInput {
                project_id: project_id.clone(),
                query: "source grounded".into(),
                provider: Some("chatgpt".into()),
                date_from: None,
                date_to: None,
                limit: Some(10),
            })
            .expect("MCP chat search")
            .result_count,
        1
    );

    let client = store
        .register_mcp_client(RegisterMcpClientInput {
            client_id: "codex-local".into(),
            display_name: "Codex Local".into(),
        })
        .expect("register MCP client");
    let default_permission = store
        .mcp_permission(&client.id, Some(&project_id))
        .expect("default MCP permission");
    assert!(default_permission.read_allowed);
    assert!(!default_permission.candidate_write_allowed);
    let updated_permission = store
        .set_mcp_permission(SetMcpPermissionInput {
            client_id: client.id.clone(),
            project_id: Some(project_id.clone()),
            read_allowed: true,
            candidate_write_allowed: true,
        })
        .expect("project MCP permission");
    assert!(updated_permission.candidate_write_allowed);
    store
        .record_mcp_audit(RecordMcpAuditInput {
            client_id: client.id.clone(),
            tool_name: "get_project_context".into(),
            access_type: "read".into(),
            project_id: Some(project_id),
            entity_type: Some("project".into()),
            entity_id: None,
            outcome: "allowed".into(),
        })
        .expect("MCP audit entry");
    let audit = store.mcp_audit_log(&client.id, 10).expect("MCP audit log");
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].tool_name, "get_project_context");
}

#[test]
fn phase_twelve_selective_sync_fast_forwards_and_surfaces_offline_conflicts() {
    let (_source_directory, source) = store();
    let (project_id, space_id) = project_and_space(&source);
    let excluded = source
        .create_project(CreateProjectInput {
            name: "Excluded".into(),
            description: String::new(),
        })
        .expect("excluded project");
    let memory = source
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id.clone()),
            memory_space_id: Some(space_id),
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Sync policy".into(),
            content: "Start locally".into(),
        })
        .expect("source memory");
    source
        .create_memory(CreateMemoryInput {
            project_id: Some(excluded.id.clone()),
            memory_space_id: None,
            memory_type: "fact".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Private excluded memory".into(),
            content: "Do not sync".into(),
        })
        .expect("excluded memory");
    source
        .save_context_pack(SaveContextPackInput {
            id: None,
            project_id: Some(project_id.clone()),
            name: "Portable handoff".into(),
            description: String::new(),
            items: vec![ContextPackItem {
                target_id: memory.id.clone(),
                ordering: 0,
                inclusion_mode: "full".into(),
            }],
        })
        .expect("context pack");
    let selection = ExportPortableWorkspaceInput {
        project_ids: vec![project_id.clone()],
        include_global: false,
        source_device_id: "source-laptop".into(),
    };
    let initial = source
        .export_portable_workspace(selection.clone())
        .expect("selective export");
    assert_eq!(initial.projects.len(), 1);
    assert_eq!(initial.memories.len(), 1);
    assert_eq!(initial.context_packs.len(), 1);
    assert!(
        !initial
            .projects
            .iter()
            .any(|project| project.id == excluded.id)
    );

    let (_target_directory, target) = store();
    let applied = target
        .apply_portable_workspace(ApplyPortableWorkspaceInput { workspace: initial })
        .expect("initial sync");
    assert_eq!(applied.added, 1);
    assert_eq!(
        target.get_memory(&memory.id).unwrap().current_content,
        "Start locally"
    );

    let remote = source
        .append_memory_version(AppendVersionInput {
            memory_id: memory.id.clone(),
            content: "Remote offline edit".into(),
            change_type: "replace".into(),
            next_status: None,
        })
        .expect("remote edit");
    let fast_forward = source.export_portable_workspace(selection.clone()).unwrap();
    let result = target
        .apply_portable_workspace(ApplyPortableWorkspaceInput {
            workspace: fast_forward,
        })
        .unwrap();
    assert_eq!(result.fast_forwarded, 1);
    assert_eq!(
        target.get_memory(&memory.id).unwrap().current_version_id,
        remote.id
    );

    target
        .append_memory_version(AppendVersionInput {
            memory_id: memory.id.clone(),
            content: "Target offline edit".into(),
            change_type: "replace".into(),
            next_status: None,
        })
        .expect("target concurrent edit");
    source
        .append_memory_version(AppendVersionInput {
            memory_id: memory.id.clone(),
            content: "Source concurrent edit".into(),
            change_type: "replace".into(),
            next_status: None,
        })
        .expect("source concurrent edit");
    let concurrent = source.export_portable_workspace(selection).unwrap();
    let result = target
        .apply_portable_workspace(ApplyPortableWorkspaceInput {
            workspace: concurrent,
        })
        .unwrap();
    assert_eq!(result.conflicts, 1);
    let conflict = target
        .list_sync_conflicts("unresolved")
        .unwrap()
        .pop()
        .expect("sync conflict");
    assert_eq!(conflict.remote_device_id, "source-laptop");
    target
        .resolve_sync_conflict(ResolveSyncConflictInput {
            conflict_id: conflict.id,
            resolution: "use_remote".into(),
        })
        .expect("resolve with remote");
    assert_eq!(
        target.get_memory(&memory.id).unwrap().current_content,
        "Source concurrent edit"
    );
    assert!(target.list_sync_conflicts("unresolved").unwrap().is_empty());
}

#[test]
fn phase_four_spaces_packs_bindings_and_temporary_context_work_together() {
    let (_directory, store) = store();
    let (project_id, root_space_id) = project_and_space(&store);
    let child = store
        .create_memory_space(CreateMemorySpaceInput {
            project_id: Some(project_id.clone()),
            parent_id: Some(root_space_id.clone()),
            name: "Nested".into(),
            description: String::new(),
            default_scope: "project".into(),
        })
        .expect("nested space");
    assert!(
        store
            .move_memory_space(&root_space_id, Some(&child.id))
            .is_err()
    );
    let memory = store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id.clone()),
            memory_space_id: None,
            memory_type: "requirement".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Performance budget".into(),
            content: "Keep every interaction fast and deterministic.".repeat(20),
        })
        .expect("memory");
    store
        .move_memory_to_space(&memory.id, Some(&child.id))
        .expect("move memory");
    let pack = store
        .save_context_pack(SaveContextPackInput {
            id: None,
            project_id: Some(project_id.clone()),
            name: "Fast defaults".into(),
            description: "Reusable test pack".into(),
            items: vec![ContextPackItem {
                target_id: memory.id.clone(),
                ordering: 0,
                inclusion_mode: "summary".into(),
            }],
        })
        .expect("save pack");
    assert_eq!(pack.items[0].inclusion_mode, "summary");

    let conversation_id = Uuid::new_v4().to_string();
    let connection = Connection::open(store.database_path()).expect("open sqlite");
    connection
        .execute("PRAGMA foreign_keys = ON", [])
        .expect("foreign keys");
    connection
        .execute(
            "INSERT INTO conversations (id, project_id, provider, external_ref, title, captured_at) VALUES (?1, ?2, 'chatgpt', 'phase-four', 'Phase four', '2026-09-19T00:00:00Z')",
            params![conversation_id, project_id],
        )
        .expect("conversation");
    store
        .set_context_binding(
            &conversation_id,
            "context_pack",
            &pack.pack.id,
            true,
            "conversation",
        )
        .expect("binding");
    store
        .create_temporary_attachment(context_core::CreateTemporaryAttachmentInput {
            conversation_id: conversation_id.clone(),
            target_type: "memory_space".into(),
            target_id: root_space_id,
            lifetime_mode: "one_prompt".into(),
            remaining_prompts: None,
            expires_at: None,
        })
        .expect("temporary context");
    let state = store
        .conversation_context(&conversation_id)
        .expect("context state");
    assert_eq!(state.bindings.len(), 1);
    assert_eq!(state.temporary_attachments[0].remaining_prompts, Some(1));
    let composed = store
        .compose_context(ComposeContextInput {
            memory_ids: vec![],
            memory_space_ids: vec![],
            context_pack_ids: vec![],
            sources: vec![],
            conversation_id: Some(conversation_id.clone()),
        })
        .expect("compose conversation context");
    assert_eq!(composed.item_count, 1);
    assert!(composed.text.contains("Mode: summary"));
    assert!(composed.estimated_tokens > 0);
    assert!(
        store
            .consume_prompt(&conversation_id)
            .expect("consume prompt")
            .is_empty()
    );
}

#[test]
fn memory_versions_are_append_only_and_restore_creates_a_version() {
    let (_directory, store) = store();
    let (project_id, space_id) = project_and_space(&store);
    let memory = store
        .create_memory(CreateMemoryInput {
            project_id: Some(project_id),
            memory_space_id: Some(space_id),
            memory_type: "decision".into(),
            authority: "user_confirmed".into(),
            status: "active".into(),
            title: "Database".into(),
            content: "Use SQLite".into(),
        })
        .expect("create memory");
    let original_version = memory.current_version_id.clone();
    store
        .append_memory_version(AppendVersionInput {
            memory_id: memory.id.clone(),
            content: "Use SQLite with WAL".into(),
            change_type: "replace".into(),
            next_status: None,
        })
        .expect("append version");
    store
        .restore_memory_version(&memory.id, &original_version)
        .expect("restore version");

    let versions = store
        .list_memory_versions(&memory.id)
        .expect("list versions");
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].change_type, "create");
    assert_eq!(versions[1].change_type, "replace");
    assert_eq!(versions[2].change_type, "restore");
    assert_eq!(
        store
            .get_memory(&memory.id)
            .expect("get memory")
            .current_content,
        "Use SQLite"
    );
}

#[test]
fn failed_memory_creation_leaves_no_partial_rows() {
    let (_directory, store) = store();
    let result = store.create_memory(CreateMemoryInput {
        project_id: None,
        memory_space_id: Some(Uuid::new_v4().to_string()),
        memory_type: "decision".into(),
        authority: "user_confirmed".into(),
        status: "active".into(),
        title: "Invalid".into(),
        content: "Must not persist".into(),
    });
    assert!(result.is_err());
    assert!(store.snapshot().expect("snapshot").memories.is_empty());
}

#[test]
fn backup_opens_as_an_independent_healthy_store() {
    let (directory, store) = store();
    let (project_id, _) = project_and_space(&store);
    store
        .create_context_pack(CreateContextPackInput {
            project_id: Some(project_id),
            name: "Release context".into(),
            description: "Test pack".into(),
        })
        .expect("create pack");
    let backup_path = directory.path().join("backup.db");
    store.backup(&backup_path).expect("create backup");
    let restored = ContextStore::open(&backup_path).expect("open backup");
    assert_eq!(
        restored.snapshot().expect("snapshot").context_packs.len(),
        1
    );
}

#[test]
fn exports_json_and_markdown_without_overwriting() {
    let (directory, store) = store();
    project_and_space(&store);
    let json_path = directory.path().join("export.json");
    let markdown_path = directory.path().join("export.md");
    store.export_json(&json_path).expect("json export");
    store
        .export_markdown(&markdown_path)
        .expect("markdown export");
    assert!(
        fs::read_to_string(json_path)
            .expect("read json")
            .contains("tf0000-context/v1")
    );
    assert!(
        fs::read_to_string(markdown_path)
            .expect("read markdown")
            .contains("# TF0000 Context Export")
    );
}

#[test]
fn prompt_consumption_expires_counted_attachments() {
    let (_directory, store) = store();
    let (project_id, _) = project_and_space(&store);
    let pack = store
        .create_context_pack(CreateContextPackInput {
            project_id: Some(project_id.clone()),
            name: "Temporary".into(),
            description: String::new(),
        })
        .expect("create pack");
    let conversation_id = Uuid::new_v4().to_string();
    let connection = Connection::open(store.database_path()).expect("open sqlite");
    connection
        .execute("PRAGMA foreign_keys = ON", [])
        .expect("foreign keys");
    connection
        .execute(
            "INSERT INTO conversations (id, project_id, provider, title, captured_at) VALUES (?1, ?2, 'manual', 'Test', '2026-01-01T00:00:00Z')",
            params![conversation_id, project_id],
        )
        .expect("create conversation");
    store
        .create_temporary_attachment(context_core::CreateTemporaryAttachmentInput {
            conversation_id: conversation_id.clone(),
            target_type: "context_pack".into(),
            target_id: pack.id,
            lifetime_mode: "n_prompts".into(),
            remaining_prompts: Some(2),
            expires_at: None,
        })
        .expect("attach");
    let after_one = store.consume_prompt(&conversation_id).expect("consume one");
    assert_eq!(after_one[0].remaining_prompts, Some(1));
    assert!(
        store
            .consume_prompt(&conversation_id)
            .expect("consume two")
            .is_empty()
    );
}

#[test]
fn corrupted_database_is_rejected() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("corrupt.db");
    fs::write(&path, b"not a sqlite database").expect("write corrupt database");
    assert!(ContextStore::open(path).is_err());
}

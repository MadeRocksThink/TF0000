use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use context_core::{
    AppendBranchVersionInput, AppendVersionInput, ApplyMemoryUpdateInput,
    ApplyPortableWorkspaceInput, AskMemoryInput, AskMemoryResponse, BackupSettings, CapturedSource,
    ConflictExplanation, ContextPack, ContextRecommendation, ContextRecommendationInput,
    ContextStore, ConversationHandoff, CreateContextPackInput, CreateMemoryBranchInput,
    CreateMemoryInput, CreateMemorySpaceInput, CreateProjectInput, DashboardSnapshot,
    DiagnosticReport, ExportPortableWorkspaceInput, ExtractCandidatesInput, GenerateSummaryInput,
    GeneratedArtifact, HandoffInput, HandoffPreview, HealthReport, ImportInput, ImportPreview,
    ImportResult, Memory, MemoryBranch, MemoryConflict, MemoryHistory, MemorySpace,
    MemoryUpdatePreview, MemoryUpdateResult, MemoryVersion, PortableWorkspace,
    PreviewMemoryUpdateInput, Project, ResolveSyncConflictInput, RestoreResult,
    ReviewSmartCandidateInput, ScheduledBackupResult, SearchInput, SearchResponse, SecretWarning,
    SmartBenchmarkReport, SmartCandidate, SmartCandidateReview, SmartSettings, SyncApplyResult,
    SyncConflict, UpdateBackupSettingsInput, UpdateSmartSettingsInput,
};
use serde::Serialize;
use tauri::{Manager, State};

struct AppState {
    store: ContextStore,
}

type CommandResult<T> = std::result::Result<T, String>;

fn command_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command]
fn dashboard_snapshot(state: State<'_, AppState>) -> CommandResult<DashboardSnapshot> {
    state.store.snapshot().map_err(command_error)
}

#[tauri::command]
fn health_check(state: State<'_, AppState>) -> CommandResult<HealthReport> {
    state.store.health().map_err(command_error)
}

#[tauri::command]
fn search_context(state: State<'_, AppState>, input: SearchInput) -> CommandResult<SearchResponse> {
    state.store.search(input).map_err(command_error)
}

#[tauri::command]
fn ask_memory(
    state: State<'_, AppState>,
    input: AskMemoryInput,
) -> CommandResult<AskMemoryResponse> {
    state.store.ask_memory(input).map_err(command_error)
}

#[tauri::command]
fn detect_secrets(state: State<'_, AppState>, text: String) -> Vec<SecretWarning> {
    state.store.detect_secrets(&text)
}

#[tauri::command]
fn preview_import(state: State<'_, AppState>, input: ImportInput) -> CommandResult<ImportPreview> {
    state.store.preview_import(input).map_err(command_error)
}

#[tauri::command]
fn import_file(state: State<'_, AppState>, input: ImportInput) -> CommandResult<ImportResult> {
    state.store.import_file(input).map_err(command_error)
}

#[tauri::command]
fn backup_settings(state: State<'_, AppState>) -> CommandResult<BackupSettings> {
    state.store.backup_settings().map_err(command_error)
}

#[tauri::command]
fn update_backup_settings(
    state: State<'_, AppState>,
    input: UpdateBackupSettingsInput,
) -> CommandResult<BackupSettings> {
    state
        .store
        .update_backup_settings(input)
        .map_err(command_error)
}

#[tauri::command]
fn run_scheduled_backup(
    state: State<'_, AppState>,
    force: bool,
) -> CommandResult<ScheduledBackupResult> {
    state
        .store
        .run_scheduled_backup(force)
        .map_err(command_error)
}

#[tauri::command]
fn restore_backup(state: State<'_, AppState>, source: String) -> CommandResult<RestoreResult> {
    state
        .store
        .restore_backup(PathBuf::from(source))
        .map_err(command_error)
}

#[tauri::command]
fn diagnostics(state: State<'_, AppState>) -> CommandResult<DiagnosticReport> {
    state.store.diagnostics().map_err(command_error)
}

#[tauri::command]
fn export_diagnostics(state: State<'_, AppState>, destination: String) -> CommandResult<String> {
    state
        .store
        .export_diagnostics(PathBuf::from(destination))
        .map(|path| path.display().to_string())
        .map_err(command_error)
}

#[tauri::command]
fn smart_settings(state: State<'_, AppState>) -> CommandResult<SmartSettings> {
    state.store.smart_settings().map_err(command_error)
}

#[tauri::command]
fn update_smart_settings(
    state: State<'_, AppState>,
    input: UpdateSmartSettingsInput,
) -> CommandResult<SmartSettings> {
    state
        .store
        .update_smart_settings(input)
        .map_err(command_error)
}

#[tauri::command]
fn rebuild_semantic_index(state: State<'_, AppState>) -> CommandResult<usize> {
    state.store.rebuild_semantic_index().map_err(command_error)
}

#[tauri::command]
fn generate_summary(
    state: State<'_, AppState>,
    input: GenerateSummaryInput,
) -> CommandResult<GeneratedArtifact> {
    state.store.generate_summary(input).map_err(command_error)
}

#[tauri::command]
fn generated_artifacts(
    state: State<'_, AppState>,
    source_id: Option<String>,
) -> CommandResult<Vec<GeneratedArtifact>> {
    state
        .store
        .generated_artifacts(source_id.as_deref())
        .map_err(command_error)
}

#[tauri::command]
fn extract_smart_candidates(
    state: State<'_, AppState>,
    input: ExtractCandidatesInput,
) -> CommandResult<Vec<SmartCandidate>> {
    state.store.extract_candidates(input).map_err(command_error)
}

#[tauri::command]
fn smart_candidates(
    state: State<'_, AppState>,
    status: Option<String>,
) -> CommandResult<Vec<SmartCandidate>> {
    state
        .store
        .smart_candidates(status.as_deref())
        .map_err(command_error)
}

#[tauri::command]
fn review_smart_candidate(
    state: State<'_, AppState>,
    input: ReviewSmartCandidateInput,
) -> CommandResult<SmartCandidateReview> {
    state
        .store
        .review_smart_candidate(input)
        .map_err(command_error)
}

#[tauri::command]
fn explain_conflict(
    state: State<'_, AppState>,
    conflict_id: String,
) -> CommandResult<ConflictExplanation> {
    state
        .store
        .explain_conflict(&conflict_id)
        .map_err(command_error)
}

#[tauri::command]
fn recommend_context(
    state: State<'_, AppState>,
    input: ContextRecommendationInput,
) -> CommandResult<ContextRecommendation> {
    state.store.recommend_context(input).map_err(command_error)
}

#[tauri::command]
fn benchmark_local_embeddings(state: State<'_, AppState>) -> CommandResult<SmartBenchmarkReport> {
    state
        .store
        .benchmark_local_embeddings()
        .map_err(command_error)
}

#[tauri::command]
fn preview_handoff(
    state: State<'_, AppState>,
    input: HandoffInput,
) -> CommandResult<HandoffPreview> {
    state.store.preview_handoff(input).map_err(command_error)
}

#[tauri::command]
fn conversation_handoffs(
    state: State<'_, AppState>,
    source_conversation_id: Option<String>,
) -> CommandResult<Vec<ConversationHandoff>> {
    state
        .store
        .list_conversation_handoffs(source_conversation_id.as_deref())
        .map_err(command_error)
}

#[tauri::command]
fn open_source_url(url: String) -> CommandResult<()> {
    let url = url.trim();
    if url.len() > 4_000
        || !(url.starts_with("https://") || url.starts_with("http://"))
        || url.chars().any(char::is_control)
    {
        return Err("source URL must be a valid HTTP or HTTPS address".into());
    }
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler").arg(url);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open source URL: {error}"))
}

#[tauri::command]
fn create_project(state: State<'_, AppState>, input: CreateProjectInput) -> CommandResult<Project> {
    state.store.create_project(input).map_err(command_error)
}

#[tauri::command]
fn create_memory_space(
    state: State<'_, AppState>,
    input: CreateMemorySpaceInput,
) -> CommandResult<MemorySpace> {
    state
        .store
        .create_memory_space(input)
        .map_err(command_error)
}

#[tauri::command]
fn move_memory_space(
    state: State<'_, AppState>,
    space_id: String,
    parent_id: Option<String>,
) -> CommandResult<()> {
    state
        .store
        .move_memory_space(&space_id, parent_id.as_deref())
        .map_err(command_error)
}

#[tauri::command]
fn move_memory_to_space(
    state: State<'_, AppState>,
    memory_id: String,
    space_id: Option<String>,
) -> CommandResult<()> {
    state
        .store
        .move_memory_to_space(&memory_id, space_id.as_deref())
        .map_err(command_error)
}

#[tauri::command]
fn create_memory(state: State<'_, AppState>, input: CreateMemoryInput) -> CommandResult<Memory> {
    state.store.create_memory(input).map_err(command_error)
}

#[tauri::command]
fn append_memory_version(
    state: State<'_, AppState>,
    input: AppendVersionInput,
) -> CommandResult<MemoryVersion> {
    state
        .store
        .append_memory_version(input)
        .map_err(command_error)
}

#[tauri::command]
fn captured_sources(state: State<'_, AppState>) -> CommandResult<Vec<CapturedSource>> {
    state.store.list_captured_sources().map_err(command_error)
}

#[tauri::command]
fn preview_memory_update(
    state: State<'_, AppState>,
    input: PreviewMemoryUpdateInput,
) -> CommandResult<MemoryUpdatePreview> {
    state
        .store
        .preview_memory_update(input)
        .map_err(command_error)
}

#[tauri::command]
fn apply_memory_update(
    state: State<'_, AppState>,
    input: ApplyMemoryUpdateInput,
) -> CommandResult<MemoryUpdateResult> {
    state
        .store
        .apply_memory_update(input)
        .map_err(command_error)
}

#[tauri::command]
fn memory_history(state: State<'_, AppState>, memory_id: String) -> CommandResult<MemoryHistory> {
    state
        .store
        .memory_history(&memory_id)
        .map_err(command_error)
}

#[tauri::command]
fn restore_memory_version(
    state: State<'_, AppState>,
    memory_id: String,
    version_id: String,
) -> CommandResult<MemoryVersion> {
    state
        .store
        .restore_memory_version(&memory_id, &version_id)
        .map_err(command_error)
}

#[tauri::command]
fn create_memory_branch(
    state: State<'_, AppState>,
    input: CreateMemoryBranchInput,
) -> CommandResult<MemoryBranch> {
    state
        .store
        .create_memory_branch(input)
        .map_err(command_error)
}

#[tauri::command]
fn append_branch_version(
    state: State<'_, AppState>,
    input: AppendBranchVersionInput,
) -> CommandResult<MemoryBranch> {
    state
        .store
        .append_branch_version(input)
        .map_err(command_error)
}

#[tauri::command]
fn memory_branches(
    state: State<'_, AppState>,
    memory_id: Option<String>,
) -> CommandResult<Vec<MemoryBranch>> {
    state
        .store
        .list_memory_branches(memory_id.as_deref())
        .map_err(command_error)
}

#[tauri::command]
fn finalize_memory_branch(
    state: State<'_, AppState>,
    branch_id: String,
    mode: String,
) -> CommandResult<MemoryBranch> {
    state
        .store
        .finalize_memory_branch(&branch_id, &mode)
        .map_err(command_error)
}

#[tauri::command]
fn memory_conflicts(state: State<'_, AppState>) -> CommandResult<Vec<MemoryConflict>> {
    state
        .store
        .list_memory_conflicts(None, "unresolved")
        .map_err(command_error)
}

#[tauri::command]
fn resolve_memory_conflict(
    state: State<'_, AppState>,
    conflict_id: String,
    resolution: String,
) -> CommandResult<()> {
    state
        .store
        .resolve_memory_conflict(&conflict_id, &resolution)
        .map_err(command_error)
}

#[tauri::command]
fn create_context_pack(
    state: State<'_, AppState>,
    input: CreateContextPackInput,
) -> CommandResult<ContextPack> {
    state
        .store
        .create_context_pack(input)
        .map_err(command_error)
}

#[tauri::command]
fn export_json(state: State<'_, AppState>, destination: String) -> CommandResult<String> {
    state
        .store
        .export_json(PathBuf::from(destination))
        .map(|path| path.display().to_string())
        .map_err(command_error)
}

#[tauri::command]
fn export_markdown(state: State<'_, AppState>, destination: String) -> CommandResult<String> {
    state
        .store
        .export_markdown(PathBuf::from(destination))
        .map(|path| path.display().to_string())
        .map_err(command_error)
}

#[tauri::command]
fn backup_database(state: State<'_, AppState>, destination: String) -> CommandResult<String> {
    state
        .store
        .backup(PathBuf::from(destination))
        .map(|path| path.display().to_string())
        .map_err(command_error)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncEnvelopeEntry {
    path: String,
    device_id: String,
    created_at: String,
    size: u64,
}

#[tauri::command]
fn export_portable_workspace(
    state: State<'_, AppState>,
    input: ExportPortableWorkspaceInput,
) -> CommandResult<PortableWorkspace> {
    state
        .store
        .export_portable_workspace(input)
        .map_err(command_error)
}

#[tauri::command]
fn apply_portable_workspace(
    state: State<'_, AppState>,
    input: ApplyPortableWorkspaceInput,
) -> CommandResult<SyncApplyResult> {
    state
        .store
        .apply_portable_workspace(input)
        .map_err(command_error)
}

#[tauri::command]
fn sync_conflicts(state: State<'_, AppState>) -> CommandResult<Vec<SyncConflict>> {
    state
        .store
        .list_sync_conflicts("unresolved")
        .map_err(command_error)
}

#[tauri::command]
fn resolve_sync_conflict(
    state: State<'_, AppState>,
    input: ResolveSyncConflictInput,
) -> CommandResult<SyncConflict> {
    state
        .store
        .resolve_sync_conflict(input)
        .map_err(command_error)
}

#[tauri::command]
fn write_sync_envelope(
    directory: String,
    device_id: String,
    envelope: String,
) -> CommandResult<String> {
    let directory = absolute_existing_directory(&directory)?;
    let safe_device = safe_device_id(&device_id)?;
    if envelope.len() > 125 * 1024 * 1024 {
        return Err("encrypted sync envelope exceeds 125 MB".into());
    }
    let value: serde_json::Value = serde_json::from_str(&envelope).map_err(command_error)?;
    if value.get("format").and_then(|item| item.as_str()) != Some("tf0000-sync/v1")
        || value.get("deviceId").and_then(|item| item.as_str()) != Some(device_id.as_str())
    {
        return Err("encrypted sync metadata is invalid".into());
    }
    let destination = directory.join(format!("{safe_device}.tf0000sync"));
    fs::write(&destination, envelope.as_bytes()).map_err(command_error)?;
    Ok(destination.display().to_string())
}

#[tauri::command]
fn list_sync_envelopes(directory: String) -> CommandResult<Vec<SyncEnvelopeEntry>> {
    let directory = absolute_existing_directory(&directory)?;
    let mut entries = Vec::new();
    for item in fs::read_dir(directory).map_err(command_error)? {
        let item = item.map_err(command_error)?;
        let path = item.path();
        if path.extension().and_then(|value| value.to_str()) != Some("tf0000sync") {
            continue;
        }
        let metadata = item.metadata().map_err(command_error)?;
        if !metadata.is_file() || metadata.len() > 125 * 1024 * 1024 {
            continue;
        }
        let value: serde_json::Value =
            match serde_json::from_slice(&fs::read(&path).map_err(command_error)?) {
                Ok(value) => value,
                Err(_) => continue,
            };
        if value.get("format").and_then(|item| item.as_str()) != Some("tf0000-sync/v1") {
            continue;
        }
        entries.push(SyncEnvelopeEntry {
            path: path.display().to_string(),
            device_id: value
                .get("deviceId")
                .and_then(|item| item.as_str())
                .unwrap_or("Unknown device")
                .to_owned(),
            created_at: value
                .get("createdAt")
                .and_then(|item| item.as_str())
                .unwrap_or("")
                .to_owned(),
            size: metadata.len(),
        });
    }
    entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(entries)
}

#[tauri::command]
fn read_sync_envelope(path: String) -> CommandResult<String> {
    let path = PathBuf::from(path);
    if !path.is_absolute()
        || path.extension().and_then(|value| value.to_str()) != Some("tf0000sync")
    {
        return Err("sync file must be an absolute .tf0000sync path".into());
    }
    let metadata = fs::metadata(&path).map_err(command_error)?;
    if !metadata.is_file() || metadata.len() > 125 * 1024 * 1024 {
        return Err("sync file is invalid or too large".into());
    }
    fs::read_to_string(path).map_err(command_error)
}

fn absolute_existing_directory(value: &str) -> CommandResult<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || !path.is_dir() {
        return Err("sync folder must be an existing absolute directory".into());
    }
    Ok(path)
}

fn safe_device_id(value: &str) -> CommandResult<String> {
    let trimmed = value.trim();
    if trimmed.len() < 2 || trimmed.len() > 64 {
        return Err("device name must contain 2–64 characters".into());
    }
    if !trimmed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(
            "device name may contain only letters, numbers, hyphens, and underscores".into(),
        );
    }
    let upper = trimmed.to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    ) {
        return Err("device name is reserved by Windows".into());
    }
    Ok(trimmed.to_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_directory = app.path().app_local_data_dir()?;
            let database = data_directory.join("context.db");
            migrate_legacy_database(&data_directory, &database)?;
            let store = ContextStore::open(database)
                .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            app.manage(AppState { store });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dashboard_snapshot,
            health_check,
            search_context,
            ask_memory,
            detect_secrets,
            preview_import,
            import_file,
            backup_settings,
            update_backup_settings,
            run_scheduled_backup,
            restore_backup,
            diagnostics,
            export_diagnostics,
            smart_settings,
            update_smart_settings,
            rebuild_semantic_index,
            generate_summary,
            generated_artifacts,
            extract_smart_candidates,
            smart_candidates,
            review_smart_candidate,
            explain_conflict,
            recommend_context,
            benchmark_local_embeddings,
            preview_handoff,
            conversation_handoffs,
            open_source_url,
            create_project,
            create_memory_space,
            move_memory_space,
            move_memory_to_space,
            create_memory,
            append_memory_version,
            captured_sources,
            preview_memory_update,
            apply_memory_update,
            memory_history,
            restore_memory_version,
            create_memory_branch,
            append_branch_version,
            memory_branches,
            finalize_memory_branch,
            memory_conflicts,
            resolve_memory_conflict,
            create_context_pack,
            export_json,
            export_markdown,
            backup_database,
            export_portable_workspace,
            apply_portable_workspace,
            sync_conflicts,
            resolve_sync_conflict,
            write_sync_envelope,
            list_sync_envelopes,
            read_sync_envelope
        ])
        .run(tauri::generate_context!())
        .expect("failed to run TF0000");
}

fn migrate_legacy_database(
    data_directory: &Path,
    destination: &Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if destination.exists() {
        return Ok(());
    }
    let Some(data_root) = data_directory.parent() else {
        return Ok(());
    };
    let legacy = data_root.join("com.crossai.context").join("context.db");
    if legacy.exists() {
        ContextStore::open(legacy)?.backup(destination)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_envelopes_are_scoped_to_safe_device_files() {
        let directory = tempfile::tempdir().expect("temporary sync folder");
        let envelope = serde_json::json!({
            "format": "tf0000-sync/v1",
            "deviceId": "work-laptop",
            "createdAt": "2026-09-19T00:00:00Z",
            "ciphertext": "opaque"
        })
        .to_string();
        let path = write_sync_envelope(
            directory.path().display().to_string(),
            "work-laptop".into(),
            envelope.clone(),
        )
        .expect("write encrypted envelope");
        assert_eq!(read_sync_envelope(path).unwrap(), envelope);
        let entries = list_sync_envelopes(directory.path().display().to_string()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].device_id, "work-laptop");
        assert!(safe_device_id("../escape").is_err());
        assert!(safe_device_id("CON").is_err());
    }
}

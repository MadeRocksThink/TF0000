use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySpace {
    pub id: String,
    pub project_id: Option<String>,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: String,
    pub default_scope: String,
    pub created_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySpaceLink {
    pub memory_id: String,
    pub memory_space_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub id: String,
    pub project_id: Option<String>,
    pub memory_type: String,
    pub authority: String,
    pub status: String,
    pub title: String,
    pub current_version_id: String,
    pub current_content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryVersion {
    pub id: String,
    pub memory_id: String,
    pub content: String,
    pub change_type: String,
    pub created_at: String,
    pub supersedes_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextPack {
    pub id: String,
    pub project_id: Option<String>,
    pub name: String,
    pub description: String,
    pub current_version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TemporaryAttachment {
    pub id: String,
    pub conversation_id: String,
    pub target_type: String,
    pub target_id: String,
    pub lifetime_mode: String,
    pub remaining_prompts: Option<i64>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub projects: Vec<Project>,
    pub memory_spaces: Vec<MemorySpace>,
    pub memories: Vec<Memory>,
    pub context_packs: Vec<ContextPack>,
    pub memory_space_links: Vec<MemorySpaceLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub status: String,
    pub schema_version: i64,
    pub sqlite_version: String,
    pub database_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchInput {
    pub query: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub memory_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub result_type: String,
    pub id: String,
    pub parent_id: Option<String>,
    pub project_id: Option<String>,
    pub title: String,
    pub excerpt: String,
    pub provider: Option<String>,
    pub memory_type: Option<String>,
    pub authority: Option<String>,
    pub status: Option<String>,
    pub source_url: Option<String>,
    pub created_at: String,
    pub is_current: bool,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub query: String,
    pub result_count: usize,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AskMemoryInput {
    pub query: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AskMemoryResponse {
    pub status: String,
    pub message: String,
    pub evidence: Vec<SearchResult>,
    pub current_decisions: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CapturedMessageInput {
    pub external_ref: Option<String>,
    pub role: String,
    pub speaker: Option<String>,
    pub body: String,
    pub ordinal: i64,
    pub sent_at: Option<String>,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CapturedFragmentInput {
    pub message_external_ref: Option<String>,
    pub message_source_hash: Option<String>,
    pub start_offset: Option<i64>,
    pub end_offset: Option<i64>,
    pub selected_text: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CaptureConversationInput {
    pub provider: String,
    pub external_ref: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub captured_at: String,
    #[serde(default)]
    pub messages: Vec<CapturedMessageInput>,
    #[serde(default)]
    pub fragments: Vec<CapturedFragmentInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub conversation_id: String,
    pub saved_messages: usize,
    pub saved_fragments: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComposedContext {
    pub text: String,
    pub item_count: usize,
    pub character_count: usize,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandoffItem {
    pub item_type: String,
    pub item_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandoffPreview {
    pub source_conversation_id: String,
    pub source_provider: String,
    pub source_title: String,
    pub destination_provider: String,
    pub mode: String,
    pub text: String,
    pub content_hash: String,
    pub items: Vec<HandoffItem>,
    pub item_count: usize,
    pub character_count: usize,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationHandoff {
    pub id: String,
    pub source_conversation_id: String,
    pub destination_provider: String,
    pub destination_conversation_id: Option<String>,
    pub mode: String,
    pub content_hash: String,
    pub item_count: usize,
    pub character_count: usize,
    pub estimated_tokens: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffInput {
    pub source_provider: String,
    pub source_external_ref: String,
    pub destination_provider: String,
    pub mode: String,
    #[serde(default)]
    pub recent_message_count: Option<usize>,
    #[serde(default)]
    pub include_current_task: bool,
    #[serde(default)]
    pub include_active_decisions: bool,
    #[serde(default)]
    pub memory_ids: Vec<String>,
    #[serde(default)]
    pub context_pack_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapturedSource {
    pub id: String,
    pub source_type: String,
    pub provider: String,
    pub conversation_title: String,
    pub role: String,
    pub content: String,
    pub source_url: Option<String>,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemorySource {
    pub memory_version_id: String,
    pub source_type: String,
    pub source_id: String,
    pub source_hash: String,
    pub source_label: String,
    pub source_excerpt: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MergeSourceInput {
    pub source_type: String,
    pub source_id: Option<String>,
    pub content: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedMergeSource {
    pub source_type: String,
    pub source_id: String,
    pub label: String,
    pub content: String,
    pub source_hash: String,
    pub duplicate_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDiff {
    pub before: String,
    pub after: String,
    pub added_lines: usize,
    pub removed_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConflictCandidate {
    pub conflicting_memory_id: String,
    pub conflicting_memory_title: String,
    pub conflict_key: String,
    pub current_value: String,
    pub conflicting_value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewMemoryUpdateInput {
    pub memory_id: Option<String>,
    pub action: String,
    pub sources: Vec<MergeSourceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryUpdatePreview {
    pub memory_id: Option<String>,
    pub current_version_id: Option<String>,
    pub action: String,
    pub sources: Vec<ResolvedMergeSource>,
    pub unique_source_count: usize,
    pub duplicate_count: usize,
    pub diff: UpdateDiff,
    pub conflicts: Vec<ConflictCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyMemoryUpdateInput {
    pub memory_id: Option<String>,
    pub expected_current_version_id: Option<String>,
    pub project_id: Option<String>,
    pub memory_space_id: Option<String>,
    pub memory_type: Option<String>,
    pub authority: Option<String>,
    pub title: Option<String>,
    pub action: String,
    pub sources: Vec<MergeSourceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryUpdateResult {
    pub memory: Memory,
    pub version: MemoryVersion,
    pub conflicts: Vec<MemoryConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryHistory {
    pub memory: Memory,
    pub versions: Vec<MemoryVersion>,
    pub sources: Vec<MemorySource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryBranch {
    pub id: String,
    pub memory_id: String,
    pub base_version_id: String,
    pub name: String,
    pub status: String,
    pub current_version_id: String,
    pub current_content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateMemoryBranchInput {
    pub memory_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppendBranchVersionInput {
    pub branch_id: String,
    pub content: String,
    pub change_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryConflict {
    pub id: String,
    pub memory_id: String,
    pub memory_title: String,
    pub conflicting_memory_id: String,
    pub conflicting_memory_title: String,
    pub conflict_key: String,
    pub current_value: String,
    pub conflicting_value: String,
    pub status: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SourceReference {
    pub id: String,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextPackItem {
    pub target_id: String,
    pub ordering: i64,
    pub inclusion_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextPackDetail {
    pub pack: ContextPack,
    pub items: Vec<ContextPackItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveContextPackInput {
    pub id: Option<String>,
    pub project_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub items: Vec<ContextPackItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContextBinding {
    pub target_type: String,
    pub target_id: String,
    pub enabled: bool,
    pub lifetime_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationContextState {
    pub conversation_id: String,
    pub bindings: Vec<ContextBinding>,
    pub temporary_attachments: Vec<TemporaryAttachment>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposeContextInput {
    #[serde(default)]
    pub memory_ids: Vec<String>,
    #[serde(default)]
    pub memory_space_ids: Vec<String>,
    #[serde(default)]
    pub context_pack_ids: Vec<String>,
    #[serde(default)]
    pub sources: Vec<SourceReference>,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportMemory {
    pub memory: Memory,
    pub versions: Vec<MemoryVersion>,
    pub sources: Vec<MemorySource>,
    pub memory_space_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub format: String,
    pub schema_version: i64,
    pub exported_at: String,
    pub projects: Vec<Project>,
    pub memory_spaces: Vec<MemorySpace>,
    pub memories: Vec<ExportMemory>,
    pub context_packs: Vec<ContextPack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableWorkspace {
    pub format: String,
    pub schema_version: i64,
    pub exported_at: String,
    pub source_device_id: String,
    pub projects: Vec<Project>,
    pub memory_spaces: Vec<MemorySpace>,
    pub memories: Vec<ExportMemory>,
    pub context_packs: Vec<ContextPackDetail>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportPortableWorkspaceInput {
    pub project_ids: Vec<String>,
    #[serde(default)]
    pub include_global: bool,
    pub source_device_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyPortableWorkspaceInput {
    pub workspace: PortableWorkspace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncApplyResult {
    pub added: usize,
    pub fast_forwarded: usize,
    pub unchanged: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflict {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub project_id: Option<String>,
    pub local_value: String,
    pub remote_value: String,
    pub remote_device_id: String,
    pub status: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveSyncConflictInput {
    pub conflict_id: String,
    pub resolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecretWarning {
    pub kind: String,
    pub line: usize,
    pub redacted_excerpt: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretScanInput {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportInput {
    pub source_path: String,
    pub project_id: Option<String>,
    pub memory_space_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidate {
    pub title: String,
    pub content: String,
    pub memory_type: String,
    pub authority: String,
    pub status: String,
    pub content_hash: String,
    pub duplicate: bool,
    pub secret_warnings: Vec<SecretWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub source_label: String,
    pub source_format: String,
    pub source_hash: String,
    pub candidates: Vec<ImportCandidate>,
    pub importable_count: usize,
    pub duplicate_count: usize,
    pub secret_warning_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub import_id: String,
    pub source_label: String,
    pub imported_count: usize,
    pub skipped_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub enabled: bool,
    pub interval_hours: i64,
    pub directory: String,
    pub last_backup_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateBackupSettingsInput {
    pub enabled: bool,
    pub interval_hours: i64,
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledBackupResult {
    pub created: bool,
    pub path: Option<String>,
    pub reason: String,
    pub settings: BackupSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored_from: String,
    pub recovery_backup_path: String,
    pub schema_version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub generated_at: String,
    pub application: String,
    pub status: String,
    pub schema_version: i64,
    pub sqlite_version: String,
    pub database_path: String,
    pub integrity: String,
    pub project_count: i64,
    pub conversation_count: i64,
    pub message_count: i64,
    pub memory_count: i64,
    pub context_pack_count: i64,
    pub unresolved_conflict_count: i64,
    pub handoff_count: i64,
    pub import_count: i64,
    pub backup_settings: BackupSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SmartSettings {
    pub mode: String,
    pub provider_name: String,
    pub provider_endpoint: String,
    pub provider_model: String,
    pub recommendation_mode: String,
    pub embedding_model: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSmartSettingsInput {
    pub mode: String,
    #[serde(default)]
    pub provider_name: String,
    #[serde(default)]
    pub provider_endpoint: String,
    #[serde(default)]
    pub provider_model: String,
    pub recommendation_mode: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateSummaryInput {
    pub source_type: String,
    pub source_id: String,
    #[serde(default)]
    pub maximum_characters: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedArtifact {
    pub id: String,
    pub source_type: String,
    pub source_id: String,
    pub artifact_type: String,
    pub content: String,
    pub model_mode: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractCandidatesInput {
    pub source_type: String,
    pub source_id: String,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SmartCandidate {
    pub id: String,
    pub project_id: Option<String>,
    pub source_type: String,
    pub source_id: String,
    pub memory_type: String,
    pub title: String,
    pub content: String,
    pub confidence: f64,
    pub status: String,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewSmartCandidateInput {
    pub candidate_id: String,
    pub action: String,
    pub memory_space_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SmartCandidateReview {
    pub candidate: SmartCandidate,
    pub memory: Option<Memory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConflictExplanationSource {
    pub memory_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConflictExplanation {
    pub artifact: GeneratedArtifact,
    pub conflict_key: String,
    pub sources: Vec<ConflictExplanationSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextRecommendationInput {
    pub prompt: String,
    pub project_id: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextRecommendation {
    pub mode: String,
    pub requires_confirmation: bool,
    pub message: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingBenchmark {
    pub profile: String,
    pub dimensions: usize,
    pub documents: usize,
    pub elapsed_milliseconds: f64,
    pub documents_per_second: f64,
    pub estimated_bytes_per_document: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SmartBenchmarkReport {
    pub device: String,
    pub model_family: String,
    pub recommended_profile: String,
    pub benchmarks: Vec<EmbeddingBenchmark>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMapping {
    pub workspace_uri: String,
    pub repository_root: String,
    pub project_id: String,
    pub project_name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapWorkspaceInput {
    pub workspace_uri: String,
    pub repository_root: String,
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeReference {
    pub id: String,
    pub project_id: String,
    pub workspace_uri: String,
    pub repository_root: String,
    pub relative_path: String,
    pub language: String,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub content: String,
    pub content_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveCodeReferenceInput {
    pub workspace_uri: String,
    pub relative_path: String,
    pub language: String,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectContextInput {
    pub project_id: String,
    #[serde(default)]
    pub maximum_characters: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContext {
    pub project_id: String,
    pub project_name: String,
    pub text: String,
    pub memory_count: usize,
    pub code_reference_count: usize,
    pub character_count: usize,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchDecisionsInput {
    pub project_id: String,
    pub query: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentWriteCandidate {
    pub id: String,
    pub project_id: String,
    pub requested_by: String,
    pub memory_type: String,
    pub title: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
    pub reviewed_at: Option<String>,
    pub memory_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAgentWriteCandidateInput {
    pub project_id: String,
    pub requested_by: String,
    pub memory_type: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewAgentWriteCandidateInput {
    pub candidate_id: String,
    pub action: String,
    pub confirmed: bool,
    pub memory_space_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchMemoriesInput {
    pub project_id: String,
    pub query: String,
    #[serde(default)]
    pub memory_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchChatsInput {
    pub project_id: String,
    pub query: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDecisionsInput {
    pub project_id: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpClient {
    pub id: String,
    pub display_name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterMcpClientInput {
    pub client_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpPermission {
    pub client_id: String,
    pub project_id: Option<String>,
    pub read_allowed: bool,
    pub candidate_write_allowed: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetMcpPermissionInput {
    pub client_id: String,
    pub project_id: Option<String>,
    pub read_allowed: bool,
    pub candidate_write_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpAuditEntry {
    pub id: String,
    pub client_id: String,
    pub tool_name: String,
    pub access_type: String,
    pub project_id: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub outcome: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordMcpAuditInput {
    pub client_id: String,
    pub tool_name: String,
    pub access_type: String,
    pub project_id: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub outcome: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemorySpaceInput {
    pub project_id: Option<String>,
    pub parent_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub default_scope: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryInput {
    pub project_id: Option<String>,
    pub memory_space_id: Option<String>,
    pub memory_type: String,
    pub authority: String,
    pub status: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendVersionInput {
    pub memory_id: String,
    pub content: String,
    pub change_type: String,
    pub next_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContextPackInput {
    pub project_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTemporaryAttachmentInput {
    pub conversation_id: String,
    pub target_type: String,
    pub target_id: String,
    pub lifetime_mode: String,
    pub remaining_prompts: Option<i64>,
    pub expires_at: Option<String>,
}

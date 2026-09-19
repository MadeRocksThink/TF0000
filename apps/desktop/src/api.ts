import { invoke } from "@tauri-apps/api/core";
import type { PortableWorkspace } from "@tf0000/context-format";

export interface Project {
  id: string;
  name: string;
  description: string;
  createdAt: string;
  archivedAt: string | null;
}

export interface MemorySpace {
  id: string;
  projectId: string | null;
  parentId: string | null;
  name: string;
  description: string;
  defaultScope: string;
  createdAt: string;
  archivedAt: string | null;
}

export interface Memory {
  id: string;
  projectId: string | null;
  memoryType: string;
  authority: string;
  status: string;
  title: string;
  currentVersionId: string;
  currentContent: string;
  createdAt: string;
}

export interface MemorySpaceLink {
  memoryId: string;
  memorySpaceId: string;
}

export interface ContextPack {
  id: string;
  projectId: string | null;
  name: string;
  description: string;
  currentVersion: number;
  createdAt: string;
  updatedAt: string;
}

export interface DashboardSnapshot {
  projects: Project[];
  memorySpaces: MemorySpace[];
  memories: Memory[];
  contextPacks: ContextPack[];
  memorySpaceLinks: MemorySpaceLink[];
}

export interface HealthReport {
  status: string;
  schemaVersion: number;
  sqliteVersion: string;
  databasePath: string;
}

export interface SearchInput {
  query: string;
  scope: "all" | "global" | "project";
  projectId: string | null;
  provider: string | null;
  dateFrom: string | null;
  dateTo: string | null;
  memoryType: string | null;
  status: string | null;
  limit: number;
  offset: number;
}

export interface SearchResult {
  resultType: "memory" | "message" | "fragment" | "context_pack" | "code_reference";
  id: string;
  parentId: string | null;
  projectId: string | null;
  title: string;
  excerpt: string;
  provider: string | null;
  memoryType: string | null;
  authority: string | null;
  status: string | null;
  sourceUrl: string | null;
  createdAt: string;
  isCurrent: boolean;
  score: number;
}

export interface SearchResponse {
  query: string;
  resultCount: number;
  results: SearchResult[];
}

export interface AskMemoryResponse {
  status: "evidence" | "not_recorded";
  message: string;
  evidence: SearchResult[];
  currentDecisions: SearchResult[];
}

export interface CapturedSource {
  id: string;
  sourceType: "conversation" | "message" | "fragment";
  provider: string;
  conversationTitle: string;
  role: string;
  content: string;
  sourceUrl: string | null;
  capturedAt: string;
}

export interface MergeSourceInput {
  sourceType: "conversation" | "message" | "fragment" | "memory" | "manual";
  sourceId: string | null;
  content: string | null;
  label: string | null;
}

export interface ResolvedMergeSource {
  sourceType: string;
  sourceId: string;
  label: string;
  content: string;
  sourceHash: string;
  duplicateOf: string | null;
}

export interface MemoryUpdatePreview {
  memoryId: string | null;
  currentVersionId: string | null;
  action: "add" | "merge" | "replace" | "supersede";
  sources: ResolvedMergeSource[];
  uniqueSourceCount: number;
  duplicateCount: number;
  diff: {
    before: string;
    after: string;
    addedLines: number;
    removedLines: number;
  };
  conflicts: ConflictCandidate[];
}

export interface ConflictCandidate {
  conflictingMemoryId: string;
  conflictingMemoryTitle: string;
  conflictKey: string;
  currentValue: string;
  conflictingValue: string;
}

export interface MemoryVersion {
  id: string;
  memoryId: string;
  content: string;
  changeType: string;
  createdAt: string;
  supersedesVersionId: string | null;
}

export interface MemorySource {
  memoryVersionId: string;
  sourceType: string;
  sourceId: string;
  sourceHash: string;
  sourceLabel: string;
  sourceExcerpt: string;
}

export interface MemoryHistory {
  memory: Memory;
  versions: MemoryVersion[];
  sources: MemorySource[];
}

export interface MemoryBranch {
  id: string;
  memoryId: string;
  baseVersionId: string;
  name: string;
  status: "active" | "promoted" | "merged" | "abandoned";
  currentVersionId: string;
  currentContent: string;
  createdAt: string;
  updatedAt: string;
}

export interface MemoryConflict {
  id: string;
  memoryId: string;
  memoryTitle: string;
  conflictingMemoryId: string;
  conflictingMemoryTitle: string;
  conflictKey: string;
  currentValue: string;
  conflictingValue: string;
  status: "unresolved" | "resolved" | "ignored";
  createdAt: string;
  resolvedAt: string | null;
}

export interface ApplyMemoryUpdateInput {
  memoryId: string | null;
  expectedCurrentVersionId: string | null;
  projectId: string | null;
  memorySpaceId: string | null;
  memoryType: string | null;
  authority: string | null;
  title: string | null;
  action: "add" | "merge" | "replace" | "supersede";
  sources: MergeSourceInput[];
}

export interface SecretWarning {
  kind: string;
  line: number;
  redactedExcerpt: string;
}

export interface ImportInput {
  sourcePath: string;
  projectId: string | null;
  memorySpaceId: string | null;
}

export interface ImportCandidate {
  title: string;
  content: string;
  memoryType: string;
  authority: string;
  status: string;
  contentHash: string;
  duplicate: boolean;
  secretWarnings: SecretWarning[];
}

export interface ImportPreview {
  sourceLabel: string;
  sourceFormat: "json" | "markdown" | "text";
  sourceHash: string;
  candidates: ImportCandidate[];
  importableCount: number;
  duplicateCount: number;
  secretWarningCount: number;
}

export interface ImportResult {
  importId: string;
  sourceLabel: string;
  importedCount: number;
  skippedCount: number;
}

export interface BackupSettings {
  enabled: boolean;
  intervalHours: number;
  directory: string;
  lastBackupAt: string | null;
  updatedAt: string;
}

export interface ScheduledBackupResult {
  created: boolean;
  path: string | null;
  reason: string;
  settings: BackupSettings;
}

export interface RestoreResult {
  restoredFrom: string;
  recoveryBackupPath: string;
  schemaVersion: number;
}

export interface DiagnosticReport {
  generatedAt: string;
  application: string;
  status: string;
  schemaVersion: number;
  sqliteVersion: string;
  databasePath: string;
  integrity: string;
  projectCount: number;
  conversationCount: number;
  messageCount: number;
  memoryCount: number;
  contextPackCount: number;
  unresolvedConflictCount: number;
  handoffCount: number;
  importCount: number;
  backupSettings: BackupSettings;
}

export interface SmartSettings {
  mode: "off" | "local" | "provider";
  providerName: string;
  providerEndpoint: string;
  providerModel: string;
  recommendationMode: "off" | "ask" | "automatic";
  embeddingModel: string;
  updatedAt: string;
}

export interface GeneratedArtifact {
  id: string;
  sourceType: "memory" | "conversation" | "conflict";
  sourceId: string;
  artifactType: "summary" | "conflict_explanation";
  content: string;
  modelMode: "local" | "provider";
  createdAt: string;
}

export interface SmartCandidate {
  id: string;
  projectId: string | null;
  sourceType: "memory" | "conversation";
  sourceId: string;
  memoryType: "decision" | "requirement" | "suggestion";
  title: string;
  content: string;
  confidence: number;
  status: "pending" | "accepted" | "dismissed";
  createdAt: string;
  reviewedAt: string | null;
}

export interface ConflictExplanation {
  artifact: GeneratedArtifact;
  conflictKey: string;
  sources: Array<{ memoryId: string; title: string; content: string }>;
}

export interface ContextRecommendation {
  mode: "off" | "ask" | "automatic";
  requiresConfirmation: boolean;
  message: string;
  results: SearchResult[];
}

export interface SmartBenchmarkReport {
  device: string;
  modelFamily: string;
  recommendedProfile: string;
  benchmarks: Array<{
    profile: string;
    dimensions: number;
    documents: number;
    elapsedMilliseconds: number;
    documentsPerSecond: number;
    estimatedBytesPerDocument: number;
  }>;
}

export interface SyncApplyResult {
  added: number;
  fastForwarded: number;
  unchanged: number;
  conflicts: number;
}

export interface SyncConflict {
  id: string;
  entityType: "memory";
  entityId: string;
  projectId: string | null;
  localValue: string;
  remoteValue: string;
  remoteDeviceId: string;
  status: "unresolved" | "keep_local" | "use_remote";
  createdAt: string;
  resolvedAt: string | null;
}

export interface SyncEnvelopeEntry {
  path: string;
  deviceId: string;
  createdAt: string;
  size: number;
}

export const core = {
  dashboard: () => invoke<DashboardSnapshot>("dashboard_snapshot"),
  health: () => invoke<HealthReport>("health_check"),
  search: (input: SearchInput) => invoke<SearchResponse>("search_context", { input }),
  askMemory: (input: {
    query: string;
    scope: "all" | "global" | "project";
    projectId: string | null;
    provider: string | null;
  }) => invoke<AskMemoryResponse>("ask_memory", { input }),
  detectSecrets: (text: string) => invoke<SecretWarning[]>("detect_secrets", { text }),
  previewImport: (input: ImportInput) => invoke<ImportPreview>("preview_import", { input }),
  importFile: (input: ImportInput) => invoke<ImportResult>("import_file", { input }),
  backupSettings: () => invoke<BackupSettings>("backup_settings"),
  updateBackupSettings: (input: { enabled: boolean; intervalHours: number; directory: string }) =>
    invoke<BackupSettings>("update_backup_settings", { input }),
  runScheduledBackup: (force: boolean) =>
    invoke<ScheduledBackupResult>("run_scheduled_backup", { force }),
  restoreBackup: (source: string) => invoke<RestoreResult>("restore_backup", { source }),
  diagnostics: () => invoke<DiagnosticReport>("diagnostics"),
  exportDiagnostics: (destination: string) => invoke<string>("export_diagnostics", { destination }),
  smartSettings: () => invoke<SmartSettings>("smart_settings"),
  updateSmartSettings: (input: {
    mode: SmartSettings["mode"];
    providerName: string;
    providerEndpoint: string;
    providerModel: string;
    recommendationMode: SmartSettings["recommendationMode"];
  }) => invoke<SmartSettings>("update_smart_settings", { input }),
  rebuildSemanticIndex: () => invoke<number>("rebuild_semantic_index"),
  generateSummary: (sourceType: "memory" | "conversation", sourceId: string) =>
    invoke<GeneratedArtifact>("generate_summary", {
      input: { sourceType, sourceId, maximumCharacters: 700 },
    }),
  generatedArtifacts: (sourceId: string | null = null) =>
    invoke<GeneratedArtifact[]>("generated_artifacts", { sourceId }),
  extractSmartCandidates: (input: {
    sourceType: "memory" | "conversation";
    sourceId: string;
    projectId: string | null;
  }) => invoke<SmartCandidate[]>("extract_smart_candidates", { input }),
  smartCandidates: (status: SmartCandidate["status"] | null = "pending") =>
    invoke<SmartCandidate[]>("smart_candidates", { status }),
  reviewSmartCandidate: (
    candidateId: string,
    action: "accept" | "dismiss",
    memorySpaceId: string | null = null,
  ) =>
    invoke<{ candidate: SmartCandidate; memory: Memory | null }>("review_smart_candidate", {
      input: { candidateId, action, memorySpaceId },
    }),
  explainConflict: (conflictId: string) =>
    invoke<ConflictExplanation>("explain_conflict", { conflictId }),
  recommendContext: (prompt: string, projectId: string | null, limit = 6) =>
    invoke<ContextRecommendation>("recommend_context", {
      input: { prompt, projectId, limit },
    }),
  benchmarkLocalEmbeddings: () => invoke<SmartBenchmarkReport>("benchmark_local_embeddings"),
  openSourceUrl: (url: string) => invoke<void>("open_source_url", { url }),
  createProject: (input: { name: string; description: string }) =>
    invoke<Project>("create_project", { input }),
  createMemorySpace: (input: {
    projectId: string | null;
    parentId: string | null;
    name: string;
    description: string;
    defaultScope: string;
  }) => invoke<MemorySpace>("create_memory_space", { input }),
  moveMemorySpace: (spaceId: string, parentId: string | null) =>
    invoke<void>("move_memory_space", { spaceId, parentId }),
  moveMemoryToSpace: (memoryId: string, spaceId: string | null) =>
    invoke<void>("move_memory_to_space", { memoryId, spaceId }),
  createMemory: (input: {
    projectId: string | null;
    memorySpaceId: string | null;
    memoryType: string;
    authority: string;
    status: string;
    title: string;
    content: string;
  }) => invoke<Memory>("create_memory", { input }),
  appendVersion: (input: {
    memoryId: string;
    content: string;
    changeType: string;
    nextStatus: string | null;
  }) => invoke("append_memory_version", { input }),
  capturedSources: () => invoke<CapturedSource[]>("captured_sources"),
  previewMemoryUpdate: (input: {
    memoryId: string | null;
    action: ApplyMemoryUpdateInput["action"];
    sources: MergeSourceInput[];
  }) => invoke<MemoryUpdatePreview>("preview_memory_update", { input }),
  applyMemoryUpdate: (input: ApplyMemoryUpdateInput) =>
    invoke<{ memory: Memory; version: MemoryVersion; conflicts: MemoryConflict[] }>(
      "apply_memory_update",
      { input },
    ),
  memoryHistory: (memoryId: string) => invoke<MemoryHistory>("memory_history", { memoryId }),
  restoreMemoryVersion: (memoryId: string, versionId: string) =>
    invoke<MemoryVersion>("restore_memory_version", { memoryId, versionId }),
  createMemoryBranch: (memoryId: string, name: string) =>
    invoke<MemoryBranch>("create_memory_branch", { input: { memoryId, name } }),
  appendBranchVersion: (
    branchId: string,
    content: string,
    changeType: "add" | "merge" | "replace" | "restore" = "replace",
  ) =>
    invoke<MemoryBranch>("append_branch_version", {
      input: { branchId, content, changeType },
    }),
  memoryBranches: (memoryId: string | null = null) =>
    invoke<MemoryBranch[]>("memory_branches", { memoryId }),
  finalizeMemoryBranch: (branchId: string, mode: "promote" | "merge" | "abandon") =>
    invoke<MemoryBranch>("finalize_memory_branch", { branchId, mode }),
  memoryConflicts: () => invoke<MemoryConflict[]>("memory_conflicts"),
  resolveMemoryConflict: (conflictId: string, resolution: "resolved" | "ignored") =>
    invoke<void>("resolve_memory_conflict", { conflictId, resolution }),
  createPack: (input: { projectId: string | null; name: string; description: string }) =>
    invoke<ContextPack>("create_context_pack", { input }),
  exportJson: (destination: string) => invoke<string>("export_json", { destination }),
  exportMarkdown: (destination: string) => invoke<string>("export_markdown", { destination }),
  backup: (destination: string) => invoke<string>("backup_database", { destination }),
  exportPortableWorkspace: (input: {
    projectIds: string[];
    includeGlobal: boolean;
    sourceDeviceId: string;
  }) => invoke<PortableWorkspace>("export_portable_workspace", { input }),
  applyPortableWorkspace: (workspace: PortableWorkspace) =>
    invoke<SyncApplyResult>("apply_portable_workspace", { input: { workspace } }),
  syncConflicts: () => invoke<SyncConflict[]>("sync_conflicts"),
  resolveSyncConflict: (conflictId: string, resolution: "keep_local" | "use_remote") =>
    invoke<SyncConflict>("resolve_sync_conflict", { input: { conflictId, resolution } }),
  writeSyncEnvelope: (directory: string, deviceId: string, envelope: string) =>
    invoke<string>("write_sync_envelope", { directory, deviceId, envelope }),
  listSyncEnvelopes: (directory: string) =>
    invoke<SyncEnvelopeEntry[]>("list_sync_envelopes", { directory }),
  readSyncEnvelope: (path: string) => invoke<string>("read_sync_envelope", { path }),
};

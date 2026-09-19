import { open, save } from "@tauri-apps/plugin-dialog";
import {
  decryptWorkspace,
  encryptWorkspace,
  parseEncryptedEnvelope,
  serializeEncryptedEnvelope,
} from "@tf0000/encrypted-sync";
import {
  cloneElement,
  type DragEvent,
  type FormEvent,
  type ReactElement,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  type AskMemoryResponse,
  type BackupSettings,
  type CapturedSource,
  type ConflictExplanation,
  type ContextRecommendation,
  core,
  type DashboardSnapshot,
  type DiagnosticReport,
  type GeneratedArtifact,
  type HealthReport,
  type ImportInput,
  type ImportPreview,
  type Memory,
  type MemoryBranch,
  type MemoryConflict,
  type MemoryHistory,
  type MemoryUpdatePreview,
  type MergeSourceInput,
  type SearchResponse,
  type SearchResult,
  type SmartBenchmarkReport,
  type SmartCandidate,
  type SmartSettings,
  type SyncConflict,
  type SyncEnvelopeEntry,
} from "./api";
import { AppearanceControls, Icon, viewFromHash, views, WorkspaceInsights } from "./workspace-ui";

const emptySnapshot: DashboardSnapshot = {
  projects: [],
  memorySpaces: [],
  memories: [],
  contextPacks: [],
  memorySpaceLinks: [],
};

const defaultBackupSettings: BackupSettings = {
  enabled: false,
  intervalHours: 24,
  directory: "",
  lastBackupAt: null,
  updatedAt: "",
};

const defaultSmartSettings: SmartSettings = {
  mode: "off",
  providerName: "",
  providerEndpoint: "",
  providerModel: "",
  recommendationMode: "off",
  embeddingModel: "tf0000-mini-embed-v1",
  updatedAt: "",
};

function messageFrom(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function readSyncPreference<T>(key: string, fallback: T): T {
  try {
    const value = localStorage.getItem(`tf0000.sync.${key}`);
    return value === null ? fallback : (JSON.parse(value) as T);
  } catch {
    return fallback;
  }
}

function initialSyncDeviceId(): string {
  const stored = readSyncPreference<unknown>("device", null);
  return typeof stored === "string" && /^[A-Za-z0-9][A-Za-z0-9_-]{1,63}$/.test(stored)
    ? stored
    : `device-${crypto.randomUUID().slice(0, 8)}`;
}

function initialSyncDirectory(): string {
  const stored = readSyncPreference<unknown>("directory", "");
  return typeof stored === "string" ? stored : "";
}

function initialSyncProjects(): Set<string> {
  const stored = readSyncPreference<unknown>("projects", []);
  return new Set(
    Array.isArray(stored)
      ? stored.filter((value): value is string => typeof value === "string")
      : [],
  );
}

function remoteConflictContent(value: string): string {
  try {
    const parsed = JSON.parse(value) as { memory?: { currentContent?: unknown } };
    return typeof parsed.memory?.currentContent === "string"
      ? parsed.memory.currentContent
      : "Remote content is unavailable";
  } catch {
    return "Remote content is unavailable";
  }
}

export function App() {
  const [activeView, setActiveView] = useState(() => viewFromHash(window.location.hash));
  const [loaded, setLoaded] = useState(false);
  const navigate = useCallback((id: string) => {
    setActiveView(viewFromHash(`#${id}`));
    window.location.hash = id;
  }, []);
  useEffect(() => {
    const onHashChange = () => {
      if (window.location.hash !== "#main-content") {
        setActiveView(viewFromHash(window.location.hash));
      }
    };
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);
  useEffect(() => {
    if (activeView) window.scrollTo({ top: 0, behavior: "instant" });
  }, [activeView]);
  const [snapshot, setSnapshot] = useState(emptySnapshot);
  const [health, setHealth] = useState<HealthReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [selectedProjectId, setSelectedProjectId] = useState<string>("");
  const [capturedSources, setCapturedSources] = useState<CapturedSource[]>([]);
  const [conflicts, setConflicts] = useState<MemoryConflict[]>([]);
  const [branches, setBranches] = useState<MemoryBranch[]>([]);
  const [selectedMemoryId, setSelectedMemoryId] = useState("");
  const [selectedSourceKeys, setSelectedSourceKeys] = useState<Set<string>>(new Set());
  const [manualNote, setManualNote] = useState("");
  const [manualNoteLabel, setManualNoteLabel] = useState("Manual note");
  const [updateAction, setUpdateAction] = useState<"add" | "merge" | "replace" | "supersede">(
    "merge",
  );
  const [newMemoryTitle, setNewMemoryTitle] = useState("");
  const [preview, setPreview] = useState<MemoryUpdatePreview | null>(null);
  const [history, setHistory] = useState<MemoryHistory | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchMode, setSearchMode] = useState<"search" | "ask">("search");
  const [searchScope, setSearchScope] = useState<"all" | "global" | "project">("all");
  const [searchProvider, setSearchProvider] = useState("");
  const [searchType, setSearchType] = useState("");
  const [searchStatus, setSearchStatus] = useState("");
  const [searchDateFrom, setSearchDateFrom] = useState("");
  const [searchDateTo, setSearchDateTo] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResponse | null>(null);
  const [memoryAnswer, setMemoryAnswer] = useState<AskMemoryResponse | null>(null);
  const [searchBusy, setSearchBusy] = useState(false);
  const [importPath, setImportPath] = useState("");
  const [importSpaceId, setImportSpaceId] = useState("");
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);
  const [backupSettings, setBackupSettings] = useState(defaultBackupSettings);
  const [diagnostics, setDiagnostics] = useState<DiagnosticReport | null>(null);
  const [smartSettings, setSmartSettings] = useState(defaultSmartSettings);
  const [smartSourceId, setSmartSourceId] = useState("");
  const [smartCandidates, setSmartCandidates] = useState<SmartCandidate[]>([]);
  const [generatedArtifacts, setGeneratedArtifacts] = useState<GeneratedArtifact[]>([]);
  const [recommendationPrompt, setRecommendationPrompt] = useState("");
  const [recommendation, setRecommendation] = useState<ContextRecommendation | null>(null);
  const [benchmark, setBenchmark] = useState<SmartBenchmarkReport | null>(null);
  const [conflictExplanation, setConflictExplanation] = useState<ConflictExplanation | null>(null);
  const [syncDirectory, setSyncDirectory] = useState(initialSyncDirectory);
  const [syncDeviceId, setSyncDeviceId] = useState(initialSyncDeviceId);
  const [syncProjectIds, setSyncProjectIds] = useState<Set<string>>(initialSyncProjects);
  const [syncIncludeGlobal, setSyncIncludeGlobal] = useState(
    () => readSyncPreference<unknown>("includeGlobal", false) === true,
  );
  const [syncPassphrase, setSyncPassphrase] = useState("");
  const [syncEntries, setSyncEntries] = useState<SyncEnvelopeEntry[]>([]);
  const [syncConflicts, setSyncConflicts] = useState<SyncConflict[]>([]);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    const [
      nextSnapshot,
      nextHealth,
      nextSources,
      nextConflicts,
      nextBranches,
      nextBackupSettings,
      nextDiagnostics,
      nextSmartSettings,
      nextSmartCandidates,
      nextArtifacts,
      nextSyncConflicts,
    ] = await Promise.all([
      core.dashboard(),
      core.health(),
      core.capturedSources(),
      core.memoryConflicts(),
      core.memoryBranches(),
      core.backupSettings(),
      core.diagnostics(),
      core.smartSettings(),
      core.smartCandidates("pending"),
      core.generatedArtifacts(),
      core.syncConflicts(),
    ]);
    setSnapshot(nextSnapshot);
    setHealth(nextHealth);
    setCapturedSources(nextSources);
    setConflicts(nextConflicts);
    setBranches(nextBranches);
    setBackupSettings(nextBackupSettings);
    setDiagnostics(nextDiagnostics);
    setSmartSettings(nextSmartSettings);
    setSmartCandidates(nextSmartCandidates);
    setGeneratedArtifacts(nextArtifacts);
    setSyncConflicts(nextSyncConflicts);
    setLoaded(true);
    if (!selectedProjectId && nextSnapshot.projects[0]) {
      setSelectedProjectId(nextSnapshot.projects[0].id);
    }
  }, [selectedProjectId]);

  useEffect(() => {
    try {
      localStorage.setItem("tf0000.sync.directory", JSON.stringify(syncDirectory));
      localStorage.setItem("tf0000.sync.device", JSON.stringify(syncDeviceId));
      localStorage.setItem("tf0000.sync.projects", JSON.stringify([...syncProjectIds]));
      localStorage.setItem("tf0000.sync.includeGlobal", JSON.stringify(syncIncludeGlobal));
    } catch {
      /* Sync still works for this session when preference storage is unavailable. */
    }
  }, [syncDeviceId, syncDirectory, syncIncludeGlobal, syncProjectIds]);

  useEffect(() => {
    refresh().catch((cause) => setError(messageFrom(cause)));
  }, [refresh]);

  useEffect(() => {
    const focusSearch = (event: globalThis.KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        navigate("search");
        requestAnimationFrame(() => searchInputRef.current?.focus());
      }
    };
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  }, [navigate]);

  useEffect(() => {
    const checkBackup = async () => {
      try {
        const result = await core.runScheduledBackup(false);
        if (result.created && result.path) {
          setBackupSettings(result.settings);
          setNotice(`Scheduled backup saved to ${result.path}`);
        }
      } catch (cause) {
        setError(messageFrom(cause));
      }
    };
    void checkBackup();
    const timer = window.setInterval(() => void checkBackup(), 5 * 60 * 1000);
    return () => window.clearInterval(timer);
  }, []);

  const run = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await action();
      await refresh();
      setNotice(success);
      return true;
    } catch (cause) {
      setError(messageFrom(cause));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const spacesForProject = useMemo(
    () => snapshot.memorySpaces.filter((space) => space.projectId === (selectedProjectId || null)),
    [selectedProjectId, snapshot.memorySpaces],
  );

  const availableProviders = useMemo(
    () => [...new Set(capturedSources.map((source) => source.provider))].sort(),
    [capturedSources],
  );

  const timeline = useMemo(
    () =>
      [
        ...snapshot.memories.map((memory) => ({
          id: `memory:${memory.id}`,
          at: memory.createdAt,
          kind: "Memory",
          label: memory.title,
        })),
        ...snapshot.contextPacks.map((pack) => ({
          id: `pack:${pack.id}`,
          at: pack.updatedAt,
          kind: "Context pack",
          label: pack.name,
        })),
        ...capturedSources
          .filter((source) => source.sourceType === "conversation")
          .map((source) => ({
            id: `source:${source.id}`,
            at: source.capturedAt,
            kind: source.provider,
            label: source.conversationTitle,
          })),
        ...conflicts.map((conflict) => ({
          id: `conflict:${conflict.id}`,
          at: conflict.createdAt,
          kind: "Conflict",
          label: conflict.conflictKey,
        })),
      ]
        .sort((left, right) => right.at.localeCompare(left.at))
        .slice(0, 8),
    [capturedSources, conflicts, snapshot.contextPacks, snapshot.memories],
  );

  const confirmSecretText = async (text: string, action: string): Promise<boolean> => {
    const warnings = await core.detectSecrets(text);
    if (warnings.length === 0) return true;
    const kinds = [...new Set(warnings.map((warning) => warning.kind))].join(", ");
    return window.confirm(
      `TF0000 detected ${warnings.length} possible secret${warnings.length === 1 ? "" : "s"} (${kinds}). ${action} anyway?`,
    );
  };

  const handleProject = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    void run(
      () =>
        core.createProject({
          name: String(form.get("name") ?? ""),
          description: String(form.get("description") ?? ""),
        }),
      "Project created",
    );
    event.currentTarget.reset();
  };

  const handleSpace = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    void run(
      () =>
        core.createMemorySpace({
          projectId: selectedProjectId || null,
          parentId: String(form.get("parentId") ?? "") || null,
          name: String(form.get("name") ?? ""),
          description: String(form.get("description") ?? ""),
          defaultScope: String(form.get("defaultScope") ?? "project"),
        }),
      "Memory space created",
    );
    event.currentTarget.reset();
  };

  const handleMemory = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const formElement = event.currentTarget;
    const form = new FormData(formElement);
    const content = String(form.get("content") ?? "");
    if (!(await confirmSecretText(content, "Save this memory"))) return;
    const saved = await run(
      () =>
        core.createMemory({
          projectId: selectedProjectId || null,
          memorySpaceId: String(form.get("memorySpaceId") ?? "") || null,
          memoryType: String(form.get("memoryType") ?? "decision"),
          authority: String(form.get("authority") ?? "user_confirmed"),
          status: "active",
          title: String(form.get("title") ?? ""),
          content,
        }),
      "Memory created with its first immutable version",
    );
    if (saved) formElement.reset();
  };

  const handlePack = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    void run(
      () =>
        core.createPack({
          projectId: selectedProjectId || null,
          name: String(form.get("name") ?? ""),
          description: String(form.get("description") ?? ""),
        }),
      "Context pack created",
    );
    event.currentTarget.reset();
  };

  const dropContext = (event: DragEvent<HTMLElement>, parentId: string | null) => {
    event.preventDefault();
    const memoryId = event.dataTransfer.getData("application/x-tf0000-memory");
    const spaceId = event.dataTransfer.getData("application/x-tf0000-space");
    if (memoryId) {
      void run(() => core.moveMemoryToSpace(memoryId, parentId), "Memory moved");
    } else if (spaceId && spaceId !== parentId) {
      void run(() => core.moveMemorySpace(spaceId, parentId), "Memory space moved");
    }
  };

  const toggleMergeSource = (key: string) => {
    setPreview(null);
    setSelectedSourceKeys((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const mergeSources = (): MergeSourceInput[] => {
    const captured = capturedSources
      .filter((source) => selectedSourceKeys.has(`${source.sourceType}:${source.id}`))
      .map((source) => ({
        sourceType: source.sourceType,
        sourceId: source.id,
        content: null,
        label: null,
      }));
    const memories = snapshot.memories
      .filter((memory) => selectedSourceKeys.has(`memory:${memory.id}`))
      .map((memory) => ({
        sourceType: "memory" as const,
        sourceId: memory.id,
        content: null,
        label: null,
      }));
    const manual: MergeSourceInput[] = manualNote.trim()
      ? [
          {
            sourceType: "manual",
            sourceId: null,
            content: manualNote,
            label: manualNoteLabel,
          },
        ]
      : [];
    return [...captured, ...memories, ...manual];
  };

  const buildUpdatePreview = async () => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const next = await core.previewMemoryUpdate({
        memoryId: selectedMemoryId || null,
        action: updateAction,
        sources: mergeSources(),
      });
      setPreview(next);
      setNotice("Preview is ready. Review the exact before/after content before applying.");
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const applyUpdate = async () => {
    if (!preview) return;
    if (!(await confirmSecretText(preview.diff.after, "Apply this memory update"))) return;
    const applied = await run(
      () =>
        core.applyMemoryUpdate({
          memoryId: selectedMemoryId || null,
          expectedCurrentVersionId: preview.currentVersionId,
          projectId: selectedProjectId || null,
          memorySpaceId: null,
          memoryType: "summary",
          authority: "user_confirmed",
          title: selectedMemoryId ? null : newMemoryTitle,
          action: updateAction,
          sources: mergeSources(),
        }),
      selectedMemoryId ? "Memory updated with a new immutable version" : "Merged memory created",
    );
    if (!applied) return;
    if (selectedMemoryId) setHistory(await core.memoryHistory(selectedMemoryId));
    setPreview(null);
    setSelectedSourceKeys(new Set());
    setManualNote("");
  };

  const openHistory = async (memory: Memory) => {
    setSelectedMemoryId(memory.id);
    setError(null);
    try {
      setHistory(await core.memoryHistory(memory.id));
      navigate("history");
    } catch (cause) {
      setError(messageFrom(cause));
    }
  };

  const handleSearch = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (searchScope === "project" && !selectedProjectId) {
      setError("Choose an active project before using the project search scope.");
      return;
    }
    setSearchBusy(true);
    setError(null);
    setNotice(null);
    try {
      if (searchMode === "ask") {
        setMemoryAnswer(
          await core.askMemory({
            query: searchQuery,
            scope: searchScope,
            projectId: searchScope === "project" ? selectedProjectId : null,
            provider: searchProvider || null,
          }),
        );
        setSearchResults(null);
      } else {
        setSearchResults(
          await core.search({
            query: searchQuery,
            scope: searchScope,
            projectId: searchScope === "project" ? selectedProjectId : null,
            provider: searchProvider || null,
            dateFrom: searchDateFrom || null,
            dateTo: searchDateTo || null,
            memoryType: searchType || null,
            status: searchStatus || null,
            limit: 50,
            offset: 0,
          }),
        );
        setMemoryAnswer(null);
      }
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setSearchBusy(false);
    }
  };

  const openSearchResult = async (result: SearchResult) => {
    if (result.resultType === "memory" && result.parentId) {
      const memory = snapshot.memories.find((candidate) => candidate.id === result.parentId);
      if (memory) await openHistory(memory);
      return;
    }
    if (result.sourceUrl) {
      try {
        await core.openSourceUrl(result.sourceUrl);
      } catch (cause) {
        setError(messageFrom(cause));
      }
      return;
    }
    if (result.resultType === "context_pack") {
      navigate("create");
    }
  };

  const restoreVersion = async (memoryId: string, versionId: string) => {
    const restored = await run(
      () => core.restoreMemoryVersion(memoryId, versionId),
      "The selected content was restored as a new immutable version",
    );
    if (restored) setHistory(await core.memoryHistory(memoryId));
  };

  const createBranch = async () => {
    if (!selectedMemoryId) return;
    const name = window.prompt("Branch name");
    if (!name?.trim()) return;
    await run(() => core.createMemoryBranch(selectedMemoryId, name), "Branch created");
  };

  const editBranch = async (branch: MemoryBranch) => {
    const content = window.prompt(`Edit branch: ${branch.name}`, branch.currentContent);
    if (!content?.trim() || content.trim() === branch.currentContent.trim()) return;
    await run(
      () => core.appendBranchVersion(branch.id, content, "replace"),
      "A new immutable branch version was appended",
    );
  };

  const finalizeBranch = async (branch: MemoryBranch, mode: "promote" | "merge" | "abandon") => {
    const finalized = await run(
      () => core.finalizeMemoryBranch(branch.id, mode),
      mode === "abandon" ? "Branch abandoned" : `Branch ${mode}d into memory history`,
    );
    if (finalized && history?.memory.id === branch.memoryId) {
      setHistory(await core.memoryHistory(branch.memoryId));
    }
  };

  const chooseAndExport = async (kind: "json" | "markdown" | "backup") => {
    const extension = kind === "json" ? "json" : kind === "markdown" ? "md" : "db";
    const destination = await save({
      title: kind === "backup" ? "Create database backup" : `Export ${kind}`,
      defaultPath: `tf0000-${new Date().toISOString().slice(0, 10)}.${extension}`,
      filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
    });
    if (!destination) return;
    const action =
      kind === "json"
        ? () => core.exportJson(destination)
        : kind === "markdown"
          ? () => core.exportMarkdown(destination)
          : () => core.backup(destination);
    await run(action, `Saved ${destination}`);
  };

  const importInput = (): ImportInput => ({
    sourcePath: importPath,
    projectId: selectedProjectId || null,
    memorySpaceId: importSpaceId || null,
  });

  const chooseImport = async () => {
    const selected = await open({
      title: "Choose context to import",
      multiple: false,
      directory: false,
      filters: [{ name: "Context files", extensions: ["json", "md", "markdown", "txt"] }],
    });
    if (typeof selected !== "string") return;
    setImportPath(selected);
    setImportPreview(null);
    setError(null);
    try {
      setImportPreview(
        await core.previewImport({
          sourcePath: selected,
          projectId: selectedProjectId || null,
          memorySpaceId: importSpaceId || null,
        }),
      );
      setNotice("Import preview is ready. Review duplicates and secret warnings before applying.");
    } catch (cause) {
      setError(messageFrom(cause));
    }
  };

  const refreshImportPreview = async () => {
    if (!importPath) return;
    setBusy(true);
    setError(null);
    try {
      setImportPreview(await core.previewImport(importInput()));
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const applyImport = async () => {
    if (!importPreview) return;
    if (
      importPreview.secretWarningCount > 0 &&
      !window.confirm(
        `This import contains ${importPreview.secretWarningCount} possible secret warning(s). Import the non-duplicate items anyway?`,
      )
    ) {
      return;
    }
    const applied = await run(
      () => core.importFile(importInput()),
      `Imported ${importPreview.importableCount} item(s); duplicates were skipped`,
    );
    if (applied) {
      setImportPreview(null);
      setImportPath("");
    }
  };

  const chooseBackupDirectory = async () => {
    const selected = await open({
      title: "Choose backup folder",
      directory: true,
      multiple: false,
    });
    if (typeof selected === "string") {
      setBackupSettings((current) => ({ ...current, directory: selected }));
    }
  };

  const saveBackupSchedule = async () => {
    const saved = await run(
      () =>
        core.updateBackupSettings({
          enabled: backupSettings.enabled,
          intervalHours: backupSettings.intervalHours,
          directory: backupSettings.directory,
        }),
      "Backup schedule saved",
    );
    if (saved) setBackupSettings(await core.backupSettings());
  };

  const runBackupNow = async () => {
    setBusy(true);
    setError(null);
    try {
      const result = await core.runScheduledBackup(true);
      setBackupSettings(result.settings);
      setNotice(result.path ? `Backup saved to ${result.path}` : result.reason);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const chooseAndRestore = async () => {
    const selected = await open({
      title: "Restore TF0000 backup",
      multiple: false,
      directory: false,
      filters: [{ name: "SQLite backup", extensions: ["db"] }],
    });
    if (typeof selected !== "string") return;
    if (
      !window.confirm(
        "Restore this backup? TF0000 will first create a recovery copy of the current database.",
      )
    ) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const result = await core.restoreBackup(selected);
      await refresh();
      setNotice(`Backup restored. Previous database preserved at ${result.recoveryBackupPath}`);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const runDiagnostics = async () => {
    setBusy(true);
    setError(null);
    try {
      const report = await core.diagnostics();
      setDiagnostics(report);
      setNotice(`Diagnostics complete: database integrity is ${report.integrity}`);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const chooseAndExportDiagnostics = async () => {
    const destination = await save({
      title: "Export privacy-safe diagnostics",
      defaultPath: `tf0000-diagnostics-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!destination) return;
    await run(() => core.exportDiagnostics(destination), `Diagnostics saved to ${destination}`);
  };

  const saveSmartSettings = async () => {
    const saved = await run(
      () =>
        core.updateSmartSettings({
          mode: smartSettings.mode,
          providerName: smartSettings.providerName,
          providerEndpoint: smartSettings.providerEndpoint,
          providerModel: smartSettings.providerModel,
          recommendationMode: smartSettings.recommendationMode,
        }),
      smartSettings.mode === "off"
        ? "Smart features turned off; standard search remains available"
        : "Smart settings saved and the local semantic index is ready",
    );
    if (saved) setSmartSettings(await core.smartSettings());
  };

  const generateSmartSummary = async () => {
    if (!smartSourceId) return;
    const generated = await run(
      () => core.generateSummary("memory", smartSourceId),
      "Optional summary generated without changing the source memory",
    );
    if (generated) setGeneratedArtifacts(await core.generatedArtifacts());
  };

  const extractSmartCandidates = async () => {
    if (!smartSourceId) return;
    const extracted = await run(
      () =>
        core.extractSmartCandidates({
          sourceType: "memory",
          sourceId: smartSourceId,
          projectId: selectedProjectId || null,
        }),
      "Candidate memories extracted for review",
    );
    if (extracted) setSmartCandidates(await core.smartCandidates("pending"));
  };

  const reviewCandidate = async (candidateId: string, action: "accept" | "dismiss") => {
    const reviewed = await run(
      () => core.reviewSmartCandidate(candidateId, action),
      action === "accept" ? "Candidate accepted as a draft AI suggestion" : "Candidate dismissed",
    );
    if (reviewed) setSmartCandidates(await core.smartCandidates("pending"));
  };

  const getRecommendations = async () => {
    if (!recommendationPrompt.trim()) return;
    setBusy(true);
    setError(null);
    try {
      setRecommendation(
        await core.recommendContext(recommendationPrompt, selectedProjectId || null, 6),
      );
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const runSmartBenchmark = async () => {
    setBusy(true);
    setError(null);
    try {
      const report = await core.benchmarkLocalEmbeddings();
      setBenchmark(report);
      setNotice(`CPU benchmark complete; ${report.recommendedProfile} is recommended`);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const chooseSyncDirectory = async () => {
    try {
      const selected = await open({
        title: "Choose encrypted sync folder",
        directory: true,
        multiple: false,
      });
      if (typeof selected === "string") {
        setSyncDirectory(selected);
        setSyncEntries(await core.listSyncEnvelopes(selected));
      }
    } catch (cause) {
      setError(messageFrom(cause));
    }
  };

  const refreshSyncEntries = async () => {
    if (!syncDirectory) return;
    try {
      setSyncEntries(await core.listSyncEnvelopes(syncDirectory));
    } catch (cause) {
      setError(messageFrom(cause));
    }
  };

  const publishEncryptedSnapshot = async () => {
    if (!syncDirectory || syncPassphrase.length < 12) return;
    setBusy(true);
    setError(null);
    try {
      const workspace = await core.exportPortableWorkspace({
        projectIds: [...syncProjectIds],
        includeGlobal: syncIncludeGlobal,
        sourceDeviceId: syncDeviceId.trim(),
      });
      const envelope = await encryptWorkspace(workspace, syncPassphrase);
      const path = await core.writeSyncEnvelope(
        syncDirectory,
        syncDeviceId.trim(),
        serializeEncryptedEnvelope(envelope),
      );
      await refreshSyncEntries();
      setNotice(`Encrypted snapshot published to ${path}`);
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const pullEncryptedSnapshots = async () => {
    if (!syncDirectory || syncPassphrase.length < 12) return;
    setBusy(true);
    setError(null);
    try {
      const entries = await core.listSyncEnvelopes(syncDirectory);
      const remoteEntries = entries.filter((entry) => entry.deviceId !== syncDeviceId.trim());
      const remoteWorkspaces = [];
      for (const entry of remoteEntries) {
        const envelope = parseEncryptedEnvelope(await core.readSyncEnvelope(entry.path));
        remoteWorkspaces.push(await decryptWorkspace(envelope, syncPassphrase));
      }
      let added = 0;
      let fastForwarded = 0;
      let mergeConflicts = 0;
      for (const workspace of remoteWorkspaces) {
        const result = await core.applyPortableWorkspace(workspace);
        added += result.added;
        fastForwarded += result.fastForwarded;
        mergeConflicts += result.conflicts;
      }
      setSyncEntries(entries);
      await refresh();
      setNotice(
        remoteEntries.length === 0
          ? "No snapshots from another device were found."
          : `Sync complete: ${added} added, ${fastForwarded} updated, ${mergeConflicts} awaiting review.`,
      );
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  const resolveOfflineConflict = async (
    conflictId: string,
    resolution: "keep_local" | "use_remote",
  ) => {
    await run(
      () => core.resolveSyncConflict(conflictId, resolution),
      resolution === "keep_local" ? "Kept this device’s version" : "Accepted the remote version",
    );
  };

  const explainConflict = async (conflictId: string) => {
    setBusy(true);
    setError(null);
    try {
      setConflictExplanation(await core.explainConflict(conflictId));
      navigate("smart");
    } catch (cause) {
      setError(messageFrom(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to main content
      </a>
      <aside className="sidebar">
        <div>
          <a className="brand" href="#dashboard">
            <span className="brand-mark" aria-hidden="true">
              tf
            </span>
            <h1>TF0000</h1>
          </a>
          <p className="sidebar-copy">Your context. Every AI.</p>
        </div>
        <nav aria-label="Workspace sections">
          <span className="nav-caption">Workspace</span>
          {views.map(([id, label, icon]) => (
            <a key={id} href={`#${id}`} aria-current={activeView === id ? "page" : undefined}>
              <Icon name={icon} />
              <span>{label}</span>
              {id === "conflicts" && conflicts.length > 0 && (
                <span className="nav-count">{conflicts.length}</span>
              )}
              {id === "sync" && syncConflicts.length > 0 && (
                <span className="nav-count">{syncConflicts.length}</span>
              )}
            </a>
          ))}
        </nav>
        <AppearanceControls />
        <div className="health-card">
          <span
            className={`status-dot ${health?.status === "healthy" ? "healthy" : ""}`}
            aria-hidden="true"
          />
          <div>
            <strong>
              {health?.status === "healthy"
                ? "Local storage healthy"
                : error
                  ? "Connection needs attention"
                  : "Connecting to local storage"}
            </strong>
            <small>{health ? "Your workspace lives on this device" : "Opening local store"}</small>
          </div>
        </div>
      </aside>

      <main id="main-content" tabIndex={-1}>
        <div className="workspace-toolbar">
          <span>
            Workspace <span aria-hidden="true">/</span>{" "}
            <strong>{views.find(([id]) => id === activeView)?.[1]}</strong>
          </span>
          <button
            type="button"
            className="secondary quick-search"
            onClick={() => {
              navigate("search");
              requestAnimationFrame(() => searchInputRef.current?.focus());
            }}
          >
            <Icon name="search" />
            Search context <kbd>Ctrl K</kbd>
          </button>
        </div>
        <header
          id="overview"
          className={`hero ${activeView !== "dashboard" ? "compact-hero" : ""}`}
        >
          <div>
            <div className="eyebrow">TF0000 · Your personal context workspace</div>
            <h2>
              {activeView === "dashboard" ? (
                <>
                  Good ideas deserve
                  <br />a place to stay.
                </>
              ) : (
                views.find(([id]) => id === activeView)?.[1]
              )}
            </h2>
            <p>Save, organize, and carry your context across every AI you use.</p>
            {activeView === "dashboard" && (
              <div className="hero-actions">
                <a className="button-link" href="#create">
                  <Icon name="plus" />
                  Create context
                </a>
                <a className="text-action" href="#import">
                  Import a conversation <Icon name="arrow" />
                </a>
              </div>
            )}
          </div>
          <label className="project-picker">
            Active project
            <select
              value={selectedProjectId}
              onChange={(event) => setSelectedProjectId(event.target.value)}
            >
              <option value="">Global context</option>
              {snapshot.projects.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>
          </label>
        </header>

        {error && (
          <div className="banner error" role="alert">
            {error}
          </div>
        )}
        {notice && (
          <div className="banner success" role="status" aria-live="polite">
            {notice}
          </div>
        )}

        <section
          className="stats"
          aria-label="Workspace-wide summary"
          hidden={activeView !== "dashboard"}
        >
          <Stat label="Projects" value={loaded ? snapshot.projects.length : null} />
          <Stat label="Memory spaces" value={loaded ? snapshot.memorySpaces.length : null} />
          <Stat label="Memories" value={loaded ? snapshot.memories.length : null} />
          <Stat label="Context packs" value={loaded ? snapshot.contextPacks.length : null} />
          <Stat label="Open conflicts" value={loaded ? conflicts.length : null} />
          <Stat label="Candidates to review" value={loaded ? smartCandidates.length : null} />
        </section>

        <section id="dashboard" hidden={activeView !== "dashboard"}>
          <WorkspaceInsights snapshot={snapshot} sources={capturedSources} loaded={loaded} />
          <div className="section-heading">
            <div>
              <span className="eyebrow">At a glance</span>
              <h2>Workspace dashboard</h2>
            </div>
            <span className="quiet">Projects, packs, activity, and conflicts in one view.</span>
          </div>
          <div className="dashboard-grid">
            <article className="dashboard-card">
              <h3>Projects</h3>
              {snapshot.projects.length === 0 ? (
                <p className="quiet">No projects yet.</p>
              ) : (
                <ul>
                  {snapshot.projects.map((project) => (
                    <li key={project.id}>
                      <button
                        type="button"
                        className="list-button"
                        aria-pressed={selectedProjectId === project.id}
                        onClick={() => setSelectedProjectId(project.id)}
                      >
                        <strong>{project.name}</strong>
                        <span className="dashboard-meta">
                          {
                            snapshot.memories.filter((memory) => memory.projectId === project.id)
                              .length
                          }{" "}
                          memories
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </article>
            <article className="dashboard-card">
              <h3>Context packs</h3>
              {snapshot.contextPacks.length === 0 ? (
                <p className="quiet">No context packs yet.</p>
              ) : (
                <ul>
                  {snapshot.contextPacks.map((pack) => (
                    <li key={pack.id}>
                      <strong>{pack.name}</strong>
                      <span className="dashboard-meta">Version {pack.currentVersion}</span>
                    </li>
                  ))}
                </ul>
              )}
            </article>
            <article className="dashboard-card timeline-card">
              <h3>Recent timeline</h3>
              {timeline.length === 0 ? (
                <p className="quiet">Activity will appear here.</p>
              ) : (
                <ol>
                  {timeline.map((event) => (
                    <li key={event.id}>
                      <span className="dashboard-meta">{event.kind}</span>
                      <strong>{event.label}</strong>
                      <time className="dashboard-meta" dateTime={event.at}>
                        {new Date(event.at).toLocaleString()}
                      </time>
                    </li>
                  ))}
                </ol>
              )}
            </article>
            <article className="dashboard-card">
              <h3>Conflict status</h3>
              <strong className="dashboard-number">{loaded ? conflicts.length : "—"}</strong>
              <p className="quiet">
                {!loaded
                  ? "Waiting for local data."
                  : conflicts.length === 0
                    ? "No unresolved conflicts."
                    : "Review unresolved conflicts before important handoffs."}
              </p>
              {conflicts.length > 0 && (
                <a className="dashboard-link" href="#conflicts">
                  Open conflict inbox
                </a>
              )}
              <p className="quiet">
                {loaded
                  ? `${smartCandidates.length} AI-generated candidates awaiting your review.`
                  : "Loading review queue…"}
              </p>
              {smartCandidates.length > 0 && (
                <a className="dashboard-link" href="#smart">
                  Review candidates
                </a>
              )}
            </article>
          </div>
        </section>

        <section id="import" hidden={activeView !== "import"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Reviewed and deduplicated</span>
              <h2>Import context</h2>
            </div>
            <span className="quiet">JSON, Markdown, text, and common provider exports.</span>
          </div>
          <div className="import-layout">
            <article className="import-controls">
              <Field label="Destination project">
                <select
                  value={selectedProjectId}
                  onChange={(event) => {
                    setSelectedProjectId(event.target.value);
                    setImportSpaceId("");
                    setImportPreview(null);
                  }}
                >
                  <option value="">Global context</option>
                  {snapshot.projects.map((project) => (
                    <option key={project.id} value={project.id}>
                      {project.name}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Destination memory space">
                <select
                  value={importSpaceId}
                  onChange={(event) => {
                    setImportSpaceId(event.target.value);
                    setImportPreview(null);
                  }}
                >
                  <option value="">Unfiled</option>
                  {spacesForProject.map((space) => (
                    <option key={space.id} value={space.id}>
                      {space.name}
                    </option>
                  ))}
                </select>
              </Field>
              <button type="button" onClick={() => void chooseImport()} disabled={busy}>
                Choose import file
              </button>
              {importPath && <code className="database-path">{importPath}</code>}
              {importPath && !importPreview && (
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void refreshImportPreview()}
                  disabled={busy}
                >
                  Rebuild preview
                </button>
              )}
            </article>
            <article className="import-preview" aria-live="polite">
              {importPreview ? (
                <>
                  <div className="section-heading compact-heading">
                    <div>
                      <h3>{importPreview.sourceLabel}</h3>
                      <span className="quiet">{importPreview.sourceFormat}</span>
                    </div>
                    <span>
                      {importPreview.importableCount} new · {importPreview.duplicateCount} duplicate
                    </span>
                  </div>
                  {importPreview.secretWarningCount > 0 && (
                    <div className="secret-warning" role="alert">
                      {importPreview.secretWarningCount} possible secret-like value(s) detected.
                      Review the affected items before importing.
                    </div>
                  )}
                  <div className="import-items">
                    {importPreview.candidates.map((candidate) => (
                      <div
                        className={`import-item ${candidate.duplicate ? "duplicate" : ""}`}
                        key={candidate.contentHash}
                      >
                        <strong>{candidate.title}</strong>
                        <span>
                          {candidate.duplicate ? "Duplicate — skipped" : candidate.memoryType}
                        </span>
                        <p>{candidate.content}</p>
                        {candidate.secretWarnings.map((warning) => (
                          <small key={`${warning.line}:${warning.kind}`}>
                            Warning on line {warning.line}: {warning.kind}
                          </small>
                        ))}
                      </div>
                    ))}
                  </div>
                  <button
                    type="button"
                    onClick={() => void applyImport()}
                    disabled={busy || importPreview.importableCount === 0}
                  >
                    Import {importPreview.importableCount} new item(s)
                  </button>
                </>
              ) : (
                <div className="empty-state">Choose a file to build a safe import preview.</div>
              )}
            </article>
          </div>
        </section>

        <section id="search" hidden={activeView !== "search"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">
                {smartSettings.mode === "off" ? "Local FTS5 retrieval" : "Hybrid local retrieval"}
              </span>
              <h2>Universal search</h2>
            </div>
            <span className="quiet">
              Press <kbd>Ctrl</kbd> + <kbd>K</kbd> from anywhere.
            </span>
          </div>
          <form className="search-panel" onSubmit={(event) => void handleSearch(event)}>
            <div className="search-bar">
              <input
                ref={searchInputRef}
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder="Search memories, chats, fragments and packs"
                required
                maxLength={500}
                aria-label="Universal search query"
              />
              <fieldset className="search-mode">
                <legend className="visually-hidden">Search mode</legend>
                <button
                  type="button"
                  className={searchMode === "search" ? "selected" : "secondary"}
                  aria-pressed={searchMode === "search"}
                  onClick={() => setSearchMode("search")}
                >
                  Search
                </button>
                <button
                  type="button"
                  className={searchMode === "ask" ? "selected" : "secondary"}
                  aria-pressed={searchMode === "ask"}
                  onClick={() => setSearchMode("ask")}
                >
                  Ask Memory
                </button>
              </fieldset>
              <button type="submit" disabled={searchBusy}>
                {searchBusy ? "Searching…" : searchMode === "ask" ? "Find evidence" : "Search"}
              </button>
            </div>
            <div className="search-filters">
              <Field label="Scope">
                <select
                  value={searchScope}
                  onChange={(event) =>
                    setSearchScope(event.target.value as "all" | "global" | "project")
                  }
                >
                  <option value="all">Everywhere</option>
                  <option value="global">Global only</option>
                  <option value="project">Active project</option>
                </select>
              </Field>
              <Field label="Provider">
                <select
                  value={searchProvider}
                  onChange={(event) => setSearchProvider(event.target.value)}
                >
                  <option value="">All providers</option>
                  {availableProviders.map((provider) => (
                    <option key={provider} value={provider}>
                      {provider}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Memory type">
                <select
                  value={searchType}
                  disabled={searchMode === "ask"}
                  onChange={(event) => setSearchType(event.target.value)}
                >
                  <option value="">All types</option>
                  {[
                    "decision",
                    "requirement",
                    "fact",
                    "preference",
                    "suggestion",
                    "idea",
                    "task",
                    "bug",
                    "question",
                    "reference",
                    "summary",
                  ].map((value) => (
                    <option key={value} value={value}>
                      {value.replace("_", " ")}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Status">
                <select
                  value={searchStatus}
                  disabled={searchMode === "ask"}
                  onChange={(event) => setSearchStatus(event.target.value)}
                >
                  <option value="">All statuses</option>
                  {["active", "draft", "superseded", "rejected", "archived"].map((value) => (
                    <option key={value} value={value}>
                      {value}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="From date">
                <input
                  type="date"
                  value={searchDateFrom}
                  disabled={searchMode === "ask"}
                  onChange={(event) => setSearchDateFrom(event.target.value)}
                />
              </Field>
              <Field label="To date">
                <input
                  type="date"
                  value={searchDateTo}
                  disabled={searchMode === "ask"}
                  onChange={(event) => setSearchDateTo(event.target.value)}
                />
              </Field>
            </div>
          </form>

          {searchResults && (
            <div className="search-output">
              <div className="result-summary">
                {searchResults.resultCount} result(s) for “{searchResults.query}”
              </div>
              {searchResults.results.length === 0 ? (
                <div className="empty-state">No matching local context.</div>
              ) : (
                <div className="search-result-list">
                  {searchResults.results.map((result) => (
                    <SearchResultCard
                      key={`${result.resultType}:${result.id}`}
                      result={result}
                      onOpen={openSearchResult}
                    />
                  ))}
                </div>
              )}
            </div>
          )}

          {memoryAnswer && (
            <div className={`ask-answer ${memoryAnswer.status}`}>
              <div className="answer-heading">
                <strong>
                  {memoryAnswer.status === "not_recorded" ? "Not recorded" : "Recorded evidence"}
                </strong>
                <span>{memoryAnswer.message}</span>
              </div>
              {memoryAnswer.currentDecisions.length > 0 && (
                <>
                  <h3>Current decisions</h3>
                  <div className="search-result-list">
                    {memoryAnswer.currentDecisions.map((result) => (
                      <SearchResultCard
                        key={`decision:${result.id}`}
                        result={result}
                        onOpen={openSearchResult}
                      />
                    ))}
                  </div>
                </>
              )}
              {memoryAnswer.evidence.length > 0 && (
                <>
                  <h3>Direct matches</h3>
                  <div className="search-result-list">
                    {memoryAnswer.evidence.map((result) => (
                      <SearchResultCard
                        key={`evidence:${result.resultType}:${result.id}`}
                        result={result}
                        onOpen={openSearchResult}
                      />
                    ))}
                  </div>
                </>
              )}
            </div>
          )}
        </section>

        <section id="smart" hidden={activeView !== "smart"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Optional and local first</span>
              <h2>Smart features</h2>
            </div>
            <span className={`smart-status ${smartSettings.mode}`}>
              {smartSettings.mode === "off" ? "Off" : `${smartSettings.mode} mode`}
            </span>
          </div>
          <div className="smart-grid">
            <article className="smart-panel">
              <h3>Control</h3>
              <p className="quiet">
                Turning this off immediately returns search to exact local FTS. Provider profiles
                store configuration only—never API keys.
              </p>
              <div className="field-row">
                <Field label="Smart features">
                  <select
                    value={smartSettings.mode}
                    onChange={(event) =>
                      setSmartSettings((current) => ({
                        ...current,
                        mode: event.target.value as SmartSettings["mode"],
                      }))
                    }
                  >
                    <option value="off">Off</option>
                    <option value="local">Local CPU</option>
                    <option value="provider">User-configured provider</option>
                  </select>
                </Field>
                <Field label="Context recommendations">
                  <select
                    value={smartSettings.recommendationMode}
                    onChange={(event) =>
                      setSmartSettings((current) => ({
                        ...current,
                        recommendationMode: event.target
                          .value as SmartSettings["recommendationMode"],
                      }))
                    }
                  >
                    <option value="off">Off</option>
                    <option value="ask">Ask me</option>
                    <option value="automatic">Automatic</option>
                  </select>
                </Field>
              </div>
              {smartSettings.mode === "provider" && (
                <div className="provider-settings">
                  <Field label="Provider name">
                    <input
                      value={smartSettings.providerName}
                      maxLength={80}
                      placeholder="My provider"
                      onChange={(event) =>
                        setSmartSettings((current) => ({
                          ...current,
                          providerName: event.target.value,
                        }))
                      }
                    />
                  </Field>
                  <Field label="HTTPS endpoint">
                    <input
                      value={smartSettings.providerEndpoint}
                      maxLength={2000}
                      placeholder="https://provider.example/v1"
                      onChange={(event) =>
                        setSmartSettings((current) => ({
                          ...current,
                          providerEndpoint: event.target.value,
                        }))
                      }
                    />
                  </Field>
                  <Field label="Model">
                    <input
                      value={smartSettings.providerModel}
                      maxLength={160}
                      placeholder="Model identifier"
                      onChange={(event) =>
                        setSmartSettings((current) => ({
                          ...current,
                          providerModel: event.target.value,
                        }))
                      }
                    />
                  </Field>
                </div>
              )}
              <div className="action-row compact-actions">
                <button type="button" onClick={() => void saveSmartSettings()} disabled={busy}>
                  Save smart settings
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={busy || smartSettings.mode === "off"}
                  onClick={() =>
                    void run(
                      () => core.rebuildSemanticIndex(),
                      "Semantic index rebuilt from canonical local content",
                    )
                  }
                >
                  Rebuild semantic index
                </button>
              </div>
              <small className="quiet">Embedding model: {smartSettings.embeddingModel}</small>
            </article>

            <article className="smart-panel">
              <h3>Summaries and candidate memories</h3>
              <p className="quiet">
                Summaries are separate artifacts. Extracted decisions, requirements, and suggestions
                stay pending until you accept them.
              </p>
              <Field label="Source memory">
                <select
                  value={smartSourceId}
                  onChange={(event) => setSmartSourceId(event.target.value)}
                >
                  <option value="">Choose a memory</option>
                  {snapshot.memories
                    .filter(
                      (memory) => !selectedProjectId || memory.projectId === selectedProjectId,
                    )
                    .map((memory) => (
                      <option key={memory.id} value={memory.id}>
                        {memory.title}
                      </option>
                    ))}
                </select>
              </Field>
              <div className="action-row compact-actions">
                <button
                  type="button"
                  disabled={busy || smartSettings.mode === "off" || !smartSourceId}
                  onClick={() => void generateSmartSummary()}
                >
                  Generate summary
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={busy || smartSettings.mode === "off" || !smartSourceId}
                  onClick={() => void extractSmartCandidates()}
                >
                  Extract candidates
                </button>
              </div>
              {generatedArtifacts.filter((artifact) => artifact.artifactType === "summary").length >
              0 ? (
                <div className="smart-list">
                  {generatedArtifacts
                    .filter((artifact) => artifact.artifactType === "summary")
                    .slice(0, 3)
                    .map((artifact) => (
                      <div className="smart-list-item" key={artifact.id}>
                        <strong className="smart-item-heading">Generated summary</strong>
                        <p className="smart-item-copy">{artifact.content}</p>
                        <small className="smart-item-meta">
                          {new Date(artifact.createdAt).toLocaleString()}
                        </small>
                      </div>
                    ))}
                </div>
              ) : (
                <div className="empty-state compact-empty">No generated summaries yet.</div>
              )}
            </article>

            <article className="smart-panel">
              <h3>Candidate review</h3>
              {smartCandidates.length === 0 ? (
                <div className="empty-state compact-empty">No pending candidates.</div>
              ) : (
                <div className="smart-list">
                  {smartCandidates.map((candidate) => (
                    <div className="smart-list-item" key={candidate.id}>
                      <span className="result-kind">{candidate.memoryType}</span>
                      <strong className="smart-item-heading">{candidate.title}</strong>
                      <p className="smart-item-copy">{candidate.content}</p>
                      <small className="smart-item-meta">
                        {Math.round(candidate.confidence * 100)}% heuristic confidence
                      </small>
                      <div className="action-row compact-actions">
                        <button
                          type="button"
                          onClick={() => void reviewCandidate(candidate.id, "accept")}
                          disabled={busy}
                        >
                          Accept as draft
                        </button>
                        <button
                          type="button"
                          className="secondary"
                          onClick={() => void reviewCandidate(candidate.id, "dismiss")}
                          disabled={busy}
                        >
                          Dismiss
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </article>

            <article className="smart-panel">
              <h3>Context recommendations</h3>
              <p className="quiet">
                Ask Me requires confirmation. Automatic marks relevant active memories as ready for
                attachment without altering them.
              </p>
              <Field label="What are you working on?">
                <textarea
                  rows={3}
                  maxLength={4000}
                  value={recommendationPrompt}
                  onChange={(event) => setRecommendationPrompt(event.target.value)}
                  placeholder="Describe the task or question"
                />
              </Field>
              <button
                type="button"
                disabled={busy || smartSettings.mode === "off" || !recommendationPrompt.trim()}
                onClick={() => void getRecommendations()}
              >
                Recommend context
              </button>
              {recommendation && (
                <div className="recommendation-output" aria-live="polite">
                  <strong>{recommendation.message}</strong>
                  <div className="smart-list">
                    {recommendation.results.map((result) => (
                      <SearchResultCard
                        key={`recommendation:${result.resultType}:${result.id}`}
                        result={result}
                        onOpen={openSearchResult}
                      />
                    ))}
                  </div>
                </div>
              )}
            </article>

            <article className="smart-panel wide-smart-panel">
              <div className="smart-panel-heading">
                <div>
                  <h3>Ordinary-laptop benchmark</h3>
                  <p className="quiet">Measures three compact embedding profiles on this CPU.</p>
                </div>
                <button
                  type="button"
                  className="secondary"
                  disabled={busy || smartSettings.mode === "off"}
                  onClick={() => void runSmartBenchmark()}
                >
                  Run CPU benchmark
                </button>
              </div>
              {benchmark && (
                <div className="benchmark-grid">
                  {benchmark.benchmarks.map((item) => (
                    <div key={item.profile}>
                      <strong>{item.profile}</strong>
                      <span>{item.dimensions} dimensions</span>
                      <span>{Math.round(item.documentsPerSecond).toLocaleString()} docs/sec</span>
                      <span>{item.estimatedBytesPerDocument} bytes/document</span>
                    </div>
                  ))}
                </div>
              )}
              {conflictExplanation && (
                <div className="conflict-explanation">
                  <strong>Source-grounded conflict explanation</strong>
                  <p>{conflictExplanation.artifact.content}</p>
                  <div className="conflict-values">
                    {conflictExplanation.sources.map((source) => (
                      <div className="conflict-source" key={source.memoryId}>
                        <strong className="conflict-source-title">{source.title}</strong>
                        <p>{source.content}</p>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </article>
          </div>
        </section>

        <section id="create" hidden={activeView !== "create"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Build your context</span>
              <h2>Create</h2>
            </div>
            <span className="quiet">Every write is validated and transactional.</span>
          </div>
          <div className="form-grid">
            <Card title="New project" description="A home for related chats and memory.">
              <form onSubmit={handleProject}>
                <Field label="Name">
                  <input name="name" required maxLength={160} />
                </Field>
                <Field label="Description">
                  <textarea name="description" rows={2} />
                </Field>
                <button type="submit" disabled={busy}>
                  Create project
                </button>
              </form>
            </Card>

            <Card
              title="New memory space"
              description="Organize memory without changing its source."
            >
              <form onSubmit={handleSpace}>
                <Field label="Name">
                  <input name="name" required maxLength={160} />
                </Field>
                <Field label="Parent">
                  <select name="parentId">
                    <option value="">No parent</option>
                    {spacesForProject.map((space) => (
                      <option key={space.id} value={space.id}>
                        {space.name}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Default scope">
                  <select name="defaultScope" defaultValue="project">
                    <option value="global">Global</option>
                    <option value="project">Project</option>
                    <option value="conversation">Conversation</option>
                    <option value="task">Task</option>
                  </select>
                </Field>
                <input name="description" type="hidden" value="" />
                <button type="submit" disabled={busy}>
                  Create space
                </button>
              </form>
            </Card>

            <Card title="New memory" description="The first version is preserved permanently." wide>
              <form onSubmit={handleMemory}>
                <div className="field-row">
                  <Field label="Title">
                    <input name="title" required maxLength={160} />
                  </Field>
                  <Field label="Space">
                    <select name="memorySpaceId">
                      <option value="">Unfiled</option>
                      {spacesForProject.map((space) => (
                        <option key={space.id} value={space.id}>
                          {space.name}
                        </option>
                      ))}
                    </select>
                  </Field>
                </div>
                <div className="field-row">
                  <Field label="Type">
                    <select name="memoryType" defaultValue="decision">
                      {[
                        "decision",
                        "requirement",
                        "fact",
                        "preference",
                        "suggestion",
                        "idea",
                        "task",
                        "bug",
                        "question",
                        "reference",
                        "summary",
                      ].map((value) => (
                        <option key={value} value={value}>
                          {value.replace("_", " ")}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <Field label="Authority">
                    <select name="authority" defaultValue="user_confirmed">
                      <option value="user_confirmed">User confirmed</option>
                      <option value="external_fact">External fact</option>
                      <option value="ai_suggestion">AI suggestion</option>
                      <option value="inferred">Inferred</option>
                    </select>
                  </Field>
                </div>
                <Field label="Content">
                  <textarea name="content" required rows={5} />
                </Field>
                <button type="submit" disabled={busy}>
                  Create memory
                </button>
              </form>
            </Card>

            <Card title="New context pack" description="A reusable ordered context container.">
              <form onSubmit={handlePack}>
                <Field label="Name">
                  <input name="name" required maxLength={160} />
                </Field>
                <Field label="Description">
                  <textarea name="description" rows={2} />
                </Field>
                <button type="submit" disabled={busy}>
                  Create pack
                </button>
              </form>
            </Card>
          </div>
        </section>

        <section id="spaces" hidden={activeView !== "memories"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Drag and organize</span>
              <h2>Memory spaces</h2>
            </div>
            <span className="quiet">Drop spaces to nest them or drop memories to file them.</span>
          </div>
          <button
            type="button"
            className="space-root-drop"
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => dropContext(event, null)}
          >
            Drop here for top-level / unfiled
          </button>
          <div className="space-tree">
            {spacesForProject.length === 0 ? (
              <div className="empty-state">
                Organize related memories in a space. <a href="#create">Create a memory space</a>
              </div>
            ) : (
              spacesForProject
                .filter((space) => space.parentId === null)
                .map((space) => (
                  <SpaceNode
                    key={space.id}
                    space={space}
                    spaces={spacesForProject}
                    memoryCount={
                      snapshot.memorySpaceLinks.filter((link) => link.memorySpaceId === space.id)
                        .length
                    }
                    links={snapshot.memorySpaceLinks}
                    onDrop={dropContext}
                  />
                ))
            )}
          </div>
        </section>

        <section id="memories" hidden={activeView !== "memories"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Current state</span>
              <h2>Memories</h2>
            </div>
            <span className="quiet">Updating appends a version; it never overwrites history.</span>
          </div>
          <div className="memory-list">
            {snapshot.memories.length === 0 ? (
              <div className="empty-state">
                <Icon name="library" />
                <p>A little context goes a long way.</p>
                <a className="button-link" href="#create">
                  Create your first memory <Icon name="plus" />
                </a>
              </div>
            ) : (
              snapshot.memories.map((memory) => (
                <article
                  className="memory-card"
                  key={memory.id}
                  draggable
                  onDragStart={(event) => {
                    event.dataTransfer.effectAllowed = "move";
                    event.dataTransfer.setData("application/x-tf0000-memory", memory.id);
                  }}
                >
                  <div className="memory-meta">
                    <span>{memory.memoryType}</span>
                    <span>{memory.authority.replace("_", " ")}</span>
                    <span className={memory.status === "active" ? "active-label" : ""}>
                      {memory.status}
                    </span>
                  </div>
                  <h3>{memory.title}</h3>
                  <p>{memory.currentContent}</p>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void openHistory(memory)}
                    disabled={busy}
                  >
                    History & branches
                  </button>
                </article>
              ))
            )}
          </div>
        </section>

        <section id="merge" hidden={activeView !== "merge"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Review before writing</span>
              <h2>Merge and update</h2>
            </div>
            <span className="quiet">
              Duplicates are removed by source identity and content hash.
            </span>
          </div>
          <div className="merge-layout">
            <article className="merge-builder">
              <div className="field-row">
                <Field label="Target memory">
                  <select
                    value={selectedMemoryId}
                    onChange={(event) => {
                      setSelectedMemoryId(event.target.value);
                      setPreview(null);
                    }}
                  >
                    <option value="">Create a new merged memory</option>
                    {snapshot.memories.map((memory) => (
                      <option key={memory.id} value={memory.id}>
                        {memory.title}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Update action">
                  <select
                    value={updateAction}
                    onChange={(event) => {
                      setUpdateAction(
                        event.target.value as "add" | "merge" | "replace" | "supersede",
                      );
                      setPreview(null);
                    }}
                  >
                    <option value="add">Add</option>
                    <option value="merge">Merge</option>
                    <option value="replace">Replace</option>
                    <option value="supersede">Supersede</option>
                  </select>
                </Field>
              </div>
              {!selectedMemoryId && (
                <Field label="New memory name">
                  <input
                    value={newMemoryTitle}
                    maxLength={160}
                    onChange={(event) => {
                      setNewMemoryTitle(event.target.value);
                      setPreview(null);
                    }}
                    placeholder="Cross-provider architecture context"
                  />
                </Field>
              )}

              <h3>Captured chats, messages and fragments</h3>
              <p className="quiet">
                Select a whole chat or several messages to form an exact message range.
              </p>
              <div className="source-picker">
                {capturedSources.length === 0 ? (
                  <div className="empty-state">
                    Capture a conversation from the browser extension.
                  </div>
                ) : (
                  capturedSources.map((source) => {
                    const key = `${source.sourceType}:${source.id}`;
                    return (
                      <label className="source-option" key={key}>
                        <input
                          className="source-checkbox"
                          type="checkbox"
                          checked={selectedSourceKeys.has(key)}
                          onChange={() => toggleMergeSource(key)}
                        />
                        <span className="source-option-content">
                          <strong>{source.conversationTitle}</strong>
                          <small>
                            {source.provider} · {source.sourceType} · {source.role}
                          </small>
                          <span className="source-excerpt">{source.content.slice(0, 180)}</span>
                        </span>
                      </label>
                    );
                  })
                )}
              </div>

              <h3>Existing memories</h3>
              <div className="source-picker compact">
                {snapshot.memories
                  .filter((memory) => memory.id !== selectedMemoryId)
                  .map((memory) => {
                    const key = `memory:${memory.id}`;
                    return (
                      <label className="source-option" key={key}>
                        <input
                          className="source-checkbox"
                          type="checkbox"
                          checked={selectedSourceKeys.has(key)}
                          onChange={() => toggleMergeSource(key)}
                        />
                        <span className="source-option-content">
                          <strong>{memory.title}</strong>
                          <small>
                            {memory.memoryType} · {memory.status}
                          </small>
                        </span>
                      </label>
                    );
                  })}
              </div>

              <div className="field-row">
                <Field label="Manual source label">
                  <input
                    value={manualNoteLabel}
                    maxLength={160}
                    onChange={(event) => {
                      setManualNoteLabel(event.target.value);
                      setPreview(null);
                    }}
                  />
                </Field>
                <Field label="Manual note">
                  <textarea
                    value={manualNote}
                    rows={3}
                    onChange={(event) => {
                      setManualNote(event.target.value);
                      setPreview(null);
                    }}
                  />
                </Field>
              </div>
              <button type="button" onClick={() => void buildUpdatePreview()} disabled={busy}>
                Build exact diff preview
              </button>
            </article>

            <article className="preview-panel">
              <h3>Pending change</h3>
              {!preview ? (
                <div className="empty-state">Select sources and build a preview.</div>
              ) : (
                <>
                  <div className="preview-stats">
                    <span>{preview.uniqueSourceCount} unique sources</span>
                    <span>{preview.duplicateCount} duplicates skipped</span>
                    <span className={preview.conflicts.length ? "warning-label" : "active-label"}>
                      {preview.conflicts.length} possible conflicts
                    </span>
                  </div>
                  <div className="diff-grid">
                    <div>
                      <strong>Before · −{preview.diff.removedLines} lines</strong>
                      <pre>{preview.diff.before || "New memory — no previous content"}</pre>
                    </div>
                    <div>
                      <strong>After · +{preview.diff.addedLines} lines</strong>
                      <pre>{preview.diff.after}</pre>
                    </div>
                  </div>
                  <div className="provenance-list">
                    {preview.sources.map((source) => (
                      <div
                        className={
                          source.duplicateOf ? "provenance-item duplicate" : "provenance-item"
                        }
                        key={`${source.sourceType}:${source.sourceId}`}
                      >
                        <strong>{source.label}</strong>
                        <span>
                          {source.sourceType} · {source.sourceHash}
                        </span>
                        {source.duplicateOf && <small>Duplicate of {source.duplicateOf}</small>}
                      </div>
                    ))}
                  </div>
                  <button
                    type="button"
                    onClick={() => void applyUpdate()}
                    disabled={busy || (!selectedMemoryId && !newMemoryTitle.trim())}
                  >
                    Apply as immutable version
                  </button>
                </>
              )}
            </article>
          </div>
        </section>

        <section id="history" hidden={activeView !== "history"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Nothing is overwritten</span>
              <h2>Version history and branches</h2>
            </div>
            {selectedMemoryId && (
              <button type="button" className="secondary" onClick={() => void createBranch()}>
                Create branch
              </button>
            )}
          </div>
          {!history ? (
            <div className="empty-state">Choose “History & branches” on a memory.</div>
          ) : (
            <div className="history-layout">
              <div className="timeline">
                <h3>{history.memory.title}</h3>
                {[...history.versions].reverse().map((version) => {
                  const versionSources = history.sources.filter(
                    (source) => source.memoryVersionId === version.id,
                  );
                  return (
                    <article className="version-card" key={version.id}>
                      <div className="version-heading">
                        <strong>{version.changeType}</strong>
                        <time>{new Date(version.createdAt).toLocaleString()}</time>
                      </div>
                      <p>{version.content}</p>
                      {versionSources.length > 0 && (
                        <div className="version-sources">
                          {versionSources.map((source) => (
                            <span key={`${version.id}:${source.sourceType}:${source.sourceId}`}>
                              {source.sourceLabel}
                            </span>
                          ))}
                        </div>
                      )}
                      {version.id !== history.memory.currentVersionId && (
                        <button
                          type="button"
                          className="secondary"
                          onClick={() => void restoreVersion(history.memory.id, version.id)}
                          disabled={busy}
                        >
                          Restore as new version
                        </button>
                      )}
                    </article>
                  );
                })}
              </div>
              <div className="branch-list">
                <h3>Alternative branches</h3>
                {branches.filter((branch) => branch.memoryId === history.memory.id).length === 0 ? (
                  <div className="empty-state">Create a branch to try an alternative safely.</div>
                ) : (
                  branches
                    .filter((branch) => branch.memoryId === history.memory.id)
                    .map((branch) => (
                      <article className="branch-card" key={branch.id}>
                        <div className="version-heading">
                          <strong>{branch.name}</strong>
                          <span>{branch.status}</span>
                        </div>
                        <p>{branch.currentContent}</p>
                        {branch.status === "active" && (
                          <div className="action-row compact-actions">
                            <button
                              type="button"
                              className="secondary"
                              onClick={() => void editBranch(branch)}
                            >
                              Edit
                            </button>
                            <button
                              type="button"
                              onClick={() => void finalizeBranch(branch, "promote")}
                            >
                              Promote
                            </button>
                            <button
                              type="button"
                              className="secondary"
                              onClick={() => void finalizeBranch(branch, "merge")}
                            >
                              Merge
                            </button>
                            <button
                              type="button"
                              className="danger-button"
                              onClick={() => void finalizeBranch(branch, "abandon")}
                            >
                              Abandon
                            </button>
                          </div>
                        )}
                      </article>
                    ))
                )}
              </div>
            </div>
          )}
        </section>

        <section id="conflicts" hidden={activeView !== "conflicts"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Deterministic review queue</span>
              <h2>Unresolved conflicts</h2>
            </div>
            <span className="quiet">
              Same-key active decisions are flagged; TF0000 never guesses.
            </span>
          </div>
          <div className="conflict-list">
            {conflicts.length === 0 ? (
              <div className="empty-state">No unresolved conflicts.</div>
            ) : (
              conflicts.map((conflict) => (
                <article className="conflict-card" key={conflict.id}>
                  <strong>{conflict.conflictKey}</strong>
                  <span>
                    {conflict.memoryTitle} ↔ {conflict.conflictingMemoryTitle}
                  </span>
                  <div className="conflict-values">
                    <p>{conflict.currentValue}</p>
                    <p>{conflict.conflictingValue}</p>
                  </div>
                  <div className="action-row compact-actions">
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => void explainConflict(conflict.id)}
                      disabled={busy || smartSettings.mode === "off"}
                    >
                      Explain with sources
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        void run(
                          () => core.resolveMemoryConflict(conflict.id, "resolved"),
                          "Conflict marked resolved",
                        )
                      }
                    >
                      Mark resolved
                    </button>
                    <button
                      type="button"
                      className="secondary"
                      onClick={() =>
                        void run(
                          () => core.resolveMemoryConflict(conflict.id, "ignored"),
                          "Conflict ignored",
                        )
                      }
                    >
                      Ignore
                    </button>
                  </div>
                </article>
              ))
            )}
          </div>
        </section>

        <section id="sync" hidden={activeView !== "sync"}>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Optional · end-to-end encrypted</span>
              <h2>Device sync</h2>
            </div>
            <span className="quiet">
              Local-only remains the default. Your passphrase is never stored.
            </span>
          </div>
          <div className="sync-layout">
            <article className="sync-card">
              <h3>Encrypted sync folder</h3>
              <p className="quiet">
                Choose any local or provider-synchronized folder. TF0000 writes only authenticated
                AES-256-GCM ciphertext into it.
              </p>
              <div className="field-row">
                <Field label="This device">
                  <input
                    value={syncDeviceId}
                    minLength={2}
                    maxLength={64}
                    pattern="[A-Za-z0-9][A-Za-z0-9_-]{1,63}"
                    onChange={(event) => setSyncDeviceId(event.target.value)}
                    placeholder="work-laptop"
                  />
                </Field>
                <Field label="Sync passphrase">
                  <input
                    type="password"
                    autoComplete="new-password"
                    minLength={12}
                    maxLength={1024}
                    value={syncPassphrase}
                    onChange={(event) => setSyncPassphrase(event.target.value)}
                    placeholder="At least 12 characters"
                    aria-describedby="sync-passphrase-help"
                  />
                </Field>
              </div>
              <small id="sync-passphrase-help" className="quiet">
                Use the same passphrase on each device. It cannot be recovered by TF0000.
              </small>
              <Field label="Sync folder">
                <div className="path-picker">
                  <input value={syncDirectory} readOnly placeholder="No folder selected" />
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void chooseSyncDirectory()}
                  >
                    Choose
                  </button>
                </div>
              </Field>
              <fieldset className="sync-selection">
                <legend>Projects to include</legend>
                {snapshot.projects.length === 0 ? (
                  <p className="quiet">Create a project before enabling selective project sync.</p>
                ) : (
                  snapshot.projects.map((project) => (
                    <label key={project.id}>
                      <input
                        type="checkbox"
                        checked={syncProjectIds.has(project.id)}
                        onChange={() =>
                          setSyncProjectIds((current) => {
                            const next = new Set(current);
                            next.has(project.id) ? next.delete(project.id) : next.add(project.id);
                            return next;
                          })
                        }
                      />
                      <span>{project.name}</span>
                    </label>
                  ))
                )}
                <label>
                  <input
                    type="checkbox"
                    checked={syncIncludeGlobal}
                    onChange={(event) => setSyncIncludeGlobal(event.target.checked)}
                  />
                  <span>Global context</span>
                </label>
              </fieldset>
              <div className="action-row">
                <button
                  type="button"
                  disabled={
                    busy ||
                    !syncDirectory ||
                    syncDeviceId.trim().length < 2 ||
                    syncPassphrase.length < 12 ||
                    (syncProjectIds.size === 0 && !syncIncludeGlobal)
                  }
                  onClick={() => void publishEncryptedSnapshot()}
                >
                  Publish encrypted snapshot
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={busy || !syncDirectory || syncPassphrase.length < 12}
                  onClick={() => void pullEncryptedSnapshots()}
                >
                  Pull other devices
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={busy || !syncDirectory}
                  onClick={() => void refreshSyncEntries()}
                >
                  Refresh folder
                </button>
              </div>
            </article>
            <article className="sync-card">
              <h3>Available device snapshots</h3>
              {syncEntries.length === 0 ? (
                <div className="empty-state">No encrypted snapshots found in this folder.</div>
              ) : (
                <ul className="sync-snapshot-list">
                  {syncEntries.map((entry) => (
                    <li key={entry.path}>
                      <div>
                        <strong>{entry.deviceId}</strong>
                        <span>
                          {entry.createdAt
                            ? new Date(entry.createdAt).toLocaleString()
                            : "Unknown date"}
                        </span>
                      </div>
                      <span>{Math.max(1, Math.ceil(entry.size / 1024)).toLocaleString()} KB</span>
                    </li>
                  ))}
                </ul>
              )}
            </article>
          </div>
          <div className="section-heading">
            <div>
              <span className="eyebrow">Never silently overwritten</span>
              <h2>Offline conflict review</h2>
            </div>
            <span className="quiet">{syncConflicts.length} concurrent edits need a decision.</span>
          </div>
          <div className="conflict-list">
            {syncConflicts.length === 0 ? (
              <div className="empty-state">No unresolved device-sync conflicts.</div>
            ) : (
              syncConflicts.map((conflict) => (
                <article className="conflict-card" key={conflict.id}>
                  <span>From {conflict.remoteDeviceId}</span>
                  <div className="conflict-values">
                    <div>
                      <strong>This device</strong>
                      <p>{conflict.localValue}</p>
                    </div>
                    <div>
                      <strong>Remote device</strong>
                      <p>{remoteConflictContent(conflict.remoteValue)}</p>
                    </div>
                  </div>
                  <div className="action-row">
                    <button
                      type="button"
                      className="secondary"
                      disabled={busy}
                      onClick={() => void resolveOfflineConflict(conflict.id, "keep_local")}
                    >
                      Keep this device
                    </button>
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void resolveOfflineConflict(conflict.id, "use_remote")}
                    >
                      Use remote version
                    </button>
                  </div>
                </article>
              ))
            )}
          </div>
        </section>

        <section id="safety" className="safety-panel" hidden={activeView !== "safety"}>
          <div>
            <span className="eyebrow">Recovery and diagnostics</span>
            <h2>Backup, restore, and health</h2>
            <p>
              Scheduled backups run while TF0000 is open. Every restore first preserves the current
              database as a recovery backup.
            </p>
          </div>
          <div className="backup-settings">
            <label className="check-field">
              <input
                type="checkbox"
                checked={backupSettings.enabled}
                onChange={(event) =>
                  setBackupSettings((current) => ({
                    ...current,
                    enabled: event.target.checked,
                  }))
                }
              />
              Enable scheduled local backups
            </label>
            <Field label="Interval in hours">
              <input
                type="number"
                min={1}
                max={8760}
                value={backupSettings.intervalHours}
                onChange={(event) =>
                  setBackupSettings((current) => ({
                    ...current,
                    intervalHours: Math.min(8760, Math.max(1, Number(event.target.value) || 1)),
                  }))
                }
              />
            </Field>
            <Field label="Backup folder">
              <div className="path-picker">
                <input value={backupSettings.directory} readOnly placeholder="Choose a folder" />
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void chooseBackupDirectory()}
                >
                  Choose
                </button>
              </div>
            </Field>
            <button type="button" onClick={() => void saveBackupSchedule()} disabled={busy}>
              Save schedule
            </button>
            <small className="quiet">
              Last scheduled backup:{" "}
              {backupSettings.lastBackupAt
                ? new Date(backupSettings.lastBackupAt).toLocaleString()
                : "Not created yet"}
            </small>
          </div>
          <div className="action-row">
            <button
              type="button"
              className="secondary"
              onClick={() => void chooseAndExport("json")}
              disabled={busy}
            >
              Export JSON
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => void chooseAndExport("markdown")}
              disabled={busy}
            >
              Export Markdown
            </button>
            <button type="button" onClick={() => void chooseAndExport("backup")} disabled={busy}>
              Create backup
            </button>
            <button
              type="button"
              onClick={() => void runBackupNow()}
              disabled={busy || !backupSettings.directory}
            >
              Run scheduled backup now
            </button>
            <button
              type="button"
              className="danger-button"
              onClick={() => void chooseAndRestore()}
              disabled={busy}
            >
              Restore backup…
            </button>
          </div>
          <div className="diagnostic-panel">
            <div>
              <h3>Database diagnostics</h3>
              <p className="quiet">
                The diagnostic export contains health and row counts, never memory or chat text.
              </p>
            </div>
            {diagnostics && (
              <dl>
                <div>
                  <dt>Integrity</dt>
                  <dd>{diagnostics.integrity}</dd>
                </div>
                <div>
                  <dt>Schema</dt>
                  <dd>v{diagnostics.schemaVersion}</dd>
                </div>
                <div>
                  <dt>Messages</dt>
                  <dd>{diagnostics.messageCount}</dd>
                </div>
                <div>
                  <dt>Imports</dt>
                  <dd>{diagnostics.importCount}</dd>
                </div>
                <div>
                  <dt>Handoffs</dt>
                  <dd>{diagnostics.handoffCount}</dd>
                </div>
              </dl>
            )}
            <div className="action-row compact-actions">
              <button type="button" onClick={() => void runDiagnostics()} disabled={busy}>
                Run diagnostics
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => void chooseAndExportDiagnostics()}
                disabled={busy}
              >
                Export diagnostics
              </button>
            </div>
          </div>
          {health && <code className="database-path">{health.databasePath}</code>}
        </section>
      </main>
    </div>
  );
}

function SpaceNode({
  space,
  spaces,
  memoryCount,
  links,
  onDrop,
}: {
  space: DashboardSnapshot["memorySpaces"][number];
  spaces: DashboardSnapshot["memorySpaces"];
  memoryCount: number;
  links: DashboardSnapshot["memorySpaceLinks"];
  onDrop: (event: DragEvent<HTMLElement>, parentId: string | null) => void;
}) {
  const children = spaces.filter((candidate) => candidate.parentId === space.id);
  return (
    <div className="space-branch">
      <button
        type="button"
        className="space-node"
        draggable
        onDragStart={(event) => {
          event.stopPropagation();
          event.dataTransfer.effectAllowed = "move";
          event.dataTransfer.setData("application/x-tf0000-space", space.id);
        }}
        onDragOver={(event) => event.preventDefault()}
        onDrop={(event) => {
          event.stopPropagation();
          onDrop(event, space.id);
        }}
      >
        <strong>{space.name}</strong>
        <span>{memoryCount} direct memories</span>
      </button>
      {children.length > 0 && (
        <div className="space-children">
          {children.map((child) => (
            <SpaceNode
              key={child.id}
              space={child}
              spaces={spaces}
              memoryCount={links.filter((link) => link.memorySpaceId === child.id).length}
              links={links}
              onDrop={onDrop}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function SearchResultCard({
  result,
  onOpen,
}: {
  result: SearchResult;
  onOpen: (result: SearchResult) => Promise<void>;
}) {
  const canOpen =
    result.resultType === "memory" ||
    result.resultType === "context_pack" ||
    Boolean(result.sourceUrl);
  return (
    <article className="search-result-card">
      <div className="search-result-heading">
        <div>
          <span className="result-kind">{result.resultType.replace("_", " ")}</span>
          <h3>{result.title}</h3>
        </div>
        <div className="result-badges">
          {result.provider && <span>{result.provider}</span>}
          {result.memoryType && <span>{result.memoryType}</span>}
          {result.authority && <span>{result.authority.replace("_", " ")}</span>}
          {result.status && <span>{result.status}</span>}
          {result.resultType === "memory" && (
            <span>{result.isCurrent ? "current" : "historical version"}</span>
          )}
        </div>
      </div>
      <p>{result.excerpt}</p>
      <div className="search-result-footer">
        <time>{new Date(result.createdAt).toLocaleString()}</time>
        {canOpen && (
          <button type="button" className="secondary" onClick={() => void onOpen(result)}>
            {result.sourceUrl
              ? "Open original source"
              : result.resultType === "memory"
                ? "Open history"
                : "Open pack area"}
          </button>
        )}
      </div>
    </article>
  );
}

function Stat({ label, value }: { label: string; value: number | null }) {
  return (
    <div className="stat">
      <strong>{value ?? "—"}</strong>
      <span>{label}</span>
    </div>
  );
}

function Card({
  title,
  description,
  wide = false,
  children,
}: {
  title: string;
  description: string;
  wide?: boolean;
  children: ReactNode;
}) {
  return (
    <article className={`form-card ${wide ? "wide" : ""}`}>
      <h3>{title}</h3>
      <p>{description}</p>
      {children}
    </article>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: ReactElement<{ "aria-label"?: string }>;
}) {
  return (
    <div className="field">
      <span>{label}</span>
      {cloneElement(children, { "aria-label": label })}
    </div>
  );
}

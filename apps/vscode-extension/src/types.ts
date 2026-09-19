export interface Project {
  id: string;
  name: string;
  description: string;
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

export interface DashboardSnapshot {
  projects: Project[];
  memories: Memory[];
}

export interface WorkspaceMapping {
  workspaceUri: string;
  repositoryRoot: string;
  projectId: string;
  projectName: string;
  createdAt: string;
  updatedAt: string;
}

export interface CodeReference {
  id: string;
  projectId: string;
  workspaceUri: string;
  repositoryRoot: string;
  relativePath: string;
  language: string;
  startLine: number | null;
  endLine: number | null;
  content: string;
  contentHash: string;
  createdAt: string;
}

export interface SearchResult {
  resultType: string;
  id: string;
  parentId: string | null;
  projectId: string | null;
  title: string;
  excerpt: string;
  memoryType: string | null;
  authority: string | null;
  status: string | null;
  score: number;
}

export interface SearchResponse {
  query: string;
  resultCount: number;
  results: SearchResult[];
}

export interface ProjectContext {
  projectId: string;
  projectName: string;
  text: string;
  memoryCount: number;
  codeReferenceCount: number;
  characterCount: number;
  estimatedTokens: number;
}

export interface AgentWriteCandidate {
  id: string;
  projectId: string;
  requestedBy: string;
  memoryType: string;
  title: string;
  content: string;
  status: "pending" | "accepted" | "rejected";
  createdAt: string;
  reviewedAt: string | null;
  memoryId: string | null;
}

export interface NativeResponse<T> {
  protocolVersion: number;
  requestId: string;
  ok: boolean;
  sessionId?: string;
  data?: T;
  error?: string;
}

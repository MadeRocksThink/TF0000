import { relative, sep } from "node:path";
import * as vscode from "vscode";

import type { NativeHostClient } from "./native-client";
import type {
  AgentWriteCandidate,
  CodeReference,
  DashboardSnapshot,
  ProjectContext,
  SearchResponse,
  WorkspaceMapping,
} from "./types";

export interface WorkspaceIdentity {
  uri: string;
  root: string;
}

export class Tf0000Service {
  constructor(private readonly client: NativeHostClient) {}

  dashboard(): Promise<DashboardSnapshot> {
    return this.client.request("dashboard");
  }

  async currentWorkspace(resource?: vscode.Uri): Promise<WorkspaceIdentity> {
    const folder = resource
      ? vscode.workspace.getWorkspaceFolder(resource)
      : vscode.window.activeTextEditor
        ? vscode.workspace.getWorkspaceFolder(vscode.window.activeTextEditor.document.uri)
        : vscode.workspace.workspaceFolders?.[0];
    if (!folder) throw new Error("Open a folder or workspace before using TF0000");
    return { uri: folder.uri.toString(), root: folder.uri.fsPath };
  }

  async mapping(resource?: vscode.Uri): Promise<WorkspaceMapping | null> {
    const workspace = await this.currentWorkspace(resource);
    return this.client.request("workspaceMapping", { workspaceUri: workspace.uri });
  }

  async mapWorkspace(projectId: string, resource?: vscode.Uri): Promise<WorkspaceMapping> {
    const workspace = await this.currentWorkspace(resource);
    return this.client.request("mapWorkspace", {
      workspaceUri: workspace.uri,
      repositoryRoot: workspace.root,
      projectId,
    });
  }

  async saveReference(
    document: vscode.TextDocument,
    selection: vscode.Selection | null,
  ): Promise<CodeReference> {
    if (document.uri.scheme !== "file") throw new Error("Only local workspace files can be saved");
    const workspace = await this.currentWorkspace(document.uri);
    const relativePath = relative(workspace.root, document.uri.fsPath).split(sep).join("/");
    if (!relativePath || relativePath.startsWith("../") || relativePath === "..") {
      throw new Error("The selected file is outside the mapped workspace");
    }
    const hasSelection = selection && !selection.isEmpty;
    const content = hasSelection ? document.getText(selection) : document.getText();
    if (!content.trim()) throw new Error("The selected source is empty");
    return this.client.request("saveCodeReference", {
      workspaceUri: workspace.uri,
      relativePath,
      language: document.languageId,
      startLine: hasSelection ? selection.start.line + 1 : null,
      endLine: hasSelection ? selection.end.line + 1 : null,
      content,
    });
  }

  async codeReferences(): Promise<CodeReference[]> {
    const mapping = await this.requireMapping();
    return this.client.request("codeReferences", { projectId: mapping.projectId });
  }

  async getProjectContext(maximumCharacters?: number): Promise<ProjectContext> {
    const mapping = await this.requireMapping();
    const configured = vscode.workspace
      .getConfiguration("tf0000")
      .get<number>("projectContextMaximumCharacters", 24_000);
    return this.client.request("getProjectContext", {
      projectId: mapping.projectId,
      maximumCharacters: maximumCharacters ?? configured,
    });
  }

  async searchDecisions(query: string, limit = 20): Promise<SearchResponse> {
    const mapping = await this.requireMapping();
    return this.client.request("searchDecisions", {
      projectId: mapping.projectId,
      query,
      limit,
    });
  }

  async createAgentCandidate(input: {
    memoryType: string;
    title: string;
    content: string;
  }): Promise<AgentWriteCandidate> {
    const mapping = await this.requireMapping();
    return this.client.request("createAgentWriteCandidate", {
      projectId: mapping.projectId,
      requestedBy: "vscode-language-model-tool",
      ...input,
    });
  }

  pendingCandidates(): Promise<AgentWriteCandidate[]> {
    return this.client.request("agentWriteCandidates", { status: "pending" });
  }

  reviewCandidate(candidateId: string, action: "accept" | "reject"): Promise<AgentWriteCandidate> {
    return this.client.request("reviewAgentWriteCandidate", {
      candidateId,
      action,
      confirmed: true,
      memorySpaceId: null,
    });
  }

  async requireMapping(): Promise<WorkspaceMapping> {
    const mapping = await this.mapping();
    if (!mapping) {
      throw new Error("Map this workspace to a TF0000 project first");
    }
    return mapping;
  }
}

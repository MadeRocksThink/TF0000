import * as vscode from "vscode";

import type { Tf0000Service } from "./service";
import type { AgentWriteCandidate, CodeReference } from "./types";

export class ContextTreeProvider implements vscode.TreeDataProvider<ContextNode> {
  private readonly changed = new vscode.EventEmitter<ContextNode | undefined>();
  readonly onDidChangeTreeData = this.changed.event;

  constructor(private readonly service: Tf0000Service) {}

  refresh(): void {
    this.changed.fire(undefined);
  }

  getTreeItem(element: ContextNode): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: ContextNode): Promise<ContextNode[]> {
    if (element) return element.children ?? [];
    try {
      const mapping = await this.service.mapping();
      if (!mapping) {
        return [
          new ContextNode(
            "Workspace not mapped",
            "Choose a TF0000 project",
            vscode.TreeItemCollapsibleState.None,
            new vscode.ThemeIcon("link"),
            { command: "tf0000.mapWorkspace", title: "Map Workspace" },
          ),
        ];
      }
      const [dashboard, references, candidates] = await Promise.all([
        this.service.dashboard(),
        this.service.codeReferences(),
        this.service.pendingCandidates(),
      ]);
      const decisions = dashboard.memories
        .filter(
          (memory) =>
            memory.projectId === mapping.projectId &&
            memory.memoryType === "decision" &&
            memory.status === "active",
        )
        .map(
          (memory) =>
            new ContextNode(
              memory.title,
              memory.authority.replace("_", " "),
              vscode.TreeItemCollapsibleState.None,
              new vscode.ThemeIcon("check"),
              {
                command: "tf0000.showText",
                title: "Show Decision",
                arguments: [memory.title, memory.currentContent],
              },
              memory.currentContent,
            ),
        );
      const referenceNodes = references.slice(0, 30).map(referenceNode);
      const candidateNodes = candidates
        .filter((candidate) => candidate.projectId === mapping.projectId)
        .map(candidateNode);
      return [
        new ContextNode(
          mapping.projectName,
          "Mapped project",
          vscode.TreeItemCollapsibleState.None,
          new vscode.ThemeIcon("folder-library"),
          { command: "tf0000.mapWorkspace", title: "Change Mapping" },
          mapping.repositoryRoot,
        ),
        new ContextNode(
          `Decisions (${decisions.length})`,
          undefined,
          vscode.TreeItemCollapsibleState.Expanded,
          new vscode.ThemeIcon("lightbulb"),
          undefined,
          undefined,
          decisions,
        ),
        new ContextNode(
          `Code references (${references.length})`,
          undefined,
          vscode.TreeItemCollapsibleState.Collapsed,
          new vscode.ThemeIcon("references"),
          undefined,
          undefined,
          referenceNodes,
        ),
        new ContextNode(
          `Pending agent candidates (${candidateNodes.length})`,
          undefined,
          vscode.TreeItemCollapsibleState.Collapsed,
          new vscode.ThemeIcon("request-changes"),
          { command: "tf0000.reviewCandidates", title: "Review Candidates" },
          undefined,
          candidateNodes,
        ),
      ];
    } catch (error) {
      return [
        new ContextNode(
          "TF0000 unavailable",
          error instanceof Error ? error.message : String(error),
          vscode.TreeItemCollapsibleState.None,
          new vscode.ThemeIcon("warning"),
          { command: "tf0000.configureNativeHost", title: "Configure Native Host" },
        ),
      ];
    }
  }
}

export class ContextNode extends vscode.TreeItem {
  constructor(
    label: string,
    description: string | undefined,
    collapsibleState: vscode.TreeItemCollapsibleState,
    iconPath: vscode.ThemeIcon,
    command?: vscode.Command,
    tooltip?: string,
    readonly children?: ContextNode[],
  ) {
    super(label, collapsibleState);
    if (description !== undefined) this.description = description;
    this.iconPath = iconPath;
    if (command !== undefined) this.command = command;
    if (tooltip !== undefined) this.tooltip = tooltip;
  }
}

function referenceNode(reference: CodeReference): ContextNode {
  const lines =
    reference.startLine && reference.endLine ? `:${reference.startLine}-${reference.endLine}` : "";
  return new ContextNode(
    `${reference.relativePath}${lines}`,
    reference.language,
    vscode.TreeItemCollapsibleState.None,
    new vscode.ThemeIcon("code"),
    {
      command: "tf0000.showText",
      title: "Show Code Reference",
      arguments: [reference.relativePath, reference.content, reference.language],
    },
    reference.content,
  );
}

function candidateNode(candidate: AgentWriteCandidate): ContextNode {
  return new ContextNode(
    candidate.title,
    candidate.memoryType,
    vscode.TreeItemCollapsibleState.None,
    new vscode.ThemeIcon("edit"),
    {
      command: "tf0000.reviewCandidate",
      title: "Review Candidate",
      arguments: [candidate],
    },
    candidate.content,
  );
}

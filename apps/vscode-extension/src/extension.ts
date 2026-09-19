import * as vscode from "vscode";

import { NativeHostClient, resolveNativeHostPath } from "./native-client";
import { Tf0000Service } from "./service";
import { registerLanguageModelTools } from "./tools";
import { ContextTreeProvider } from "./tree";
import type { AgentWriteCandidate } from "./types";

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel("TF0000");
  const configuredPath = vscode.workspace
    .getConfiguration("tf0000")
    .get<string>("nativeHostPath", "");
  let executable: string;
  try {
    executable = resolveNativeHostPath(context.extensionPath, configuredPath);
  } catch (error) {
    executable = configuredPath;
    void showError(error);
  }
  const client = new NativeHostClient(executable, (message) => output.appendLine(message));
  const service = new Tf0000Service(client);
  const tree = new ContextTreeProvider(service);
  const view = vscode.window.createTreeView("tf0000.contextView", { treeDataProvider: tree });

  context.subscriptions.push(output, client, view);
  registerLanguageModelTools(context, service, () => tree.refresh());

  context.subscriptions.push(
    vscode.commands.registerCommand("tf0000.refresh", () => tree.refresh()),
    vscode.commands.registerCommand("tf0000.mapWorkspace", () =>
      runCommand(async () => {
        const dashboard = await service.dashboard();
        const projects = dashboard.projects.filter((project) => !project.archivedAt);
        if (projects.length === 0) {
          throw new Error("Create a project in the TF0000 desktop application first");
        }
        const picked = await vscode.window.showQuickPick(
          projects.map((project) => ({
            label: project.name,
            description: project.description,
            project,
          })),
          { title: "Map this workspace to a TF0000 project", placeHolder: "Choose a project" },
        );
        if (!picked) return;
        const mapping = await service.mapWorkspace(picked.project.id);
        tree.refresh();
        void vscode.window.showInformationMessage(
          `TF0000 mapped this workspace to ${mapping.projectName}.`,
        );
      }),
    ),
    vscode.commands.registerCommand("tf0000.saveSelection", () =>
      runCommand(async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.selection.isEmpty) {
          throw new Error("Select code or text in an editor first");
        }
        const reference = await service.saveReference(editor.document, editor.selection);
        tree.refresh();
        void vscode.window.showInformationMessage(
          `TF0000 saved ${reference.relativePath}:${reference.startLine}-${reference.endLine}.`,
        );
      }),
    ),
    vscode.commands.registerCommand("tf0000.saveFile", (resource?: vscode.Uri) =>
      runCommand(async () => {
        const document = resource
          ? await vscode.workspace.openTextDocument(resource)
          : vscode.window.activeTextEditor?.document;
        if (!document) throw new Error("Open or select a file first");
        const reference = await service.saveReference(document, null);
        tree.refresh();
        void vscode.window.showInformationMessage(
          `TF0000 saved ${reference.relativePath} as a code reference.`,
        );
      }),
    ),
    vscode.commands.registerCommand("tf0000.getProjectContext", () =>
      runCommand(async () => {
        const projectContext = await service.getProjectContext();
        await showText(
          `${projectContext.projectName} — TF0000 context`,
          projectContext.text,
          "markdown",
        );
      }),
    ),
    vscode.commands.registerCommand("tf0000.searchDecisions", () =>
      runCommand(async () => {
        const query = await vscode.window.showInputBox({
          title: "Search TF0000 project decisions",
          prompt: "Describe the decision you need",
          validateInput: (value) => (value.trim() ? undefined : "Enter a search query"),
        });
        if (!query) return;
        const response = await service.searchDecisions(query);
        if (response.results.length === 0) {
          void vscode.window.showInformationMessage("No matching active decision is recorded.");
          return;
        }
        const picked = await vscode.window.showQuickPick(
          response.results.map((result) => ({
            label: result.title,
            description: result.authority?.replace("_", " ") ?? "",
            detail: result.excerpt,
            result,
          })),
          { title: `TF0000 decisions matching “${query}”`, matchOnDetail: true },
        );
        if (picked) await showText(picked.result.title, picked.result.excerpt, "markdown");
      }),
    ),
    vscode.commands.registerCommand("tf0000.reviewCandidates", () =>
      runCommand(async () => {
        const candidates = await service.pendingCandidates();
        const mapping = await service.requireMapping();
        const scoped = candidates.filter((candidate) => candidate.projectId === mapping.projectId);
        if (scoped.length === 0) {
          void vscode.window.showInformationMessage(
            "No pending agent candidates for this project.",
          );
          return;
        }
        const picked = await vscode.window.showQuickPick(
          scoped.map((candidate) => ({
            label: candidate.title,
            description: candidate.memoryType,
            detail: candidate.content,
            candidate,
          })),
          { title: "Review TF0000 agent write candidates", matchOnDetail: true },
        );
        if (picked) await reviewCandidate(service, tree, picked.candidate);
      }),
    ),
    vscode.commands.registerCommand("tf0000.reviewCandidate", (candidate: AgentWriteCandidate) =>
      runCommand(() => reviewCandidate(service, tree, candidate)),
    ),
    vscode.commands.registerCommand(
      "tf0000.showText",
      (title: string, content: string, language = "markdown") => showText(title, content, language),
    ),
    vscode.commands.registerCommand("tf0000.configureNativeHost", () =>
      runCommand(async () => {
        const dialogOptions: vscode.OpenDialogOptions = {
          title: "Choose tf0000-native-host",
          canSelectFiles: true,
          canSelectFolders: false,
          canSelectMany: false,
        };
        if (process.platform === "win32") dialogOptions.filters = { Executable: ["exe"] };
        const selected = await vscode.window.showOpenDialog(dialogOptions);
        const path = selected?.[0]?.fsPath;
        if (!path) return;
        await vscode.workspace
          .getConfiguration("tf0000")
          .update("nativeHostPath", path, vscode.ConfigurationTarget.Global);
        const reload = await vscode.window.showInformationMessage(
          "TF0000 native host path saved. Reload VS Code to reconnect.",
          "Reload Window",
        );
        if (reload) await vscode.commands.executeCommand("workbench.action.reloadWindow");
      }),
    ),
  );
}

export function deactivate(): void {}

async function reviewCandidate(
  service: Tf0000Service,
  tree: ContextTreeProvider,
  candidate: AgentWriteCandidate,
): Promise<void> {
  const action = await vscode.window.showWarningMessage(
    `Review ${candidate.memoryType} candidate “${candidate.title}”`,
    { modal: true, detail: candidate.content },
    "Accept as Draft",
    "Reject",
  );
  if (!action) return;
  const reviewed = await service.reviewCandidate(
    candidate.id,
    action === "Accept as Draft" ? "accept" : "reject",
  );
  tree.refresh();
  void vscode.window.showInformationMessage(
    reviewed.status === "accepted"
      ? "Candidate accepted as a draft AI suggestion."
      : "Candidate rejected.",
  );
}

async function showText(title: string, content: string, language: string): Promise<void> {
  const document = await vscode.workspace.openTextDocument({ language, content });
  await vscode.window.showTextDocument(document, { preview: true });
  void vscode.window.setStatusBarMessage(title, 3_000);
}

async function runCommand(action: () => Promise<void>): Promise<void> {
  try {
    await action();
  } catch (error) {
    await showError(error);
  }
}

async function showError(error: unknown): Promise<void> {
  const message = error instanceof Error ? error.message : String(error);
  if (/native host|spawn|enoent/i.test(message)) {
    const choice = await vscode.window.showErrorMessage(
      `TF0000: ${message}`,
      "Configure Native Host",
    );
    if (choice) await vscode.commands.executeCommand("tf0000.configureNativeHost");
    return;
  }
  await vscode.window.showErrorMessage(`TF0000: ${message}`);
}

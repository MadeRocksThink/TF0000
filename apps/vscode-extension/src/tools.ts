import * as vscode from "vscode";

import type { Tf0000Service } from "./service";

interface ProjectContextParameters {
  maximumCharacters?: number;
}

interface SearchDecisionsParameters {
  query: string;
  limit?: number;
}

interface SaveMemoryCandidateParameters {
  memoryType: string;
  title: string;
  content: string;
}

export function registerLanguageModelTools(
  context: vscode.ExtensionContext,
  service: Tf0000Service,
  onWriteCandidate: () => void,
): void {
  context.subscriptions.push(
    vscode.lm.registerTool("tf0000_getProjectContext", new GetProjectContextTool(service)),
    vscode.lm.registerTool("tf0000_searchDecisions", new SearchDecisionsTool(service)),
    vscode.lm.registerTool(
      "tf0000_saveMemoryCandidate",
      new SaveMemoryCandidateTool(service, onWriteCandidate),
    ),
  );
}

class GetProjectContextTool implements vscode.LanguageModelTool<ProjectContextParameters> {
  constructor(private readonly service: Tf0000Service) {}

  prepareInvocation(): vscode.ProviderResult<vscode.PreparedToolInvocation> {
    return {
      invocationMessage: "Reading mapped TF0000 project context",
      confirmationMessages: {
        title: "Read TF0000 project context",
        message: "Allow access to confirmed memories and code references in the mapped project?",
      },
    };
  }

  async invoke(
    options: vscode.LanguageModelToolInvocationOptions<ProjectContextParameters>,
    _token: vscode.CancellationToken,
  ): Promise<vscode.LanguageModelToolResult> {
    const context = await this.service.getProjectContext(options.input.maximumCharacters);
    return textResult(context.text);
  }
}

class SearchDecisionsTool implements vscode.LanguageModelTool<SearchDecisionsParameters> {
  constructor(private readonly service: Tf0000Service) {}

  prepareInvocation(
    options: vscode.LanguageModelToolInvocationPrepareOptions<SearchDecisionsParameters>,
  ): vscode.ProviderResult<vscode.PreparedToolInvocation> {
    return {
      invocationMessage: "Searching TF0000 project decisions",
      confirmationMessages: {
        title: "Search TF0000 decisions",
        message: `Search the mapped project for decisions matching “${options.input.query}”?`,
      },
    };
  }

  async invoke(
    options: vscode.LanguageModelToolInvocationOptions<SearchDecisionsParameters>,
    _token: vscode.CancellationToken,
  ): Promise<vscode.LanguageModelToolResult> {
    const response = await this.service.searchDecisions(options.input.query, options.input.limit);
    if (response.results.length === 0)
      return textResult("No matching active decision is recorded.");
    return textResult(
      response.results
        .map(
          (result, index) =>
            `${index + 1}. ${result.title} [${result.authority ?? "unknown authority"}]\n${result.excerpt}`,
        )
        .join("\n\n"),
    );
  }
}

class SaveMemoryCandidateTool implements vscode.LanguageModelTool<SaveMemoryCandidateParameters> {
  constructor(
    private readonly service: Tf0000Service,
    private readonly onWriteCandidate: () => void,
  ) {}

  prepareInvocation(
    options: vscode.LanguageModelToolInvocationPrepareOptions<SaveMemoryCandidateParameters>,
  ): vscode.ProviderResult<vscode.PreparedToolInvocation> {
    const preview = new vscode.MarkdownString();
    preview.appendMarkdown(
      `Create a pending **${options.input.memoryType}** candidate named **${escapeMarkdown(options.input.title)}**?\n\n`,
    );
    preview.appendCodeblock(options.input.content.slice(0, 2_000));
    preview.appendMarkdown(
      "\nThis will not update memory until you review and accept it separately.",
    );
    return {
      invocationMessage: "Creating a review-only TF0000 memory candidate",
      confirmationMessages: {
        title: "Propose TF0000 memory",
        message: preview,
      },
    };
  }

  async invoke(
    options: vscode.LanguageModelToolInvocationOptions<SaveMemoryCandidateParameters>,
    _token: vscode.CancellationToken,
  ): Promise<vscode.LanguageModelToolResult> {
    const candidate = await this.service.createAgentCandidate(options.input);
    this.onWriteCandidate();
    return textResult(
      `Pending candidate “${candidate.title}” was created. It is not active memory and requires explicit human review.`,
    );
  }
}

function textResult(text: string): vscode.LanguageModelToolResult {
  return new vscode.LanguageModelToolResult([new vscode.LanguageModelTextPart(text)]);
}

function escapeMarkdown(value: string): string {
  return value.replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
}

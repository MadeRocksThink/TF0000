import type { NativeHostClient } from "./native-client";

export const TOOL_NAMES = [
  "search_memory",
  "get_memory",
  "get_project_context",
  "get_decisions",
  "get_context_pack",
  "search_chats",
  "save_memory_candidate",
] as const;

export type ToolName = (typeof TOOL_NAMES)[number];

export interface McpGateway {
  invoke(toolName: ToolName, argumentsValue: Record<string, unknown>): Promise<unknown>;
}

export class NativeGateway implements McpGateway {
  private registered = false;

  constructor(
    private readonly client: NativeHostClient,
    readonly clientId: string,
    readonly displayName: string,
  ) {}

  async register(): Promise<void> {
    if (this.registered) return;
    await this.client.request("registerMcpClient", {
      clientId: this.clientId,
      displayName: this.displayName,
    });
    this.registered = true;
  }

  async invoke(toolName: ToolName, argumentsValue: Record<string, unknown>): Promise<unknown> {
    await this.register();
    return this.client.request("mcpToolCall", {
      clientId: this.clientId,
      toolName,
      arguments: argumentsValue,
    });
  }

  async setPermission(options: {
    projectId?: string;
    readAllowed: boolean;
    candidateWriteAllowed: boolean;
  }): Promise<unknown> {
    await this.register();
    return this.client.request("setMcpPermission", {
      clientId: this.clientId,
      projectId: options.projectId,
      readAllowed: options.readAllowed,
      candidateWriteAllowed: options.candidateWriteAllowed,
    });
  }

  async auditLog(limit: number): Promise<unknown> {
    await this.register();
    return this.client.request("mcpAuditLog", { clientId: this.clientId, limit });
  }

  async projects(): Promise<unknown> {
    const dashboard = await this.client.request<{ projects: unknown[] }>("dashboard");
    return dashboard.projects;
  }

  dispose(): void {
    this.client.dispose();
  }
}

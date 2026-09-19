import { Client, InMemoryTransport } from "@modelcontextprotocol/client";
import { afterEach, describe, expect, it } from "vitest";

import type { McpGateway, ToolName } from "./gateway";
import { createTf0000McpServer } from "./server";

class RecordingGateway implements McpGateway {
  readonly calls: Array<{ toolName: ToolName; argumentsValue: Record<string, unknown> }> = [];

  async invoke(toolName: ToolName, argumentsValue: Record<string, unknown>): Promise<unknown> {
    this.calls.push({ toolName, argumentsValue });
    if (toolName === "save_memory_candidate") throw new Error("candidate writes are disabled");
    return { toolName, argumentsValue };
  }
}

const activeClients: Client[] = [];
const activeServers: ReturnType<typeof createTf0000McpServer>[] = [];

afterEach(async () => {
  await Promise.all(activeClients.splice(0).map((client) => client.close()));
  await Promise.all(activeServers.splice(0).map((server) => server.close()));
});

async function connect(profile: string, gateway: McpGateway): Promise<Client> {
  const server = createTf0000McpServer(gateway);
  const client = new Client({ name: profile, version: "1.0.0" });
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  activeClients.push(client);
  activeServers.push(server);
  await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);
  return client;
}

describe("TF0000 MCP interoperability", () => {
  it.each(["vscode-mcp", "desktop-agent"])(
    "lists the complete tool surface for %s",
    async (profile) => {
      const client = await connect(profile, new RecordingGateway());
      const tools = await client.listTools();
      expect(tools.tools.map((tool) => tool.name)).toEqual([
        "search_memory",
        "get_memory",
        "get_project_context",
        "get_decisions",
        "get_context_pack",
        "search_chats",
        "save_memory_candidate",
      ]);
    },
  );

  it("validates inputs and routes a read tool through the gateway", async () => {
    const gateway = new RecordingGateway();
    const client = await connect("tool-caller", gateway);
    const projectId = "00000000-0000-4000-8000-000000000001";
    const result = await client.callTool({
      name: "get_project_context",
      arguments: { projectId, maximumCharacters: 12_000 },
    });
    expect(result.isError).not.toBe(true);
    expect(gateway.calls).toEqual([
      {
        toolName: "get_project_context",
        argumentsValue: { projectId, maximumCharacters: 12_000 },
      },
    ]);
  });

  it("returns permission failures as MCP tool errors", async () => {
    const client = await connect("read-only-client", new RecordingGateway());
    const result = await client.callTool({
      name: "save_memory_candidate",
      arguments: {
        projectId: "00000000-0000-4000-8000-000000000001",
        memoryType: "decision",
        title: "Candidate",
        content: "This must remain pending.",
      },
    });
    expect(result.isError).toBe(true);
    expect(result.content[0]).toMatchObject({
      type: "text",
      text: "candidate writes are disabled",
    });
  });
});

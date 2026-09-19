import { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";

import type { McpGateway, ToolName } from "./gateway";

const projectId = z.uuid().describe("TF0000 project UUID that scopes this request.");
const limit = z.number().int().min(1).max(50).default(20);
const memoryType = z.enum([
  "decision",
  "requirement",
  "fact",
  "preference",
  "suggestion",
  "idea",
  "rejected_idea",
  "task",
  "bug",
  "question",
  "reference",
  "summary",
]);

export function createTf0000McpServer(gateway: McpGateway): McpServer {
  const server = new McpServer(
    { name: "tf0000-context", version: "0.1.0" },
    {
      instructions:
        "TF0000 provides source-grounded local context. Reads are project-scoped. " +
        "save_memory_candidate only creates a pending candidate for human review; it never writes directly to memory.",
    },
  );

  server.registerTool(
    "search_memory",
    {
      title: "Search TF0000 memory",
      description:
        "Search active source-aware memories in one TF0000 project. Returns recorded evidence, not a generated answer.",
      inputSchema: z.strictObject({
        projectId,
        query: z.string().trim().min(1).max(500),
        memoryType: memoryType.optional(),
        status: z.enum(["active", "draft", "superseded", "rejected", "archived"]).default("active"),
        limit,
      }),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true },
    },
    (argumentsValue) => invoke(gateway, "search_memory", argumentsValue),
  );

  server.registerTool(
    "get_memory",
    {
      title: "Get TF0000 memory",
      description: "Retrieve one exact TF0000 memory by its memory UUID.",
      inputSchema: z.strictObject({ memoryId: z.uuid() }),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true },
    },
    (argumentsValue) => invoke(gateway, "get_memory", argumentsValue),
  );

  server.registerTool(
    "get_project_context",
    {
      title: "Get TF0000 project context",
      description:
        "Retrieve prioritized current memories and saved code references for one TF0000 project.",
      inputSchema: z.strictObject({
        projectId,
        maximumCharacters: z.number().int().min(1_000).max(200_000).default(24_000),
      }),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true },
    },
    (argumentsValue) => invoke(gateway, "get_project_context", argumentsValue),
  );

  server.registerTool(
    "get_decisions",
    {
      title: "Get TF0000 decisions",
      description:
        "List current project decisions, optionally narrowed by a full-text search query.",
      inputSchema: z.strictObject({
        projectId,
        query: z.string().trim().max(500).optional(),
        limit,
      }),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true },
    },
    (argumentsValue) => invoke(gateway, "get_decisions", argumentsValue),
  );

  server.registerTool(
    "get_context_pack",
    {
      title: "Get TF0000 context pack",
      description:
        "Retrieve a reusable context pack, its ordered items, and its composed exact text.",
      inputSchema: z.strictObject({ packId: z.uuid() }),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true },
    },
    (argumentsValue) => invoke(gateway, "get_context_pack", argumentsValue),
  );

  server.registerTool(
    "search_chats",
    {
      title: "Search captured TF0000 chats",
      description: "Search captured chat messages and exact fragments within one TF0000 project.",
      inputSchema: z.strictObject({
        projectId,
        query: z.string().trim().min(1).max(500),
        provider: z.string().trim().min(1).max(80).optional(),
        dateFrom: z.string().trim().max(80).optional(),
        dateTo: z.string().trim().max(80).optional(),
        limit,
      }),
      annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true },
    },
    (argumentsValue) => invoke(gateway, "search_chats", argumentsValue),
  );

  server.registerTool(
    "save_memory_candidate",
    {
      title: "Propose TF0000 memory candidate",
      description:
        "Create a pending memory candidate for explicit human review. This cannot directly create, replace, or confirm a memory.",
      inputSchema: z.strictObject({
        projectId,
        memoryType,
        title: z.string().trim().min(1).max(160),
        content: z.string().trim().min(1).max(1_000_000),
      }),
      annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: false },
    },
    (argumentsValue) => invoke(gateway, "save_memory_candidate", argumentsValue),
  );

  return server;
}

async function invoke(
  gateway: McpGateway,
  toolName: ToolName,
  argumentsValue: Record<string, unknown>,
) {
  try {
    const result = await gateway.invoke(toolName, argumentsValue);
    return {
      content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
    };
  } catch (error) {
    return {
      isError: true,
      content: [{ type: "text" as const, text: errorMessage(error) }],
    };
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

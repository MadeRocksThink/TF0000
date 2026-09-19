import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { Client } from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

const temporaryDirectory = mkdtempSync(resolve(tmpdir(), "tf0000-mcp-smoke-"));
const database = resolve(temporaryDirectory, "context.db");
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const entry = resolve(scriptDirectory, "..", "dist", "tf0000-mcp.mjs");
const client = new Client({ name: "tf0000-stdio-smoke", version: "1.0.0" });
const transport = new StdioClientTransport({
  command: process.execPath,
  args: [entry, "serve", "--client-id", "stdio-smoke"],
  env: { ...process.env, TF0000_CONTEXT_DB: database },
  stderr: "pipe",
});

try {
  await client.connect(transport);
  const result = await client.listTools();
  if (result.tools.length !== 7) {
    throw new Error(`Expected 7 MCP tools, received ${result.tools.length}`);
  }
  process.stdout.write("TF0000 MCP stdio smoke test passed (7 tools)\n");
} finally {
  await client.close();
  rmSync(temporaryDirectory, { recursive: true, force: true });
}

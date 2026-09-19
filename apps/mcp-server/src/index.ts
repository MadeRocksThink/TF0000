import { serveStdio } from "@modelcontextprotocol/server/stdio";

import { parseCli } from "./cli";
import { NativeGateway } from "./gateway";
import { NativeHostClient } from "./native-client";
import { createTf0000McpServer } from "./server";

async function main(): Promise<void> {
  const options = parseCli(process.argv.slice(2));
  const gateway = new NativeGateway(new NativeHostClient(), options.clientId, options.displayName);
  if (options.command === "permissions") {
    const permission = await gateway.setPermission({
      ...(options.projectId ? { projectId: options.projectId } : {}),
      readAllowed: options.readAllowed ?? true,
      candidateWriteAllowed: options.candidateWriteAllowed ?? false,
    });
    process.stdout.write(`${JSON.stringify(permission, null, 2)}\n`);
    gateway.dispose();
    return;
  }
  if (options.command === "projects") {
    const projects = await gateway.projects();
    process.stdout.write(`${JSON.stringify(projects, null, 2)}\n`);
    gateway.dispose();
    return;
  }
  if (options.command === "audit") {
    const entries = await gateway.auditLog(options.limit);
    process.stdout.write(`${JSON.stringify(entries, null, 2)}\n`);
    gateway.dispose();
    return;
  }

  await gateway.register();
  const handle = serveStdio(() => createTf0000McpServer(gateway));
  const close = (): void => {
    void handle.close().finally(() => gateway.dispose());
  };
  process.once("SIGINT", close);
  process.once("SIGTERM", close);
  process.stderr.write(`TF0000 MCP server ready for client '${options.clientId}'\n`);
}

void main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`TF0000 MCP server stopped: ${message}\n`);
  process.exitCode = 1;
});

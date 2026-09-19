export interface CliOptions {
  command: "serve" | "projects" | "permissions" | "audit";
  clientId: string;
  displayName: string;
  projectId?: string;
  readAllowed?: boolean;
  candidateWriteAllowed?: boolean;
  limit: number;
}

export function parseCli(argumentsValue: string[]): CliOptions {
  const commandValue = argumentsValue[0]?.startsWith("--") ? "serve" : argumentsValue.shift();
  if (commandValue && !["serve", "projects", "permissions", "audit"].includes(commandValue)) {
    throw new Error(`Unknown command: ${commandValue}`);
  }
  const values = new Map<string, string>();
  for (let index = 0; index < argumentsValue.length; index += 2) {
    const flag = argumentsValue[index];
    const value = argumentsValue[index + 1];
    if (!flag?.startsWith("--") || value === undefined) {
      throw new Error(`Expected --option value, received: ${flag ?? "end of input"}`);
    }
    values.set(flag.slice(2), value);
  }
  const clientId = values.get("client-id") ?? process.env.TF0000_MCP_CLIENT_ID ?? "local-mcp";
  const displayName = values.get("display-name") ?? clientId;
  validateClientId(clientId);
  const command = (commandValue ?? "serve") as CliOptions["command"];
  if (command === "permissions") {
    if (!values.has("read") || !values.has("candidate-write")) {
      throw new Error("permissions requires --read allow|deny and --candidate-write allow|deny");
    }
  }
  const projectId = values.get("project-id");
  return {
    command,
    clientId,
    displayName,
    ...(projectId ? { projectId } : {}),
    ...(values.has("read") ? { readAllowed: parseAllowed(values.get("read")) } : {}),
    ...(values.has("candidate-write")
      ? { candidateWriteAllowed: parseAllowed(values.get("candidate-write")) }
      : {}),
    limit: parseLimit(values.get("limit")),
  };
}

function validateClientId(value: string): void {
  if (!/^[A-Za-z0-9._:-]{1,100}$/.test(value)) {
    throw new Error(
      "client ID may contain only letters, numbers, dot, underscore, colon and hyphen",
    );
  }
}

function parseAllowed(value: string | undefined): boolean {
  if (value === "allow") return true;
  if (value === "deny") return false;
  throw new Error("permission values must be allow or deny");
}

function parseLimit(value: string | undefined): number {
  if (value === undefined) return 100;
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > 500) {
    throw new Error("limit must be an integer between 1 and 500");
  }
  return parsed;
}

import { describe, expect, it } from "vitest";

import { parseCli } from "./cli";

describe("MCP CLI", () => {
  it("defaults every new client to the local read-only profile", () => {
    expect(parseCli([])).toMatchObject({
      command: "serve",
      clientId: "local-mcp",
      displayName: "local-mcp",
    });
  });

  it("parses an explicit project permission update", () => {
    expect(
      parseCli([
        "permissions",
        "--client-id",
        "codex-local",
        "--project-id",
        "00000000-0000-4000-8000-000000000001",
        "--read",
        "allow",
        "--candidate-write",
        "deny",
      ]),
    ).toMatchObject({
      command: "permissions",
      clientId: "codex-local",
      readAllowed: true,
      candidateWriteAllowed: false,
    });
  });

  it("supports listing project IDs without starting the protocol server", () => {
    expect(parseCli(["projects", "--client-id", "setup"])).toMatchObject({
      command: "projects",
      clientId: "setup",
    });
  });
});

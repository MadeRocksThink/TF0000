# TF0000 MCP server

> Experimental integration: a known Context Pack permission issue can expose a memory from a denied project. Read [known limitations](KNOWN_LIMITATIONS.md) before configuring sensitive projects.

TF0000 exposes the canonical TF0000 database to compatible local MCP clients. The server uses
stdio, starts one process per client, and does not expose a local HTTP port.

## Build

Paths using `C:\path\TF0000` are examples; replace them with your repository location.

```powershell
pnpm build:mcp
```

The packaged server is written to:

```text
target/release/tf0000-mcp/
├─ tf0000-mcp.mjs
├─ tf0000-mcp.cmd
├─ tf0000-mcp
└─ bin/tf0000-native-host.exe
```

Node.js 20 or newer is required to launch the packaged JavaScript entry point.

## Available tools

| Tool | Access | Purpose |
|---|---|---|
| `search_memory` | Read | Search current or explicitly selected memory states in one project. |
| `get_memory` | Read | Retrieve one exact memory by UUID. |
| `get_project_context` | Read | Retrieve prioritized memories and saved code references. |
| `get_decisions` | Read | List or search active project decisions. |
| `get_context_pack` | Read | Retrieve a pack, ordered items, and composed text. |
| `search_chats` | Read | Search captured messages and exact fragments. |
| `save_memory_candidate` | Candidate write | Create a pending candidate for human review. |

There is no MCP tool that directly creates, updates, accepts, replaces, or deletes a memory.
Accepted candidates still require the existing TF0000 review flow and become draft AI suggestions.

## Find a project ID

Run this after launching TF0000 at least once:

```powershell
node C:\path\TF0000\target\release\tf0000-mcp\tf0000-mcp.mjs projects
```

The result lists the same projects shown in the desktop application.

## Client permissions

A client ID is local configuration, not a remote identity. Use a different stable ID for each MCP
host. A newly seen client is read-only by default: reads are allowed and memory-candidate writes are
denied.

Set a project-specific policy:

```powershell
node C:\path\TF0000\target\release\tf0000-mcp\tf0000-mcp.mjs permissions `
  --client-id vscode `
  --project-id PROJECT_UUID `
  --read allow `
  --candidate-write allow
```

Use `deny` instead of `allow` to revoke either capability. Omit `--project-id` to change the
client's global fallback. Permission changes never enable direct memory writes.

Inspect recent access without exposing query or memory contents in the log:

```powershell
node C:\path\TF0000\target\release\tf0000-mcp\tf0000-mcp.mjs audit `
  --client-id vscode `
  --limit 100
```

The audit records client, tool, access class, project/entity identifiers, outcome, and timestamp.

## VS Code and Copilot

Create `.vscode/mcp.json` in the repository that will use TF0000:

```json
{
  "servers": {
    "tf0000": {
      "type": "stdio",
      "command": "node",
      "args": [
        "C:\\path\\TF0000\\target\\release\\tf0000-mcp\\tf0000-mcp.mjs",
        "serve",
        "--client-id",
        "vscode",
        "--display-name",
        "VS Code Copilot"
      ]
    }
  }
}
```

Run **MCP: List Servers**, confirm `tf0000` is running, and enable its tools in Copilot Agent mode.

## Claude Code

Register the same stdio command with a distinct client ID:

```powershell
claude mcp add tf0000 -- node C:\path\TF0000\target\release\tf0000-mcp\tf0000-mcp.mjs serve --client-id claude-code --display-name "Claude Code"
```

Run `/mcp` inside Claude Code to inspect the seven tools.

## Cursor

Create `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "tf0000": {
      "command": "node",
      "args": [
        "C:\\path\\TF0000\\target\\release\\tf0000-mcp\\tf0000-mcp.mjs",
        "serve",
        "--client-id",
        "cursor",
        "--display-name",
        "Cursor"
      ]
    }
  }
}
```

## Inspector and automated checks

The MCP Inspector can launch the package directly:

```powershell
npx @modelcontextprotocol/inspector node C:\path\TF0000\target\release\tf0000-mcp\tf0000-mcp.mjs serve --client-id inspector
```

In the Inspector, connect over stdio, list tools, and call a read tool using a project UUID.

Repository verification:

```powershell
pnpm typecheck
pnpm test
pnpm lint
pnpm build:phase11
```

The tests exercise the official MCP client against the server in memory, native framing, default
read-only policy, project overrides, candidate-write denial, pending-candidate creation, audit
records, migrations, and all core retrieval operations.

## Troubleshooting

- Keep stdout reserved for MCP JSON-RPC. TF0000 writes server diagnostics only to stderr.
- Set `TF0000_NATIVE_HOST` to an existing absolute native-host path only when the bundled host
  cannot be discovered.
- Set `TF0000_CONTEXT_DB` to an alternate database path for isolated development or testing.
- If tools are visible but a call is denied, inspect the client ID in the host configuration and
  use the `permissions` command for that exact ID.

# Architecture

TF0000 uses a shared local SQLite store. Provider-specific page layouts stay in adapters; the core stores normalized conversations, source references, and reusable memories.

```text
Browser pages → Browser extension ──┐
VS Code      → Editor extension ────┼→ Native messaging host → ContextStore → SQLite
AI clients   → MCP stdio server ────┘                              ↑
                                                       Tauri desktop commands
                                                                  ↑
                                                         React desktop UI
```

## Components

| Component | Responsibility |
| --- | --- |
| `apps/desktop` | React interface and Tauri commands for organization, search, review, recovery, and sync |
| `crates/context-core` | SQLite migrations, entity validation, provenance, history, search, backup/restore, and portable merge |
| `apps/native-host` | Native messaging protocol and integration access to the core |
| `apps/browser-extension` | Provider detection, visible capture, context controls, and composer insertion |
| `apps/vscode-extension` | Workspace mapping, code references, context browsing, and Copilot tools |
| `apps/mcp-server` | Local stdio tools, client identity, and native-host requests |
| `packages/adapter-sdk` | Provider-neutral adapter contract and conformance helpers |
| `packages/context-format` | Portable project types, JSON schema, and parsing |
| `packages/encrypted-sync` | Authenticated encrypted envelopes for portable snapshots |
| `packages/test-fixtures` | Synthetic page fixtures for adapter tests |

## Data and review

Captured text retains provider and source information. Memories keep current content plus a version history. Merge previews collect source selections and show proposed changes before application. Generated candidates enter a review queue and do not automatically become confirmed memory.

The desktop, browser integration, and editor integration share local storage; a conversation in an external AI service remains external. TF0000 transfers selected text, not a provider's hidden session state.

## Optional synchronization

The desktop exports selected projects in the portable format, encrypts the payload, and writes a device snapshot to a chosen folder. Another device decrypts a snapshot before the core validates and merges its records. TF0000 does not operate a sync server.

Encryption and merge correctness are separate concerns. See [sync setup](encrypted-sync.md), [portable format](portable-context-format.md), and [known limitations](KNOWN_LIMITATIONS.md).

## Development boundaries

Keep website selectors in adapters, canonical behavior in the core, and presentation in the interfaces. Maintain migrations and regression tests with behavioral changes. The local smart layer is optional; core storage and search do not depend on an AI provider.

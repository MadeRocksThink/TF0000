<p align="center">
  <img src="docs/assets/maderocksthink-logo.png" width="260" alt="MadeRocksThink">
</p>

<h1 align="center">TF0000</h1>

<p align="center"><strong>Your context, across every AI.</strong></p>

<p align="center">
  Save, organize, and carry the context you choose between ChatGPT, Claude, Gemini, and your coding tools.
</p>

<p align="center">
  <a href="LICENSE">MIT licensed</a> · Local-first · Open source · No AI API required for core features
</p>

<p align="center">
  <a href="#installation">Get started</a> ·
  <a href="docs/USER_GUIDE.md">User guide</a> ·
  <a href="#supported-tools">Supported tools</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

![TF0000 desktop workspace showing projects, memories, and activity](docs/assets/overview.png)

*Application screenshots use sample data. Browser screenshots show the implemented interface in a test environment.*

## Why TF0000?

You plan in one AI tool, research in another, and write code in a third. Every switch means explaining the same project again.

TF0000 keeps useful conversations, decisions, requirements, and instructions in a local workspace. Select what matters, inspect the outgoing text, and bring it into the next conversation.

```text
Capture a chat → Save useful context → Review a handoff → Continue in another tool
```

## Features

- **Capture what matters.** Save visible chats, selected messages, exact text fragments, pasted notes, and code references.
- **Organize your work.** Create projects, nested memory spaces, reusable memories, and Context Packs.
- **Carry context between chats.** Prepare full, minimal, or custom handoffs for ChatGPT, Claude, and Gemini.
- **Choose what stays active.** Enable or disable context for a conversation and attach items for a limited number of insertions.
- **Review changes.** Combine sources, inspect a before-and-after preview, track versions, restore earlier content, and create branches.
- **Find the evidence.** Search locally and inspect the source behind a saved memory. Ask Memory returns recorded evidence or a “Not recorded” result.
- **Work with coding tools.** Save editor selections through VS Code and retrieve context through Copilot tools or local MCP clients.
- **Back up your knowledge.** Export supported formats, create a database backup, and check database health.
- **Explore optional assistance.** Use local summaries, candidate extraction, and similarity search. Encrypted folder sync is available experimentally.

## You control the context

Decide which memories to reuse, which sources to include, and what the destination receives. Preview the exact text before insertion. The browser extension fills the composer; you review and send the message.

Context Packs bundle related memories. Per-chat switches choose what is active. Temporary attachments can count down after successful insertions; clipboard copies do not consume that count. Automatic time-based expiry has a known issue, so remove temporary material manually when finished.

## Local-first

Chats, memories, Context Packs, and search indexes live on your device. Core capture, organization, search, and reuse need no TF0000 account, AI API key, local LLM, or paid AI subscription. The AI services you use may have their own account requirements.

Inserting text into an external AI website makes it available to that service. Review the preview before transferring it. Local databases and ordinary exports are not encrypted by TF0000; optional encrypted sync protects its snapshot files, not the local database.

## Supported tools

| Tool | Available integration | Current verification |
| --- | --- | --- |
| ChatGPT web | Capture, selection, insertion, and handoff | Adapter tested on saved page fixtures |
| Claude web | Capture, selection, insertion, and handoff | Adapter tested on saved page fixtures |
| Gemini web | Capture, selection, insertion, and handoff | Adapter tested on saved page fixtures |
| VS Code | Save code context, map workspaces, browse project context | Extension package builds; installed workflow needs verification |
| GitHub Copilot in VS Code | Retrieve project context and propose memories through tools | Integration implemented; live acceptance pending |
| Claude Code / Cursor | Local MCP setup documented | Server tested; individual clients need verification |
| Other websites and apps | Manual capture and clipboard transfer | No dedicated website adapter |

Direct web adapters target `chatgpt.com`, `claude.ai`, and `gemini.google.com` through the Chrome/Edge extension. Native chat apps do not have equivalent capture adapters. See [adapter compatibility](docs/adapter-compatibility.md).

## Installation

**TF0000 is an experimental source release.** Install from source using the Windows instructions below. Desktop installers and browser-store packages are not provided by this repository yet. See [known limitations](docs/KNOWN_LIMITATIONS.md) before using important data.

### 1. Start the desktop app

Install Node.js 24, pnpm **11.19.0**, current stable Rust with `rustfmt` and `clippy`, Microsoft C++ Build Tools, and WebView2. Then open PowerShell in your downloaded or cloned TF0000 folder:

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

Keep the terminal open while using the development app. Other operating systems have not completed the same installation checks.

### 2. Connect Chrome or Edge

```powershell
pnpm build:extension
pnpm build:native-host
```

1. Open `chrome://extensions` or `edge://extensions` and enable **Developer mode**.
2. Choose **Load unpacked** and select `apps/browser-extension/.output/chrome-mv3` within this repository.
3. Copy the extension ID, then register the local storage bridge:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\register-native-host.ps1 -ExtensionId YOUR_EXTENSION_ID -Browser Both
```

Registration writes a native messaging manifest and browser registration keys for your Windows user. Reload the AI page, open TF0000, and select **Run diagnostics**. See the [browser setup guide](docs/browser-extension.md) for details.

### 3. Add coding integrations when needed

- **VS Code / Copilot:** run `pnpm build:vscode`, then install `target/release/tf0000-vscode.vsix` using **Extensions: Install from VSIX…**. [Setup guide](docs/vscode-extension.md)
- **MCP clients:** run `pnpm build:mcp`, then configure a client to launch the packaged local server. [Setup guide](docs/mcp-server.md)

## Quick start: ChatGPT to Claude

1. Open a ChatGPT conversation and select useful text.
2. Open the TF0000 extension and choose **Save selection**. Use **Save visible chat** when you need the visible conversation.
3. Under **Continue in…**, choose **Claude** and select a full, minimal, or custom handoff.
4. Choose **Build exact handoff preview** and inspect the outgoing text.
5. Choose **Continue in Claude**. If insertion is unavailable, paste the clipboard fallback yourself.
6. Review the destination composer and send when ready.

This carries selected text and evidence between chats. It cannot transfer hidden provider memory or guarantee how the destination model will respond.

<p align="center">
  <img src="docs/assets/extension-handoff.png" width="420" alt="TF0000 preview of a ChatGPT-to-Claude handoff, before insertion">
</p>

For repeat use, save important information as memories and build a Context Pack in the browser extension. The [user guide](docs/USER_GUIDE.md) explains organization, search, packs, history, and recovery.

## Review before updating memory

Bring together a saved conversation, existing memories, and a manual note. Inspect the proposed changes before applying them, then use history to inspect sources or restore an earlier version.

![Merge workspace showing original content and a proposed update](docs/assets/merge-preview.png)

## How it works

```text
ChatGPT / Claude / Gemini → Browser extension ─┐
VS Code / Copilot        → Editor extension ─┼→ Native host → Rust core → SQLite
MCP-compatible clients  → Local MCP server ──┘                    ↑
                                                       Desktop workspace
```

The same local store serves the desktop, browser, and editor workflows. Optional encrypted sync publishes selected portable data to a folder you choose. [Architecture](docs/ARCHITECTURE.md) · [Sync setup](docs/encrypted-sync.md)

## Development

From the repository root:

```powershell
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test
pnpm lint
pnpm build:phase12
```

`pnpm build:phase12` is the current command for building all application components. It does not indicate that every planned feature is complete. See [CONTRIBUTING.md](CONTRIBUTING.md) for focused build commands and the contribution workflow.

| Directory | Contents |
| --- | --- |
| `apps/` | Desktop app, browser extension, native host, VS Code extension, MCP server |
| `crates/context-core/` | Rust storage, migrations, validation, history, search, and tests |
| `packages/` | Shared types, adapter SDK, portable format, encrypted sync, and fixtures |
| `scripts/` | Build, packaging, and native-host registration helpers |
| `docs/` | User guides, integration setup, architecture, and product reference |

## Roadmap and contributing

The next priorities are permission isolation, import and sync integrity, reliable temporary context, live integration testing, and simpler installation. See the [public roadmap](ROADMAP.md).

Contributions are welcome: fixes, accessibility, performance, documentation, and maintained provider adapters all help. Start with [CONTRIBUTING.md](CONTRIBUTING.md). For another AI website, read the [adapter SDK](packages/adapter-sdk/README.md) and include representative fixtures.

## Security and limitations

Known issues affect cross-project MCP permissions, import approval, attachment expiry, and synchronization. Some management controls remain incomplete, and configured remote AI providers are not yet used for smart processing. Review [known limitations](docs/KNOWN_LIMITATIONS.md) and [SECURITY.md](SECURITY.md) before connecting sensitive projects or reporting a vulnerability.

## MadeRocksThink

TF0000 is an open-source project from [MadeRocksThink](https://github.com/MadeRocksThink), building tools that help people carry useful context between AI systems.

Released under the [MIT License](LICENSE).

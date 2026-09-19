# VS Code and GitHub Copilot integration

> This integration is a preview. See [known limitations](KNOWN_LIMITATIONS.md) for live compatibility and context-control boundaries.

The extension uses the same local SQLite database as the TF0000 desktop and browser extension. It does not create a second memory store and does not read Copilot's internal chat transcript.

## Install

Paths using `C:\path\TF0000` are examples; replace them with your repository location.

Build the editor integration:

```powershell
pnpm build:vscode
```

Then install `target/release/tf0000-vscode.vsix` using **Extensions: Install from VSIX…** in VS Code. The packaged extension includes the matching native host. For an unpackaged development session, build the extension and launch VS Code with `--extensionDevelopmentPath=C:\path\TF0000\apps\vscode-extension`.

If the bundled host cannot be found, run **TF0000: Configure Native Host Path** and select `target/release/tf0000-native-host.exe`.

## Workflow

1. Run **TF0000: Map Workspace to Project** once per repository or workspace.
2. Use **TF0000: Save Selection as Context** from an editor selection, or **Save File as Context** from Explorer.
3. Inspect current decisions, code references, and pending agent candidates in the TF0000 sidebar.
4. Use **Open Project Context** or **Search Project Decisions** from the Command Palette.

Saved references include the mapped repository, workspace URI, relative file path, language, line range, exact content hash, and capture time. Re-saving identical content is idempotent.

## GitHub Copilot tools

- `#tf0000ProjectContext`: read confirmed project memory and saved code references.
- `#tf0000Decisions`: search current active project decisions.
- `#tf0000Remember`: create a pending memory candidate.

The first two tools are read-only. The remember tool can only create a pending candidate. A candidate becomes a draft `ai_suggestion` memory only after the user opens the review queue and confirms **Accept as Draft** in a modal dialog. The core rejects acceptance requests without the explicit confirmation flag.

# Contributing to TF0000

Thanks for helping improve TF0000. Useful contributions include bug fixes, browser compatibility, accessibility, performance, clearer documentation, and maintained provider adapters.

## Set up

Use Node.js 24, pnpm 11.19.0, stable Rust with rustfmt and Clippy, and the Windows Tauri prerequisites (Microsoft C++ Build Tools and WebView2).

From your checkout:

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

Keep `pnpm-lock.yaml` and `Cargo.lock` in version control. Do not commit dependencies, build output, personal database files, exports, credentials, or native messaging manifests.

## Make a change

1. Start with an issue describing a concrete problem or proposed behavior. Check [known limitations](docs/KNOWN_LIMITATIONS.md) for existing problem areas.
2. Create a branch with a focused change. Preserve existing data and add a migration when the schema changes; do not rewrite migrations already used by other installations.
3. Add a regression test for a changed behavioral contract. Use synthetic fixtures and disposable stores.
4. Update the relevant user or integration documentation.
5. Run the checks below and include the outcome in your pull request. Describe the trigger, resulting behavior, and any remaining limits.

```powershell
pnpm typecheck
pnpm test
pnpm lint
```

For packaging or integration changes, also run the affected build:

| Command | Output |
| --- | --- |
| `pnpm build:web` | Desktop frontend |
| `pnpm build:extension` | Chrome/Edge unpacked extension |
| `pnpm build:native-host` | Release native messaging executable |
| `pnpm build:vscode` | VS Code extension package and bundled host |
| `pnpm build:mcp` | MCP package and stdio smoke check |
| `pnpm build` | Desktop executable |
| `pnpm build:phase12` | All components above |

## Provider adapters

Read the [adapter contract](docs/adapter-contract/README.md), [SDK](packages/adapter-sdk/README.md), and [compatibility matrix](docs/adapter-compatibility.md). Include a sanitized HTML fixture and tests for capture, selection, insertion, clipboard fallback, and the no-auto-send behavior. Request only the host permissions required by the adapter.

Fixture tests cannot prove compatibility with a live account. State the browser, provider, and workflow you tested manually, and never commit real chat transcripts or credentials.

## Report problems

For ordinary bugs, include the app version, operating system, browser or editor version, reproduction steps, and expected versus actual behavior. Remove private information from screenshots and logs. Follow [SECURITY.md](SECURITY.md) for vulnerabilities.

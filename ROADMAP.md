# TF0000 roadmap

This is the public direction for the project. Items below describe implementation priorities, not release dates or guarantees.

## Available in the preview

- Local Rust/SQLite context storage and a desktop workspace.
- Projects, nested spaces, memories, source evidence, history, and branches.
- Browser adapters for ChatGPT, Claude, and Gemini, with fixture tests.
- Reviewed context handoffs, packs, presets, and per-conversation controls.
- Local search, deterministic assistance, imports, and database backup/restore.
- VS Code/Copilot integration code and a packaged local MCP server.
- Adapter SDK, portable project format, and experimental encrypted folder sync.

See [known limitations](docs/KNOWN_LIMITATIONS.md) for the current boundaries of these features.

## Next priorities

- [ ] Enforce project permissions for every memory included in MCP responses.
- [ ] Bind import confirmation to the exact reviewed file contents.
- [ ] Enforce expiry and make temporary-context behavior consistent across interfaces.
- [ ] Validate portable graphs, immutable ancestry, and record relationships completely.
- [ ] Prevent stale sync metadata overwrites and repeated resolved conflicts.
- [ ] Preserve form drafts after errors and make global selection reliable.
- [ ] Finish missing project/space management and Context Pack history controls.
- [ ] Verify live provider pages, native dialogs, installed editor tools, and two-device sync.
- [ ] Package signed desktop installers and simplify browser setup.

## Later

- [ ] Add provider adapters with committed maintenance and fixture coverage.
- [ ] Implement real optional provider-backed assistance and evaluate retrieval quality.
- [ ] Extend portable coverage toward complete workspace transfer.
- [ ] Publish versioned SDK packages and release compatibility notes.
- [ ] Broaden platform, accessibility, and large-workspace performance testing.

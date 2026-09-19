# Known limitations

TF0000 is an experimental preview. These are known behavior gaps, not a claim that every other workflow has been verified. Use disposable projects when evaluating integrations and keep independent backups of important data.

## Permissions and imports

- A Context Pack allowed through MCP can include a memory from a denied project. Project access rules are not currently a reliable security boundary for this case.
- Import confirmation reads the file again. Changing the file after preview can cause unreviewed contents to be imported.
- The portable TypeScript parser does not reject every malformed relationship, authority value, duplicate ID, or inconsistent content record.

## Temporary context and sync

- `expiresAt` is not consistently enforced when composing context. Remove time-limited attachments manually. Insertion counters work separately and do not count actual model prompts or clipboard copies.
- Portable space and version graphs accept invalid cycles in some cases.
- Metadata-only memory moves may not synchronize. Older project snapshots can overwrite newer metadata.
- Choosing the local value does not reliably prevent the same sync conflict from reopening when a snapshot is replayed.
- Portable sync does not include the complete workspace: raw chats, code references, branches, preferences, integration permissions, and complete Context Pack history are outside its current coverage.
- Authenticated encryption does not fix these merge problems. A portable snapshot is not a full database backup.

## Interface and smart features

- Global context selection may reset when a project exists. Some failed saves clear form drafts.
- Long status paths can overflow narrow windows. The native desktop window has a minimum width of 760 pixels.
- Complete rename/delete/archive/pin controls and full desktop Context Pack editing/history are not available. Use the browser extension to build packs containing memories.
- The desktop memory list can span projects. Browser-created packs are global; include global context deliberately when exporting them.
- Desktop search returns at most 50 results without pagination.
- Local smart features use deterministic extraction and feature hashing. Configuring a remote provider does not currently invoke its model.
- Marking a normal memory conflict resolved records a decision; it does not itself rewrite contradictory memories.

## Verification and distribution

- Browser adapters have saved-fixture coverage; live authenticated websites still need acceptance testing.
- VS Code packages and the MCP server build, but installed editor workflows and individual MCP clients need verification.
- Native Windows dialogs, real two-device sync, recovery during process interruption, and large-scale performance need additional checks.
- Source installation is currently documented for Windows. Signed installers, store distribution, and SDK registry releases are not included.

Follow the [roadmap](../ROADMAP.md) for planned improvements and [security guidance](../SECURITY.md) for sensitive information.

# Threat model and privacy rules

## Protected assets

Raw transcripts, fragments, memories, file references, integration permissions, exports, backups, and any future provider credentials.

## Trust boundaries

- Browser content script to extension service worker.
- Extension to native messaging host.
- Tauri frontend to Rust commands.
- Import/export files to the local core.
- Local MCP clients to read/write candidate APIs.
- Tauri frontend plaintext to the Web Crypto encryption boundary.
- Encrypted sync envelopes to a user-selected folder and its storage provider.

## Required controls

- Local storage by default; no telemetry or outbound model calls in the core.
- Strict schema validation at every IPC and import boundary.
- Least-privilege host permissions and explicit provider allowlists.
- User-triggered capture of visible content only; no hidden scraping.
- Composer insertion never submits a message.
- Explicit outbound preview before context leaves the device.
- Read-only agent access by default; writes become approval candidates.
- Secret-like content warnings before save or attachment.
- OS keychain for future credentials; never store credentials in memories.
- Transactional writes, foreign keys, integrity checks, migration backups, and recoverable exports.
- Logs contain identifiers and error categories, not raw conversation content.
- Sync uses AES-256-GCM authenticated encryption and PBKDF2-SHA-256 with fresh salts and IVs.
- Sync passphrases remain in memory for the current UI session and are never persisted or transmitted.
- Remote version ancestry is validated; concurrent edits require an explicit local/remote resolution.
- Sync is disabled until the user chooses projects, a folder, a device name, and a passphrase.

## Residual risks

- Folder providers can observe encrypted file size, modification time, and the user-chosen device filename.
- A forgotten passphrase cannot be recovered. A weak passphrase reduces resistance to offline guessing.
- The folder transport can delete or replay ciphertext. Authentication detects ciphertext modification, but does not establish snapshot freshness. Known metadata and ancestry defects mean the merge currently cannot guarantee protection against silent overwrite or repeated conflicts; availability remains the provider's responsibility.
- Provider DOM changes can break browser adapters. Local fixtures and the compatibility matrix detect known regressions but cannot predict future page changes.

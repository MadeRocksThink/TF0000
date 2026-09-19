# Optional encrypted device sync

> Experimental: metadata, version-graph validation, and replayed conflict resolution have known defects. Test with disposable projects and keep separate database backups. See [known limitations](KNOWN_LIMITATIONS.md).

TF0000 remains local-only until sync is configured on the **Encrypted sync** screen. The transport is a user-selected folder. It can stay local, sit on removable media, or be synchronized by a service such as OneDrive or Dropbox; TF0000 has no cloud account, server, telemetry, or provider dependency.

## Set up two devices

1. Choose the same shared folder on each device.
2. Give each device a different name.
3. Select only the projects that may leave that device, optionally including global context.
4. Enter the same passphrase of at least 12 characters. TF0000 keeps it only in the current UI session.
5. Publish an encrypted snapshot, then use **Pull other devices** elsewhere.

Each device owns one `.tf0000sync` file. Publishing replaces only that device's ciphertext snapshot. The storage provider sees the device name, file size, and update time, but not project selection, project names, memory text, or Context Packs.

## Cryptography

- AES-256-GCM authenticated encryption.
- PBKDF2-HMAC-SHA-256 with 310,000 iterations and a fresh 128-bit salt.
- Fresh 96-bit IV for every publication.
- The device ID, creation time, format version, and cryptographic parameters are authenticated as additional data. Only enumeration metadata remains in the envelope header; project selection and content are encrypted.
- Wrong passphrases, altered ciphertext, and altered authenticated metadata fail closed before import.

There is no passphrase recovery. Back up important work separately before relying on any sync folder.

## Offline conflicts

The merge implementation aims to use version ancestry as follows, although the known edge cases mean these rules are not reliable for every snapshot:

- Identical versions are ignored.
- A remote descendant fast-forwards locally.
- A local descendant is retained.
- Divergent descendants create an unresolved sync conflict and leave the local memory untouched.

The Encrypted sync screen displays both values. The user can choose **Keep this device** or **Use remote version**. A known issue can reopen a resolved conflict when the same snapshot is replayed. Recheck the resulting records before treating a synchronization as complete.

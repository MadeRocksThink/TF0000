# TF0000 encrypted sync

Optional end-to-end encryption and deterministic offline merge planning. It uses Web Crypto AES-256-GCM with PBKDF2-SHA-256 (310,000 iterations), a fresh 128-bit salt, a fresh 96-bit IV, and authenticated selective-sync metadata. Passphrases are never stored by this package.

The transport is deliberately separate: TF0000 writes ciphertext into a user-chosen folder, which may remain local or be synchronized by any storage provider. Concurrent memory histories are never silently overwritten; divergent version ancestry is reported as a conflict for explicit review.

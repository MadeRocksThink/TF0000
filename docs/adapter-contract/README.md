# Provider adapter contract

Adapters translate provider-specific page structures into the normalized types in `@tf0000/adapter-sdk`. They may capture only user-visible content after a user action. Insertion writes to a composer but never sends a message.

Required operations:

1. Detect the active conversation.
2. Capture visible messages and exact selections.
3. Read and insert into the composer.
4. Return normalized provider, role, timestamps, stable external references when available, and source hashes.
5. Report health without exposing conversation content in logs.

Adapters are replaceable. The canonical database never stores provider-specific DOM structures.

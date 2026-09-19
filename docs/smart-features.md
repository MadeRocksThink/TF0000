# Optional smart features

Phase 9 keeps intelligence optional. In **Off** mode, TF0000 uses the same SQLite FTS5 search and manual workflows from earlier phases. No model, network access, or GPU is required.

## Modes

- **Off**: no semantic index or generated assistance is used.
- **Local CPU**: TF0000 uses its compact `tf0000-mini-embed-v1` feature-hashing model for semantic similarity and deterministic local helpers for summaries, extraction, explanations, and recommendations.
- **User-configured provider**: stores a provider name, HTTPS or loopback endpoint, and model identifier for provider-aware integrations. TF0000 never stores an API key. Phase 9 assistance remains available through the auditable local implementation, and generated artifacts are labelled with the engine that actually produced them.

The canonical source is always SQLite plus the original content and provenance. Embeddings are disposable indexes and can be rebuilt from the Smart Features panel.

## Safety and review

- Generated summaries are separate artifacts and never overwrite raw chats or memories.
- Extracted decisions, requirements, and suggestions enter a pending queue.
- Accepting a candidate creates a `draft` memory with `ai_suggestion` authority.
- Conflict explanations include both current source memories and never select a winner.
- **Ask Me** recommendations require confirmation. **Automatic** marks matching active context as ready for attachment; it does not modify the source.

## Benchmark

The desktop benchmark measures 64-, 128-, and 256-dimensional compact profiles on the current CPU. It reports throughput and estimated index bytes per document. The balanced 128-dimensional profile is the default and does not require a discrete GPU.

# Portable project and Context Pack format

The stable identifier is `tf0000-project/v1`. The provider-neutral JSON document contains:

- export time and source device ID;
- selected projects and their memory-space hierarchy;
- memories with complete immutable version ancestry, provenance references, and space membership;
- Context Packs with ordered items and inclusion modes.

`@tf0000/context-format` exports TypeScript types, a JSON Schema descriptor, a parser, validator, and serializer. The Rust core also validates incoming portable data. Validation is incomplete: some inconsistent records, relationships, and graph cycles can still be accepted. See [known limitations](KNOWN_LIMITATIONS.md) before accepting untrusted documents.

IDs and timestamps are carried in the format. The implementation rejects changes to existing immutable version content and attempts to recognize descendants and concurrent edits. Metadata updates, ancestry validation, and replay behavior still have defects, so importing a snapshot is not guaranteed to be harmless when repeated.

The human-readable Markdown Context Pack formatter remains available, but Markdown is an export representation rather than the lossless interchange format.

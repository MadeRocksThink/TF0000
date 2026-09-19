# TF0000 portable project format

`tf0000-project/v1` is the stable, provider-neutral interchange format for selected projects, memory spaces, immutable memory versions and Context Packs. Use `serializePortableWorkspace`, `parsePortableWorkspace`, and `validatePortableWorkspace`; never accept unvalidated input.

Unknown future format versions fail closed. IDs and version ancestry are preserved so encrypted offline sync can distinguish fast-forward updates from concurrent edits.

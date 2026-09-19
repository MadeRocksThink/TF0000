# TF0000 adapter compatibility

| Provider | Host | Adapter API | Capture | Selection | Composer insert | Clipboard fallback | Fixture smoke test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ChatGPT | `chatgpt.com` | 1.0 | Yes | Yes | Yes | Yes | Automated |
| Claude | `claude.ai` | 1.0 | Yes | Yes | Yes | Yes | Automated |
| Gemini | `gemini.google.com` | 1.0 | Yes | Yes | Yes | Yes | Automated |

Each row is checked against a local HTML fixture through the stable `@tf0000/adapter-sdk` conformance runner. Tests perform no network requests and confirm normalized messages, hashes, health, and the no-auto-send contract covered by provider tests.

New web adapters are added only when usage and maintenance demand justify owning their selectors, fixtures, permissions, and regression surface. Unknown sites continue to use manual capture; Phase 12 intentionally does not claim compatibility without a maintained fixture.

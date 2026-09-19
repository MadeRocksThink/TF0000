# Security

TF0000 is an experimental preview. No version is currently designated production-ready, and there is no promised security response time or supported release window yet.

## Reporting a vulnerability

Do not put credentials, private conversations, database copies, or sensitive exploit details in a public issue.

On the official MadeRocksThink/TF0000 repository, use **Security → Advisories → Report a vulnerability** if private vulnerability reporting is enabled. Include the affected version, a minimal synthetic reproduction, impact, and any suggested fix. If that control is unavailable, open a public issue asking maintainers to establish a private reporting channel, without disclosing the vulnerability details. No separate security email is currently published here.

Repository maintainers should enable GitHub private vulnerability reporting before promoting a public release.

## Current boundaries

- Local SQLite databases, ordinary backups, and JSON/Markdown exports are not encrypted by TF0000. Protect them using your operating system and storage controls.
- Browser insertion puts text on a third-party page, where the provider may process it before you press Send.
- MCP currently has a known project-isolation defect through cross-project Context Packs. Do not rely on these permissions to separate sensitive projects.
- Encrypted sync authenticates its files, but sync merge and replay behavior still have known defects. Keep independent database backups.
- Previewed imports can become stale if the source file changes before confirmation. Temporary time-based expiry is also incomplete.

See [known limitations](docs/KNOWN_LIMITATIONS.md) and the [threat model](docs/threat-model/README.md) for details.

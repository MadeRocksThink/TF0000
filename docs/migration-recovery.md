# TF0000 migration and recovery plan

TF0000 applies every SQLite schema migration inside its own transaction. The application refuses to open a database whose schema is newer than the running build, and it runs `PRAGMA integrity_check` after opening or migrating a store.

## Before upgrading

Create a database backup from **Backup, restore, and health**. Keep that file outside the TF0000 application-data directory.

## Restore behavior

The desktop restore command validates the selected `.db` file in a staging copy, migrates that copy to the current schema, and checks its integrity before touching the active database. It then creates a timestamped `tf0000-pre-restore-*.db` recovery backup of the current store and replaces the active database through SQLite's backup API.

If replacement fails, TF0000 restores the recovery copy automatically. The UI reports the recovery backup path after every successful restore.

## Failed migration or damaged database

1. Do not delete the active database or its backups.
2. Open **Backup, restore, and health** and run diagnostics.
3. Export the privacy-safe diagnostic report. It includes schema, integrity, and row counts but no memory or chat content.
4. Restore the newest known-good `.db` backup through the desktop UI.
5. If the desktop app cannot open, preserve the database path shown in earlier diagnostics and use a previous application build that supports that schema. Never edit `PRAGMA user_version` manually.

Scheduled backups run while the desktop application is open. They are additive and are never automatically deleted.

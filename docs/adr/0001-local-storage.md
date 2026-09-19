# ADR 0001: Versioned Local SQLite Storage

- Status: accepted
- Date: 2026-09-19

## Context

ITGLA needs durable local data before editing, relationships, or sync can be trustworthy. The desktop process is single-user and currently synchronous. Data must survive restart, migrations must be atomic, and later versions need a stable model for tags and typed relationships.

## Decision

Use SQLite through `rusqlite` with its bundled SQLite feature. Store the schema version in `PRAGMA user_version` and run each migration in a transaction. Schema v1 defines projects, assets, tags, asset-to-tag membership, and directed typed relationships. Foreign keys are enabled for every connection.

The application database is `itgla.db` under `ITGLA_DATA_DIR` when set, otherwise the platform user-data directory. Configuration is resolved and validated at startup. Tests use isolated temporary database files or in-memory connections.

On an empty database, insert the deterministic prototype dataset once. Existing data is never silently replaced. Persistence errors remain typed and retain their source.

## Alternatives

- JSON file: simple initially, but atomic updates, relations, uniqueness, and migrations become application code.
- Embedded key-value store: poor fit for relational queries and integrity constraints.
- Cloud database: violates the local-first boundary and creates account/network requirements before product validation.

## Compatibility and rollback

Schema v1 is additive relative to `v0.0.1`, which had no persisted data. Schema v2 adds relationship archival and deterministic example relationships without removing data. Rollback to `v0.0.1` leaves `itgla.db` untouched but unused; `v0.0.2` and `v0.0.3` reject schema v2 rather than opening it unsafely. Back up the database before any future destructive migration. Newer application versions must not lower `user_version` or open an unknown newer schema for writes.

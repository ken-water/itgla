# ADR 0002: Bounded Portability and Recovery

- Status: accepted
- Date: 2026-09-19

## Context

Local-first data needs a user-controlled way to move between devices and recover from accidental replacement without adding an account or cloud dependency. Human-readable exchange and complete disaster recovery have different retention requirements.

## Decision

Use a versioned JSON document for active workspace exchange and SQLite's online backup API for complete same-schema recovery. JSON format version 1 contains projects, resources, tags, and typed relationships with stable identifiers. Imports reject unknown fields, unsupported versions, invalid ownership or references, and inputs above 10 MiB, 1,000 projects, 10,000 assets, or 50,000 relationships.

Parse and validate the entire JSON document before beginning one replacement transaction. Create `pre-import.db` before JSON replacement and `pre-restore.db` before database restore. A database restore is accepted only when it uses the current schema and passes SQLite integrity checking. Paths remain editable and local; no network access or implicit upload occurs.

## Alternatives

- Use JSON as the primary database: rejected because relational constraints, transactions, migrations, and archived records are stronger in SQLite.
- Copy a live database file directly: rejected because SQLite's online backup API provides a consistent snapshot while the connection is open.
- Add a native file-dialog dependency: deferred to avoid another platform integration and dependency surface before the workflow is validated.

## Compatibility and rollback

This release does not change the SQLite schema. JSON format versions are independent from database schema versions and fail closed when unsupported. JSON exchange includes active records only; complete backups retain archived records. Recovery points may be restored by `v0.0.5` or later versions that explicitly support schema v2.

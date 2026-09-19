# Changelog

All notable changes to ITGLA are documented here. Versions follow Semantic Versioning.

## [0.0.5] - 2026-09-19

### Added

- Versioned JSON export and transactional import for active projects, resources, tags, and relationships.
- Full SQLite backup and same-schema restore from editable local paths.
- Automatic `pre-import.db` and `pre-restore.db` recovery points before replacement operations.
- A data-safety panel with explicit confirmation before import or restore.

### Safety

- JSON imports are limited to 10 MiB and validate format version, counts, identifiers, types, tags, project ownership, and relationship references before writing.
- Invalid imports leave the current workspace unchanged; database restores require schema v2 and a successful SQLite integrity check.

### Known limitations

- File paths are entered directly; a native file picker is not bundled in this release.
- JSON exchange contains active records; use a database backup to retain archived records.

## [0.0.4] - 2026-09-19

### Added

- Typed relationships between resources with create and confirmation-gated remove workflows.
- A data-driven project graph whose nodes select the matching resource row.
- A relationship list that shows source, type, and target for the current project.
- Schema v2 migration with relationship archival and deterministic example relationships.

### Changed

- Database migrations now run sequentially in transactions and reject unknown newer schemas.
- The relationship graph is generated from local assets and relationships instead of static UI content.

### Migration

- Upgrades schema v1 to schema v2 without removing projects, resources, tags, or relationships.
- Releases through `v0.0.3` reject schema v2 safely; back up `itgla.db` before rolling back.

## [0.0.3] - 2026-09-19

### Added

- Create, edit, and archive workflows for projects and resources.
- Confirmation before archival and protection against archiving the final project.
- Comma-separated resource tags with validation, deduplication, persistence, and search.
- Dynamic project navigation with live resource and attention counts.
- Success and recoverable validation feedback that preserves editor input.

### Changed

- Project and resource navigation now derive entirely from local database content.
- Resource rows open the editor and show assigned tags alongside their description.

### Known limitations

- Archived records do not yet have a restore screen.
- The relationship graph still uses the prototype layout and is not editable.

## [0.0.2] - 2026-09-19

### Added

- Versioned SQLite schema for projects, resources, tags, and typed relationships.
- Atomic first-run migration and deterministic one-time sample-data import.
- Platform-aware data directory with an `ITGLA_DATA_DIR` override.
- Typed storage failures and an in-app data-read error state.
- Persistence tests covering migration, reopen behavior, and newer-schema rejection.

### Changed

- Resource lists now load from the local database instead of compiled Rust fixtures.

### Migration

- Creates schema version 1 and imports sample content only when the project table is empty.
- Rolling back to `v0.0.1` leaves `itgla.db` untouched and unused.

## [0.0.1] - 2026-09-19

### Added

- Initial Slint desktop application shell.
- Project-centered resource inventory for websites, domains, certificates, servers, and services.
- Project switching, resource filtering, search, health states, and a relationship overview.
- Typed Rust domain fixtures with focused filtering tests.
- Documented Rust architecture, dependency choices, verification gates, and rollback boundary.

### Known limitations

- Data is compiled sample content and is not persisted.
- Add-resource and global navigation actions are not implemented.
- Relationship layout is a deterministic project overview rather than a user-editable graph.

[0.0.1]: https://github.com/ken-water/itgla/releases/tag/v0.0.1
[0.0.2]: https://github.com/ken-water/itgla/releases/tag/v0.0.2
[0.0.3]: https://github.com/ken-water/itgla/releases/tag/v0.0.3
[0.0.4]: https://github.com/ken-water/itgla/releases/tag/v0.0.4
[0.0.5]: https://github.com/ken-water/itgla/releases/tag/v0.0.5

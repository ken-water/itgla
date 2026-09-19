# Changelog

All notable changes to ITGLA are documented here. Versions follow Semantic Versioning.

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

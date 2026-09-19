# ITGLA

ITGLA is a local-first desktop workspace for understanding how projects, websites, domains, certificates, servers, and services relate to one another.

Current version: `0.1.0`

## Run

Requirements: Rust stable and the native libraries required by Slint.

```bash
cargo run
```

The first build downloads and compiles Slint, so it takes longer than subsequent runs.

## Current capabilities

- Project-centered resource inventory
- Search and resource-type filtering
- Cross-project search and a consolidated attention view
- Health and expiry context
- Project relationship graph
- Deterministic local sample data
- Versioned SQLite persistence under the platform user-data directory
- Project and resource creation, editing, archival, and tags
- Typed relationship creation and confirmation-gated removal
- Data-driven interactive project graph and relationship list
- Versioned JSON import/export and complete SQLite backup/restore

Open **数据安全** in the lower-left sidebar to use the default local export and backup paths or enter another path. Import and restore replace the current workspace only after confirmation and create an automatic recovery point first.

All data remains local. Cloud sync, authentication, billing, monitoring integrations, native file pickers, and destructive infrastructure actions are intentionally outside `0.1.0`.

Rust architecture, dependency decisions, verification gates, and rollback behavior are documented in [docs/engineering.md](docs/engineering.md).

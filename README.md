# ITGLA

ITGLA is a local-first desktop workspace for understanding how projects, websites, domains, certificates, servers, and services relate to one another.

Current version: `0.0.2`

## Run

Requirements: Rust stable and the native libraries required by Slint.

```bash
cargo run
```

The first build downloads and compiles Slint, so it takes longer than subsequent runs.

## Current prototype

- Project-centered resource inventory
- Search and resource-type filtering
- Health and expiry context
- Project relationship graph
- Deterministic local sample data
- Versioned SQLite persistence under the platform user-data directory

Resource editing, cloud sync, authentication, billing, monitoring integrations, and destructive resource actions are intentionally outside this version.

Rust architecture, dependency decisions, verification gates, and rollback behavior are documented in [docs/engineering.md](docs/engineering.md).

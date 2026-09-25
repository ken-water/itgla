# ITGLA

ITGLA is a local-first desktop inventory for keeping server details easy to find and copy.

Current version: `0.2.2`

## Run

Requirements: Rust stable and the native libraries required by Slint.

```bash
cargo run
```

The first build downloads and compiles Slint, so it takes longer than subsequent runs.

## Packages

Releases include a Windows x86_64 MSI installer and portable ZIP, Linux x86_64 DEB, RPM, and AppImage packages, and a macOS arm64 DMG. Windows and macOS artifacts are currently unsigned. Every artifact has a matching SHA-256 file and the release includes a combined `SHA256SUMS` manifest.

Application data remains in the current user's local data directory.

From Linux, install the Rust Windows GNU target and MinGW toolchain, then run:

```bash
rustup target add x86_64-pc-windows-gnu
./scripts/package-windows.sh
```

The script writes the versioned archive and SHA-256 checksum to `dist/`. See [docs/windows-package.md](docs/windows-package.md) for prerequisites and verification details.

## Current capabilities

- Dense server table with tags, IP addresses, ports, and custom columns
- Click any cell to copy its value, with a compact confirmation notice
- Tag filters and search across every visible field
- Sortable Tags, IP, Ports, and custom-column headers
- Excel, OpenDocument, CSV, and TSV import with per-column mapping
- Transactional imports with bounded files, rows, columns, and values
- Versioned SQLite persistence under the platform user-data directory
- Server creation, editing, reversible hiding, and restoration

All server data remains local. Cloud sync, authentication, billing, monitoring integrations, and destructive infrastructure actions are not enabled.

Rust architecture, dependency decisions, verification gates, and rollback behavior are documented in [docs/engineering.md](docs/engineering.md).

The product and revenue path toward the first ten paid users is documented in [docs/roadmap-to-10-paid-users-2027-06.md](docs/roadmap-to-10-paid-users-2027-06.md).

# ITGLA

ITGLA is a local-first desktop inventory for keeping server details easy to find and copy.

Current version: `0.6.4`

## Run

Requirements: Rust stable and the native libraries required by Slint.

```bash
cargo run
```

The first build downloads and compiles Slint, so it takes longer than subsequent runs.

## Packages

Releases include a Windows x86_64 EXE installer and portable ZIP, Linux x86_64 DEB, RPM, and AppImage packages, and macOS ARM64 and Intel DMGs. Windows and Linux packages are built locally; only the two DMGs use GitHub Actions. Windows and macOS artifacts are currently unsigned. Every artifact has a matching SHA-256 file and the release includes a combined lowercase `sha256sums` manifest. Official downloads are available at [itgla.com](https://itgla.com/downloads.html).

Linux builds require glibc 2.35 or newer, such as Ubuntu 22.04+. macOS packages require macOS 14 or newer.

Application data remains in the current user's local data directory.

From Linux, install the Rust Windows GNU target and MinGW toolchain, then run:

```bash
rustup target add x86_64-pc-windows-gnu
./scripts/package-windows.sh
```

The script writes the versioned archive and SHA-256 checksum to `dist/`. See [docs/windows-package.md](docs/windows-package.md) for prerequisites and verification details.

## Current capabilities

- Server and service tables with tags, IP addresses, per-service ports, and custom columns
- Copy key fields, whole rows, or whole columns with explicit copy actions
- Tag filters and search across every visible field
- Sortable inventory columns
- Excel, OpenDocument, CSV, and TSV import with per-column mapping
- Transactional imports with bounded files, rows, columns, and values
- Versioned SQLite persistence under the platform user-data directory
- Server creation, editing, reversible hiding, and restoration
- Businesses, domains, services, and a filterable relationship map
- Overview, global search, recorded-date reminders, backup recovery, and metadata handover
- URL-based discovery with progress, cancellation, evidence review, and confirmation before import

The current desktop is free and has no asset-count limit. Paid options may be considered in the future; no paid plan is available today.

All server data remains local. Cloud sync, authentication, billing, monitoring integrations, and destructive infrastructure actions are not enabled.

Rust architecture, dependency decisions, verification gates, and rollback behavior are documented in [docs/engineering.md](docs/engineering.md).

The product and revenue path toward the first ten paid users is documented in [docs/roadmap-to-10-paid-users-2027-06.md](docs/roadmap-to-10-paid-users-2027-06.md).

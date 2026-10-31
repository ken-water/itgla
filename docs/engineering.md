# Rust Engineering

## Boundary

ITGLA is currently a single desktop binary. `src/domain.rs` owns typed resource data and filtering rules, `src/storage.rs` owns SQLite migrations and persistence, `src/main.rs` coordinates the repository and translates domain values into the Slint model, and `ui/app.slint` owns presentation and transient interaction state. A multi-crate workspace would add ceremony without creating a meaningful runtime or ownership boundary at this stage.

The application has no network service or background discovery. Service and domain metadata is local SQLite data; optional service usernames/passwords use Windows CurrentUser DPAPI and are never included in list/search/summary fields. `src/operational.rs` owns this bounded inventory and its UI bridge; `src/secrets.rs` isolates the native credential protection boundary. SQLite persistence, search, and portability operations are synchronous on the UI thread while the dataset is local and bounded; global search returns at most 100 resources, and JSON imports are capped at 10 MiB, 1,000 projects, 10,000 assets, and 50,000 relationships. Move expensive work off the UI thread before raising those limits. Relevant failure modes are migration or database access failure, unavailable window backends or fonts, invalid import files, and incorrect filtering. Errors remain typed through the storage boundary and are shown in the UI where recovery is possible.

## Dependencies

| Dependency | Purpose | Alternative considered | Operational considerations |
|---|---|---|---|
| `slint` 1.18 | Native declarative desktop UI and Rust bridge | egui and a webview stack; both diverge from the requested Slint implementation | GPL-3.0-or-later OR commercial license. Runtime features are limited to winit, software rendering, system fonts, accessibility, and the 1.18 compatibility level. Native windowing and font libraries are required. Slint is actively maintained; review advisories and licensing before distribution. |
| `slint-build` 1.18 | Compile `ui/app.slint` into the Rust bridge | Runtime interpretation; rejected because compile-time validation is simpler and deterministic | Build-only dependency with the same licensing and maintenance review as Slint. |
| `rusqlite` 0.37 | Versioned relational local storage | JSON and embedded key-value storage; rejected due to weaker integrity and migration ergonomics | MIT license. Uses bundled SQLite for reproducible desktop builds, increasing compile time and binary size while avoiding a runtime system-SQLite dependency. Actively maintained; malformed databases and SQL inputs remain error boundaries. |
| `serde` 1 and `serde_json` 1 | Versioned, human-readable workspace exchange | Ad hoc JSON generation; rejected because strict typed parsing and unknown-field rejection reduce recovery risk | MIT OR Apache-2.0 licenses. Pure Rust with no native runtime dependency. Import bytes and entity counts are bounded before database replacement. |
| `thiserror` 2 | Typed storage errors with causal sources | Manual `Display`/`Error` implementations | MIT OR Apache-2.0 license. Proc-macro build cost only; no native runtime dependency. Actively maintained and widely used. |

`Cargo.lock` is retained because this repository produces an executable. New dependencies require an entry here covering purpose, alternatives, license, native requirements, maintenance, and security impact.

## Verification

Required local gates:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release --locked
```

The UI must also launch with the software renderer and pass a screenshot inspection at 1440x900. For `v0.1.0`, real X11 pointer interaction covers global resources, attention filtering, result navigation, data export/backup, empty-project rendering, and recoverable import failure. Coverage and dependency-policy tooling are not installed in the current environment; a later CI baseline should add `cargo llvm-cov` and `cargo deny` gates rather than claiming those checks here.

## Rollback

Migrations are applied sequentially in transactions. Schema v2 adds relationship archival to schema v1. JSON import validates the complete document before a transactional replacement and creates `pre-import.db`; database restore validates schema and integrity and creates `pre-restore.db`. Rolling back to a release that supports only an older schema is fail-closed: the older application leaves the database untouched and reports the unsupported version. Before any future destructive migration, copy the database and verify restore against the target version. The application never guesses compatibility with a newer schema.

## Service and domain inventory (schema 16)

This first iteration records one service deployment per row, with version, ports, address, tags, notes and existing hosting servers. Domain/hostname rows record parent domain text, registrar, registration/expiry dates, CDN/DNS address and multiple origin servers. Four directed service relations cover dependency, replication, failover and balancing. These are manual records, not verified live service or DNS state.

No dependencies were added. Windows credential protection uses Crypt32/Kernel32 directly. Credentials are masked by default, explicitly unlocked for 30 seconds, and cleared when selecting another record or leaving the inventory. Copy clears the clipboard after 30 seconds only if unchanged; OS clipboard history can retain copies. Metadata/notes are unencrypted. Non-Windows builds disable credential entry and fail closed for credential operations. A different Windows account/device may require re-entering credentials even after a complete database restore.

Migration 16 creates a pre-migration SQLite snapshot at `backups/pre-schema-16.db`, then adds inventory, hosting links and relations in one transaction. Migration 17 snapshots schema 16 to `backups/pre-schema-17.db` before adding verification dates. Existing server IDs remain authoritative. Inventory verification expires after 90 days and is cleared when metadata or hosting links change. CSV exports follow the current type, text search, selected-server filter and active/hidden view, never contain credentials, and escape spreadsheet formula prefixes. Use full database backups in Settings: the legacy project JSON exchange does not include operational inventory. Older application versions reject newer schemas; rollback requires the matching older binary plus its pre-migration snapshot and loses later edits. Never overwrite the current database without separately preserving it. See ADR 0003 for the accepted boundary.

## Phase-one responsibility context (local preview, schema 21)

`asset_context` stores optional technical and backup contacts, environment, documentation availability/HTTP(S) link, source, and manual verification on each live business/server/service/domain ID. Servers retain their existing environment field. There is no automatic inheritance, discovery or contact-account provisioning. Blank means Not recorded; Unknown and Not applicable remain explicit text, while documentation uses a four-state selector. Empty fields are valid. Save/check is atomic with the asset; hidden records retain context and hard deletion cleans it up.

Migration 21 first saves `backups/pre-schema-21.db`. Older binaries reject schema 21; preserve the current workspace separately before restoring the matching schema-20 snapshot. Full SQLite backup/restore includes context and relationships. Service/domain CSV includes context with spreadsheet-formula escaping and supports the same contact/source search as its table. Server CSV and legacy project JSON remain partial exports; use a full database backup for recovery. Manual checks expire after 90 days and become invalid after metadata/context/hosting edits; unchanged saves preserve a previous check. Checks record a local text reviewer, not authenticated approval or monitoring. Audit history stores events without copying contact details or credentials.

Editors reuse a scrollable shared component and fixed save action. Demo contacts are clearly fictional. Tests cover atomic rollback, no-op checks, stale checks, hidden records, documentation states, hosted resources, file reopen, backup recovery, filtered CSV and draft isolation. Timed real-user acceptance targets from the product direction remain unmeasured. See ADR 0005.

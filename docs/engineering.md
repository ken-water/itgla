# Rust Engineering

## Boundary

ITGLA is currently a single desktop binary. `src/domain.rs` owns typed resource data and filtering rules, `src/storage.rs` owns SQLite migrations and persistence, `src/main.rs` coordinates the repository and translates domain values into the Slint model, and `ui/app.slint` owns presentation and transient interaction state. A multi-crate workspace would add ceremony without creating a meaningful runtime or ownership boundary at this stage.

The application has no network, background work, concurrency, secrets, or production data. SQLite persistence, search, and portability operations are synchronous on the UI thread while the dataset is local and bounded; global search returns at most 100 resources, and JSON imports are capped at 10 MiB, 1,000 projects, 10,000 assets, and 50,000 relationships. Move expensive work off the UI thread before raising those limits. Relevant failure modes are migration or database access failure, unavailable window backends or fonts, invalid import files, and incorrect filtering. Errors remain typed through the storage boundary and are shown in the UI where recovery is possible.

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

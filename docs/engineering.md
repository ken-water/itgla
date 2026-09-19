# Rust Engineering

## Boundary

ITGLA is currently a single desktop binary. `src/domain.rs` owns typed resource data and filtering rules, `src/main.rs` translates domain values into the Slint model, and `ui/app.slint` owns presentation and transient interaction state. A multi-crate workspace would add ceremony without creating a meaningful runtime or ownership boundary at this stage.

The application has no network, filesystem persistence, background work, concurrency, secrets, or production data. Its relevant failure modes are UI compilation failure, unavailable window backends or fonts, and incorrect filtering. Build errors propagate through the build script; runtime platform errors propagate from `main`.

## Dependencies

| Dependency | Purpose | Alternative considered | Operational considerations |
|---|---|---|---|
| `slint` 1.18 | Native declarative desktop UI and Rust bridge | egui and a webview stack; both diverge from the requested Slint implementation | GPL-3.0-or-later OR commercial license. Runtime features are limited to winit, software rendering, system fonts, accessibility, and the 1.18 compatibility level. Native windowing and font libraries are required. Slint is actively maintained; review advisories and licensing before distribution. |
| `slint-build` 1.18 | Compile `ui/app.slint` into the Rust bridge | Runtime interpretation; rejected because compile-time validation is simpler and deterministic | Build-only dependency with the same licensing and maintenance review as Slint. |

`Cargo.lock` is retained because this repository produces an executable. New dependencies require an entry here covering purpose, alternatives, license, native requirements, maintenance, and security impact.

## Verification

Required local gates:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release --locked
```

The UI must also launch with the software renderer and pass a screenshot inspection at 1440x900. Coverage and dependency-policy tooling are not installed in the current environment; introducing CI or a release candidate should add `cargo llvm-cov` and `cargo deny` gates rather than claiming those checks here.

## Rollback

This MVP has no persistent or external state. Rollback consists of restoring the prior source and `Cargo.lock`, rebuilding, and launching the previous binary. Persistence or sync work must define compatibility and recovery before it is introduced.

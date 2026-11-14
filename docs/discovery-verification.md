# Discovery: local verification

Date: 2026-11-14 (Asia/Shanghai).

## Delivery scope

Public URL discovery and a basic review inbox are implemented locally. Stages 3–6 and the remaining Stage 2 enrichment are planned, not implemented. See [the roadmap](discovery-roadmap.md).

The entry is **Overview > Discover from URL**. Results contain evidence, confidence, observation time and review state. Confirm a domain to add it to Domains; confirm the original web page to add it to Sites. Confirming a DNS address records an evidence decision only. It does not create a server or a deployment relationship.

## Environment and commands

The default local Rust 1.89 toolchain does not support Slint 1.18.1. The installed Rust 1.92 toolchain was used instead. The default Tsinghua registry waited on an index update; the existing local `output/cargo-direct.toml` configuration and cached dependencies allowed offline verification without changing the global Cargo configuration or dependency lockfile. Pass `--config` after the Cargo subcommand so Clippy's nested Cargo invocation receives it.

```powershell
cargo +1.92.0 fmt --all -- --check
cargo +1.92.0 clippy --config output/cargo-direct.toml --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo +1.92.0 test --config output/cargo-direct.toml --workspace --all-features --locked --offline
cargo +1.92.0 build --config output/cargo-direct.toml --release --locked --offline
node scripts/check-english-ui.mjs
python scripts/audit-ui.py
git diff --check
```

## Evidence

- First complete run: 133 tests passed.
- Follow-up run including hidden-asset preservation: 134 tests passed, zero failures.
- Final Clippy check: passed with `-D warnings`. Slint's build script still emits existing unused-component notices for `RecordActions` and `TagChoice`; these are not Rust Clippy errors.
- Static UI checks: 22 contracts, five modal bounds and six contrast pairs passed. The table-header contract was updated for the copy-button area reserved by the current implementation.
- English UI check: passed after correcting two corrupted strings.
- Live HTTP/DNS smoke check used the actual discovery module against `https://itgla.com`, without opening or writing the user's database. Four public endpoints responded: the home page, `robots.txt`, `sitemap.xml`, and `llms.txt`. The missing optional `security.txt` generated a warning rather than failing the run. Local output: `output/discovery-live.txt`.
- Final test/build logs: `output/discovery-tests-final.log`, `output/discovery-clippy-final.log`, `output/discovery-build-final.log`. Final build completion is recorded below once available.

## Completed Windows preview

The locked offline release build completed successfully in 10m 50s on 2026-11-14 at 02:02:09 Asia/Shanghai. The preview is dist/discovery-preview-20261114/ITGLA.exe (36,772,352 bytes). SHA256: f0842d7a7759e1987042564eddd809550809ccc517d904a328369ad71200734f.

Use the adjacent Start-Preview.cmd to select its isolated workspace directory. Starting ITGLA.exe directly uses the normal application data location. The version remains 0.6.3; the dated directory and hash distinguish this preview from the published build. build.json records the exact binary metadata.

The acceptance preparation script verified this binary hash and prepared an isolated workspace. It did not launch the GUI or imply native acceptance.

The governance check reported blocked: dependency advisory tooling is absent; its filesystem policy rejects the UI/migration paths, and the whole-working-tree diff includes pre-existing website/account changes outside this task. These are recorded blockers, not passed gates. No production deployment or policy bypass occurred. Evidence: output/discovery-governance-evidence.json.

## Regression coverage

### Discovery layout and progress follow-up (2026-11-14)

- The heading and guidance span the discovery section. The URL field and primary action share a 40px-high row; the action reserves at least 180px and scales with the selected font size.
- Scanning disables the URL field and duplicate starts. The progress bar and current-step text follow actual DNS/page/evidence stages. It is stage progress, not an elapsed-time estimate. Saving owns the final 97-to-100 transition; a save failure never marks 100.
- All 136 tests passed, including optional-page failures, monotonic progress, rejected private-target behavior, and same-row input/button bounds at all three font sizes and 1000/1280 window widths.
- English UI and static UI checks passed. Live discovery against https://itgla.com emitted ordered progress for all five page checks and returned four public endpoints. Evidence: output/discovery-progress-tests.log and output/discovery-progress-live.log.
- The live check exercises the discovery module, not a native end-to-end GUI scan. Existing dependency-audit and native-acceptance limits still apply.

- Reject secret-bearing URLs, literal IPs, nondefault ports and reserved/private address ranges.
- Extract domain names without case sensitivity, limit extracted names and avoid out-of-scope names.
- Always save observations pending, irrespective of supplied review state.
- Confirm repeatedly and across runs without duplicate assets.
- Preserve existing hidden domains/sites, names, purposes, tags and notes.
- Roll back a partially written report on invalid evidence.
- Inject a database failure after asset insertion and before review-history insertion; both promotion and review roll back together, and a retry succeeds.
- Preserve observations and decisions across reopening, and upgrade a schema-23 database to schema 24.
- Exercise the collapsed discovery entry, evidence review and confirmation callback through Slint's testing backend at all three font sizes.
- Existing regression tests cover row/column copying, sort/resize interaction, independent table fitting and synchronized headers during horizontal scrolling.

## Limits

The smoke check proves this website worked from this machine at this time. It does not establish completeness of subdomain enumeration, origin identification, internal discovery, all proxy configurations, or Windows native visual/installer acceptance. No new dependency was introduced. Fresh dependency-advisory and coverage checks were not run: `cargo-deny`, `cargo-audit` and `cargo-llvm-cov` are not installed locally. No installer, Linux/macOS package, commit, push or production deployment is implied by these local checks.

Discovery currently relies on the system `curl` executable and system DNS. It intentionally disables inherited HTTP proxies while pinning a validated address, so an environment that requires a proxy needs a separately designed discovery transport; blindly enabling a proxy would remove the current DNS-pinning guarantee.

### Follow-up build and launch

The follow-up optimized build completed successfully in 9m 30s. All 136 tests, Clippy with warnings denied, formatting, English UI, static UI checks and diff whitespace checks passed. Binary: dist/discovery-preview-20261114-r2/ITGLA.exe (36,836,864 bytes). SHA256: 6d1c70271f9e19cc714315c9f8eb6f0d35d5d937fa909f30972c7e914cca85d6.

The r2 launcher uses the previous isolated discovery-preview-20261114/workspace directory. The old preview window was closed gracefully; its remaining background process did not exit, so only that verified windowless preview process was stopped before launching r2. Process-path verification confirmed the new r2 executable running. Startup does not establish native visual or full GUI workflow acceptance. This preview does not include an installer or publication.

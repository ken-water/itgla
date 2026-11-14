# Stage 2 review workflow verification

Date: 2026-11-14, Asia/Shanghai.

## Automated checks

- Rust 1.92, locked offline dependencies, unchanged Cargo.lock.
- Complete workspace tests: 142 passed, zero failures. Log: output/discovery-stage2-tests.log.
- English UI and static UI contracts passed (22 contracts, five modal bounds, six contrast pairs).
- Initial regression run exposed non-idempotent schema-25 creation in seven historical migration fixtures. The migration now uses idempotent table/index creation. The complete rerun passed.
- New coverage includes 225 observations over multiple pages, old observations beyond the former 200-result limit, run/state/type/search filtering, page clamping, per-run warning isolation, normalized URL identities, hidden-asset preservation, invalid association rejection and failure after an asset insert rolling back confirmation and link together.
- Slint tests cover discovery shortcuts from sample mode, primary action order at three font sizes, existing-asset confirmation calling the link callback, input/button bounds and progress behavior.

## Native visual review

Four fictional, isolated debug software-renderer snapshots were generated and inspected: guide, largest-font dark guide, Overview and largest-font dark discovery. The guide presents two equal primary choices above manual entry/import. No guide card overlap was observed. Discovery exposes the run selector, filters and scrollable inbox. Snapshot dimensions reflect the current Windows DPI/minimum logical window size; they are recorded in test-results/discovery-stage2-native/manifest.json. These captures are view fixtures, not real discovery outcomes, and do not establish physical-display sharpness or native end-to-end scan acceptance.

## Remaining verification boundaries

This increment improves review and onboarding. It does not add DNS record-type/TTL enrichment, TLS SAN collection, certificate transparency or recursive discovery. It does not prove all public websites, native physical DPI, full installer behavior or a production release.

Dependency advisory and coverage tools remain unavailable locally. Governance filesystem-policy restrictions and pre-existing unrelated website changes remain recorded limitations; no release gate bypass is implied.

## Final local build and data preservation

- Complete tests: 142 passed; Clippy with warnings denied, formatting, English UI, static UI and whitespace checks passed.
- Optimized locked/offline release build completed in 11m 05s. Log: output/discovery-stage2-build.log.
- Local binary: dist/discovery-stage2-preview-20261114/ITGLA.exe, 37,214,720 bytes. SHA256: 020e199b729cb9df4ef7940217d9e8575dd241870aa816bf946d1288f5866890. Crate version remains 0.6.3; the dated directory and hash identify this preview.
- Started the exact new binary (PID 38600) after closing the old r2 preview. The launcher reuses the original isolated preview workspace, not the normal user database.
- The database upgraded from schema 24 to 25 and created backups/pre-schema-25.db. Read-only before/after fingerprints matched for 13 existing inventory, relationship, context and discovery tables. Evidence: output/discovery-stage2-preview-before.json and output/discovery-stage2-preview-after.json. New evidence-link records are separate from those retained tables.
- No installer, commit, push or production publication was performed.

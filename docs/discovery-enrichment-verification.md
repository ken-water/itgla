# Stage 2 evidence enrichment verification

Date: 2026-11-14. Local development only; no publication.

## Automated acceptance

Rust 1.92.0 with locked offline dependencies. Full workspace/all-features test run: **161 passed, zero failed, one explicitly ignored live test**, in 33.77 seconds. The ignored test was executed separately against the owned public site and passed. Logs: `output/discovery-enrichment-tests-final.log`, `output/discovery-enrichment-live.log`.

New deterministic coverage includes public/private suffix scopes, relative/Unicode links and secret-bearing URL rejection, selected HTTP header retention and sensitive-header omission, bounded HTTP payloads, canonical/external reference classification, XML namespaces/entities/CDATA/DTD/truncation/depth/count boundaries, sitemap loops/deduplication/request/deadline limits, public/private/mixed address validation before transport, partial failures, fixed public certificate parsing/dates/SANs, malformed and oversized certificates, wildcard/non-first-party SAN handling, default-off/scoped/bounded CT, DNS transaction/question validation, UDP TTL/resolver preservation, unrelated-answer exclusion and TCP fallback. A subprocess watchdog test kills a stalled collector. Storage tests confirm every new evidence-only kind without creating assets or relationships. Native headless interaction tests cover the opt-in control, conditional proxy field, busy-state consent lock and three font sizes. Readable evidence rendering preserves the original stored JSON.

The first run exposed DNS question comparisons treating wire-format FQDN markers as identity differences; comparison now normalizes a terminal dot while still matching name, type, class, transaction, opcode and response kind. A wildcard fixture caught repeated-prefix stripping accepting a multi-wildcard SAN; parsing now strips exactly one prefix and validates the remainder. A certificate fixture expectation was corrected against Python SSL decoding of the fixed public certificate.

Static UI contracts: 22 contracts, five modal bounds, six contrast pairs and desktop target sizes passed. English UI gate passed. Formatting/Clippy/build results are recorded below when complete. Existing unused Slint component build warnings are distinct from Rust Clippy diagnostics.

## Live acceptance

Executed the test binary with CT explicitly opted in and `http://127.0.0.1:5670` only for the fixed crt.sh provider. Ordinary website requests remained direct, address-pinned and certificate-verified. No production or user database was opened by this test.

Owned URL `https://itgla.com/` completed in 25.18 seconds with **27 observations**: one domain, two address observations, four responding public endpoints, one scope, five HTTP evidence rows, one canonical link, seven subdomain/source observations, one TLS certificate and five DNS records. Four observations came from CT. Observations of the same hostname from independent sources remain separate evidence, not seven unique hosts. Progress was monotonic from 0 to 95; the test deliberately excludes the application-owned persistence stage.

The only warning was HTTP 404 at the optional `/.well-known/security.txt`. No sitemap recursion occurred on this live fixture; recursion behavior is established by injected bounded-transport tests, not claimed from that run. Raw evidence and a compact source/count summary are retained in `output/discovery-enrichment-live.log` and `output/discovery-enrichment-live-summary.json`.

## Dependency/advisory boundary

Six direct dependencies were declared; three were already locked transitively. Cargo.lock gained 29 packages and removed none. Optional ring appears in the lock but `cargo tree -i ring` has no active dependency on the Windows target. The new parser stack requires no new native runtime tool.

A supplemental OSV query through proxy 5670 checked all **594 registry package versions** in Cargo.lock and returned all 594 results. None of the 29 newly locked packages was flagged by that query. Existing dependencies have advisories: bincode, libsqlite3-sys, pin-utils, pulldown-cmark, rustix, ttf-parser, zbus_names, zvariant and zvariant_derive. Several are maintenance notices; SQLite malformed DB/WAL handling, rustix buffer initialization and enabled-wikilink parsing require separate applicability review and fixes before claiming production readiness. The raw response with advisory details is `output/discovery-enrichment-osv.json`.

OSV is supplemental and does not replace cargo-audit, cargo-deny, license review or coverage. Those local tools remain absent. The skill governance checker must continue to report the incomplete dependency gate, its restrictive filesystem policy and unrelated pre-existing dirty files honestly. No successful release gate is claimed.

## Remaining scope

This increment does not implement field-level conflict overwrites, active port scans, SSH/WinRM collectors, internal discovery, cloud APIs, automatic origin detection or guaranteed complete domain enumeration. Schema stays at 25. Existing fields and evidence history are preserved. See `discovery-enrichment.md` for network limits, privacy, compatibility and rollback.

## Native UI and static checks

Five isolated fictional view fixtures were captured from the newly built debug executable in `test-results/discovery-enrichment-native/`: guide, guide at largest-font/dark appearance, Overview, discovery at largest-font/dark appearance, and discovery with CT/proxy disclosure enabled. The two discovery captures were visually inspected: input/action remain separated, the optional provider disclosure and proxy field do not overlap, and the inbox remains in the outer scroll area. These software-renderer images do not prove physical DPI or a user-driven end-to-end scan.

Recorded subprocess exit codes are all zero for format, Clippy with warnings denied, static UI, English UI and Git whitespace checks. See `output/discovery-enrichment-checks.json`. This recorder avoids PowerShell's stderr-as-error wrapper causing misleading tool exit statuses on successful Cargo commands. The actual Cargo logs contain completion and no Rust diagnostic errors.

## Final local preview

Optimized locked/offline release build completed in 13m 08s; an immediate recorded subprocess verification returned exit code 0. Logs: `output/discovery-enrichment-release.log` and `output/discovery-enrichment-release-verified.log`.

The exact portable binary is `dist/discovery-enrichment-preview-20261114/ITGLA.exe`, 38,846,464 bytes (37.05 MiB), SHA256 `ac0bc2dbe6032cad23b97dc4e506d575b955d8b3b0bd9cadeac7e67bfa0e835c`. Crate version remains 0.6.3; the dated folder and build.json identify this local revision. Run `Start-Preview.cmd` to reopen the same isolated preview workspace.

Started this exact binary as PID 13064, with a nonzero main window handle and Responding=True. It uses only `dist/discovery-preview-20261114/workspace/itgla.db`. Schema remains 25; fingerprints of all 37 tables match the read-only baseline after startup. See `output/discovery-enrichment-preview-before.json` and `output/discovery-enrichment-preview-after.json`.

Governance evidence records successful format/check/Clippy/test/contract/compatibility/performance-fixture/locked-build gates. The overall skill checker remains blocked by the dependency audit gate and generic filesystem restrictions on UI/migrations and pre-existing unrelated website changes. These restrictions are not a claim that the user withheld authorization for this local implementation. No commit, push, installer packaging, GitHub workflow or production deployment was performed.

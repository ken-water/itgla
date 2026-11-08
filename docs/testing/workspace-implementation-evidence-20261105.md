# Native workspace implementation evidence

Date: 2026-11-05. Base revision: e6c5808 plus the local working tree. Candidate remains version 0.6.2; no release/version bump or production migration is part of this task.

## Delivery scope

The six planned stages have code in the native Rust/Slint application. This report separates implementation from acceptance. The browser prototype at port 4200 is design evidence only; it does not exercise SQLite or the desktop executable.

| Stage | Implemented behavior | Main code |
| --- | --- | --- |
| S1 | Public-field global search, keyboard navigation, typed identities, read-only details, connected-resource traversal and Back | src/workspace.rs; ui/app.slint; ui/workspace.slint |
| S2 | Cross-type directory, independent saved column widths/Fit, business-aware overview, empty states and recent checks | src/workspace.rs; ui/workspace.slint; ui/data-table.slint |
| S3 | Existing validated server/service/domain/business editors, scoped server import and existing backup workflows | src/workspace.rs; src/main.rs |
| S4 | Map focus, one-level expansion, map-only hide/restore, real host context and shared details | src/topology.rs; ui/topology.slint |
| S5 | Domain expiry and dated active Work, review/postpone/reopen, independent expiry cycles, recorded actor/history, schema22 migration and backup | src/reminders.rs; migrations/022_reminder_reviews.sql; src/storage.rs |
| S6 | Overview/Assets/Asset map/Reminders/Settings, retained legacy shortcuts/page IDs, three discrete text-size buttons and existing five themes | ui/app.slint; ui/appearance.slint; src/main.rs |

Search never decrypts/indexes credentials or custom secret columns. The directory displays up to 1,000 matching rows, global search up to 50, with explicit refine messages. Existing inventory creation remains capped at 5,000 records. The 10,000-summary benchmark tests algorithm headroom, not permission to create more inventory than the existing product limit. Map rendering retains its 200-node safety bound.

## Automated verification

The initial implementation passed 119 Rust tests and Clippy. After the spreadsheet security upgrade and the overview/archive-filter regression check, the full suite passed 120/120 (6.84s test execution). Final visual corrections are included in that run. Log: output/workspace-final-tests.log. Runner tests passed 8/8. Static UI audit passed 22 contracts, 5 modal bounds, 6 contrast pairs and desktop target sizes. English-only content gate passed. These are automated checks, not proof of native DPI or target-user usability.

Meaningful module checks cover typed equal IDs, literal search, secret sentinels, read-only details/Back, live hidden/deleted resources, real search arrow/Enter/Escape/Ctrl+K behavior, modal guards, saved directory layout, view/action bounds, date sorting/calendar boundaries, changed expiry cycles, failed-action atomicity, schema21 backup/upgrade/restart/full restore and failed-migration rollback. The full suite also covers existing server/pending-SSH transactions, import/restore, graph hosts/visibility and table scrolling.

## Performance evidence and limits

Machine: Intel Core i7-10750H, 6 physical / 12 logical cores, Windows. Rust 1.92.0, debug profile. Twenty synthetic searches across 10,000 public in-memory summaries, with all matching an owner: p95 52.2984ms. Log: output/workspace-performance.log. This excludes SQLite, model creation, the 150ms input debounce and native rendering. It does not establish the full input-to-result latency target.

Still pending: native 50/100/200-node drag/render p95, over-budget frame share and full input-to-result measurement. No graph frame-rate claim is made from model tests.

## Native visual review

The desktop is locked. The OS screenshot helper captured the lock screen, so that image is rejected as application evidence. A debug-only, demo-only ITGLA_AUDIT_SNAPSHOT hook captures the actual Slint software renderer to PPM without reading the desktop. Converted PNGs are inspected separately. Such captures verify rendered layout; they do not prove real pointer feel, system DPI, screen-reader output or installer behavior.

Native capture results are appended after the final executable is built. Do not count the HTML prototype as a native screenshot.

## Spreadsheet dependency review

The dependency audit found reachable RUSTSEC-2026-0436 in existing calamine 0.36.1, affecting malformed XLS/XLSB/CFB input. Upgrade to calamine 0.38.0, MIT, pure Rust, MSRV 1.88 (below our 1.92 toolchain), retaining default-features=false. Existing API/format support is retained; no new direct dependency or native runtime requirement. Alternatives were to remove XLS/XLSB import, defer a reachable denial-of-service flaw, or patch the parser locally; the maintained upstream fix preserves compatibility with the least custom code.

The lockfile also resolves zip 9.1.0 and quick-xml 0.42.0 for calamine, plus Cargo's required transitive metadata/version refresh. ZIP extraction vulnerability RUSTSEC-2026-0429 is fixed by this update. A valid minimal XLSX fixture and a truncated OLE rejection test check import compatibility/error behavior. The truncated fixture is not claimed to reproduce every upstream security trigger. No workbook or ZIP archive is extracted to the filesystem by ITGLA.

Two scoped audit entries follow the repository's existing exception convention. RUSTSEC-2026-0433 requires ENABLE_WIKILINKS; inspected Slint 1.18.1 styled_text::parse_interpolated sets only ENABLE_STRIKETHROUGH, and ITGLA has no direct Markdown parser. RUSTSEC-2026-0441 is an unmaintained pin-utils dependency via spin_on/Slint with no safe upstream upgrade, retained as a maintenance risk, not declared vulnerability-free. Owner: ken-water. Reassess both by 2026-11-24. No blanket advisory suppression was introduced. Windows-target advisories passed using locked offline Cargo metadata and the advisory database fetched during the initial audit. Linux/macOS graph audits are still pending. See deny.toml and output/workspace-advisories-final.log.

## Migration and rollback

Only in-memory and unique temporary databases were used. Schema21 fixtures are constructed by removing the additive schema22 objects from an isolated current database. Opening one creates backups/pre-schema-22.db before migration; the tests check version21, integrity_check=ok, commit to22, persistence after restart, full backup/restore, deletion cleanup and failed upgrade leaving version21 intact. This is synthetic fixture evidence; it is not a production database migration.

For rollback, stop the candidate, preserve the entire schema22 workspace and backup files in a separate folder, then restore the pre-schema22 backup with the prior executable. Old executables are not promised to open schema22. Restoring the earlier backup loses post-backup edits; preserve the newer database first. Old-executable rollback and installer upgrade remain unrun.

## Acceptance still required

- Real Windows pointer/focus and screen-reader review after unlocking; actual 100/150/200% DPI, not merely resized windows.
- Linux/macOS basic use and packaged Windows upgrade/rollback.
- Full native search latency and map-frame performance targets.
- WS-31 observation with five target users. No user observations have been fabricated.
- Any gate without sufficient evidence remains pending/failed in machine-readable evidence; implementation is not a production-release approval.

The local candidate must be launched with a unique ITGLA_DATA_DIR to avoid migrating the user's active database. Preview instructions and final test results follow below.

## Isolated preview

Run from PowerShell:

```powershell
& D:/devops/itgla/scripts/preview-workspace.ps1
& D:/devops/itgla/scripts/preview-workspace.ps1 -View assets -LargeDark -WindowSize 1100x720
```

The script starts target/debug/itgla.exe in a newly allocated temporary workspace with fictional demo data. It restores the caller's environment variables after launching. Browser port4200 remains a prototype. This is a native debug test candidate, not an installer or production release.

Initial native-render snapshots covered overview, directory, search, details, reminders and maximum-size dark settings/map/directory at1100x720. Review found oversized search results, cramped two-line connected-resource buttons and an overwide new-type selector. These are corrected to fixed64px search rows, readable resource rows with a compact View action and a144px selector/36px actions. The overview now ignores the reminder archive filter without resetting the filter on the reminders page. Final snapshots/checks are recorded after rebuilding these corrections.

## Acceptance coverage status

| Acceptance IDs | Automated evidence | Remaining native / external acceptance |
| --- | --- | --- |
| WS-01/02/03/06/07/08/11/12/16/17/19/21/24/25/29 | Full120 Rust suite; typed/live projections, data safety, graph and isolated migration tests | Synthetic fixtures do not replace imported customer workspaces |
| WS-04/05 | Real Slint input dispatch,150ms debounce, arrows/Enter/Escape/Ctrl+K and modal guards | Input-to-render timing and exact originating-widget focus need real native review |
| WS-09/10/13/14/15 | Table scroll/layout, source/host consistency, fictional demo and existing import/backup tests | Native multi-host edit/restart and complete scoped importer walkthrough |
| WS-18 | Bounded graph traversal and200-node cap | Actual rendered frame latency at50/100/200 nodes unmeasured |
| WS-20/22/23 | Reminder persistence/atomicity, Work remains authoritative, injected leap/year/DST calendar semantics | Open-session midnight and real Work walkthrough |
| WS-26/27/28 | Stable page IDs, saved local layout, three-size/five-theme test coverage and static contrast checks | Native DPI/screen reader/OS keyboard review; resize is not DPI |
| WS-30/31 | Existing storage/credential/connect regressions in full suite | Packaged upgrade, Linux/macOS, five actual target users |

Known delivery limits: no AI/natural-language question answering, cloud discovery, automated certificate scanning or closed-app notification service was added. Domain reminders derive from recorded dates. Imported records retain their existing provenance and unknown history is not reconstructed.

## Build-network adaptation

The user's global Cargo configuration points at a queued TUNA Git index. Local builds use output/workspace-cargo-network.toml, a command-scoped sparse-registry override with proxy127.0.0.1:5670, without changing global configuration. Clippy forwards Cargo configuration only when passed after the subcommand, so the final invocation is `cargo +1.92.0 clippy --config output/workspace-cargo-network.toml --offline --all-targets --all-features --locked -j2 -- -D warnings`. Final builds use the locked, cached dependency graph. Initial interrupted index-queue attempts do not count as successful checks.

Final corrected-source Clippy passed with `-D warnings` (6m44s, including fresh checks after the registry override). This compiler check includes all targets/all features supported by the Windows build; it is not a Linux/macOS execution result.

## Final native-render inspection

Nine final Slint software-render snapshots generated successfully. Inspected overview, search, details, maximum-text dark directory/settings/map; the initial stretching/cramped-label defects are corrected. Capture manifest: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\manifest.json. These are actual native renders at1100x720, plus overview at1440x900. Actual Windows DPI/drag/screen-reader acceptance remains pending. The graph intentionally keeps table-aligned structural columns and uses canvas panning when the small viewport cannot fit all columns.

- overview-1100x720: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\overview-1100x720.png
- assets-1100x720: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\assets-1100x720.png
- search-1100x720: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\search-1100x720.png
- details-1100x720: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\details-1100x720.png
- reminders-1100x720: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\reminders-1100x720.png
- assets-dark-1100x720: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\assets-dark-1100x720.png
- settings-dark-1100x720: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\settings-dark-1100x720.png
- connections-dark-1100x720: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\connections-dark-1100x720.png
- overview-1440x900: C:\Users\dell\AppData\Local\Temp\itgla-workspace-final-native\overview-1440x900.png

Native debug candidate SHA256: `000f0ace5717b09ed8db9ccd7385685a16e21c102282c50fd324e657d0c10037`.

## Machine-governance outcome

UI check is blocked by deliberately unpassed real DPI/responsiveness and native accessibility acceptance. Rust check is blocked by unmeasured native performance, cross-platform advisory coverage ; the locked release build subsequently passed. The generic Rust skill's filesystem allowlist also does not include this repository's Slint UI, migrations, scripts or deny.toml. These paths are explicitly included in the user-approved task; this is a policy/topology mismatch, not a request to change unrelated files. The global skill policy was not modified to manufacture a pass. See output/workspace-ui-governance.json and output/workspace-rust-governance.json. No production approval is inferred from local compilation.

The isolated final native preview was launched successfully with scripts/preview-workspace.ps1 (process4444). The user can inspect it after unlocking Windows. It uses a newly created temporary workspace, not the active user database.

## Final build result

Locked optimized Windows release build passed (16m03s, clean optimized dependency build). Executable: target/release/itgla.exe. SHA256: `0e7a359255de372c1b9df8a2cadad1c3988fc72f85533dea0c049c9a8bc132ad`. This proves compilation, not installer or cross-platform acceptance. Log: output/workspace-release-build.log. No Git commit, push, package publication or production deployment was performed.

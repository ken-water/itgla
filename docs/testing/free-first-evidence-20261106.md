# Free-first implementation evidence

Date: 2026-11-06. This is a local Windows candidate, not a production release or installer acceptance.

## Delivered changes

- Three first-use entries: isolated sample data, validated server entry and scoped import. Import resets stale ready/error/site state through the shared command.
- Optional overview guidance, compact search/map/reminder actions and a direct route to the backup section. Existing users do not receive a mandatory new wizard. Domain-only and Sites workspaces no longer trigger first-server onboarding on startup.
- Full database backup destination selection and confirmed recovery. Cancel/Escape do not write; background navigation is blocked. Successful recovery closes ITGLA so reopened models use the restored data. Invalid backups leave a usable error state.
- Protection against current-database and pre-restore recovery-point aliases containing `..`. Existing full backup/recovery and migration tests remain in the suite.
- Readable metadata handover export with host/relationship context, server purpose, environment, contact, recorded verification, service version/confirmation and domain expiry/registrar. Dedicated credentials, custom fields, documentation links, free-form notes and hidden records are omitted. The report never overwrites an existing file; metadata must still be reviewed before sharing.
- Explicit reminder limitations: recorded dates/manual Work, recalculated while the program is open; no closed-app notifications or automatic certificate checks.
- Free/Pro/Team boundaries, getting-started instructions, honest current-release/local-preview separation, native candidate screenshot and matching sitemap/llms entries. Historical Founder Pro price assumptions are marked superseded.
- A five-person task plan and blank local observation CSV. No tracking, real participant observations, outreach or purchases were created.

## Verification

| Check | Result | Evidence |
| --- | --- | --- |
| Rust full suite | 122 passed, 0 failed | output/free-first-tests-final.log |
| Clippy, all targets/features supported by this Windows build, warnings denied | Passed | output/free-first-clippy-final.log |
| Locked offline Windows debug build | Passed; 2m32s | output/free-first-build.log |
| Website browser regression | 8 passed, including 390/768/1440px, public links, current-release distinction and CTA contrast | output/free-first-website-final.log |
| Format, English, shared footer, static UI contracts, diff whitespace | Passed | output/free-first-checks.json |
| Sitemap/llms consistency | All 15 sitemap URLs appear in llms.txt | Local XML/text comparison |
| Native software render inspection | Six views at1100x720: overview, guide, backup, recovery, assets, connections; four in maximum-text dark theme | Capture manifest below |

The first visual pass caught clipped demo explanatory text and redundant overview guidance; the final build fixes both. Website screenshots caught dark CTA text inherited from document links; the rule and contrast regression now cover that defect. Native map panning and bounded columns remain the existing design; a screenshot does not establish frame timing.

Candidate: `target/debug/itgla.exe`.

SHA256: `88545b1c9cebb1f0ac51b549e888bfaed939a6cafd09202fb88ffbcd12fe493e`.

Native capture manifest: `C:/Users/dell/AppData/Local/Temp/itgla-free-first-native-urn6wdqs/manifest.json`. Captures use fictional in-memory demo records and unique temporary workspace directories. The capture script terminates only its own screenshot processes.

An independent first-use preview was opened with `scripts/start-acceptance.py --binary target/debug/itgla.exe --launch`. Launch record: `test-results/acceptance-20261106T091608Z-c35aac9c/launch.json`. The real user database was not opened or migrated.

Local website: `http://127.0.0.1:4201/`; browser port4200 remains the earlier design prototype. Current public download packages remain0.6.2; these changes are local and pending release acceptance.

## Pending acceptance and governance

Actual100/150/200% Windows DPI, native screen-reader/focus-origin review, full input-to-render search timing, rendered map-frame p95, packaged upgrade/rollback, Linux/macOS basic use and the five real target-user sessions remain unrun. The pilot's activation and return targets are learning goals, not measured results.

Machine UI acceptance must remain blocked for unrun accessibility and real DPI. The Rust evaluator also sees the combined previously uncommitted workspace changes, including earlier dependency/migration work; its generic file and change-budget policy does not represent a new isolated free-first diff. Record actual evaluator output rather than treating these checks as passes. No global skill policy was changed to manufacture acceptance. See `output/free-first-ui-governance.json` and `output/free-first-rust-governance.json`.

The UI skill's workflow requires: "A gate is evidence, not an assertion." The pending native/platform/user checks therefore remain explicit. This does not prevent reviewing the implemented local candidate; it prevents claiming production readiness.

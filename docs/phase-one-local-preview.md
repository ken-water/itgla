# Phase one: usable asset responsibility inventory

Local implementation, 2026-10-30; not a public release.

## Try it

The isolated Windows preview is prepared in `dist/phase-one-preview`. Run `Start-ITGLA-Preview.cmd`; its database stays in the adjacent `workspace` directory. Choose **Open sample data**, then inspect Businesses, Servers, Services and Domains. Sample contacts and sources are fictional. This launcher does not open the normal user data directory.

Open a resource editor and find **Responsibility & references**. Enter the technical contact, backup contact, optional environment (servers retain their existing field), documentation availability/link and information source. Blank contacts mean Not recorded; literal Unknown or Not applicable are retained. Documentation provides explicit Not recorded, Unknown, Not applicable and Link available choices. Only HTTP(S) links without embedded credentials can be opened.

Select **I checked this information**, optionally enter the reviewer, then Save. The check is local manual evidence; it expires after 90 days and is cleared by relevant metadata/context/hosting edits. Unchanged saves retain checks. New servers still produce a pending port-22 SSH service to review separately. No additional field is mandatory.

Search businesses, servers, services or domains by contact, backup, source or documentation URL. Business lists emphasize contact/check status; the editor retains the full original notes and tags. Service/domain tables include responsibility summaries and their filtered CSV exports include context. In narrow windows, editing a service/domain temporarily replaces the table with a full-width form; Close returns to the table without clearing the draft. Wider windows retain the side-by-side layout.

Use a full SQLite backup for recovery. Service/domain CSV, server CSV and legacy project JSON are partial exchanges, not a recoverable workspace. Migration 21 creates `backups/pre-schema-21.db`; rolling back requires the matching older binary and snapshot, with the newer database preserved separately.

## Validation boundary

The automated suite covers saved context and draft isolation, invalid-link rollback including automatic SSH, unchanged/changed checks, hidden resources, explicit availability states, schema upgrades, existing checks, reopening, backup/restore, filtered CSV and largest-font form behavior. Native Windows software-renderer inspection uses an isolated sample workspace at maximum font size; business editing, scrolling, fixed saves and narrow service editors are checked. Screenshots and command logs are retained under `test-results/asset-context` locally. The final local suite has 93 passing tests; format, Clippy with denied warnings, English-only text, static UI contracts and component governance checks pass. A separate synthetic 1,000-record service search refresh measured 94.6191 ms (no native paint, disk or credential operations). Screenshots were inspected at 1100x720 physical pixels with maximum font size; the full 1440x900 screenshot requirement has not been verified on this display. Real users have not yet validated the proposed ten-minute onboarding / thirty-second lookup targets. Coverage tooling and cargo-deny are not installed; dependency audit is not claimed. These limits prevent calling this a production release gate pass.

## Next phase

Business-level resource summaries, operational instruction templates, handover preparation, scoped offline information packages and formal recipient acceptance remain later work. Cloud collaboration and new asset categories are not included in this step.

## Recorded verification outcome

Final Windows optimized build completed with `--locked`; the release binary was copied to the isolated preview folder and opened successfully with the software renderer and a new local workspace. SHA256: `21A09A144AFB0C5EB0FAEA70D193AC7740E4D46F23F499B187E49C24D88EA25D`.

UI component governance passes. The Rust skill's aggregate checker reports blocked: cargo-deny is missing, and its generic filesystem allowlist excludes Slint UI, SQL migrations, website files and `.agents` paths despite the task's declared scope. The user-authorized local feature includes UI and migration work; website edits were already present and preserved. Do not reinterpret this checker result as an all-gates pass or a production authorization. Full-display screenshot and real-user timing limits remain as described above.

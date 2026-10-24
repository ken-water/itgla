# Changelog

## Unreleased

## 0.5.0 - 2026-10-24

- Improved desktop action clarity with distinct primary and secondary buttons, visible focus states, and safer disabled actions.
- Added immediate Appearance previews for color, text size, and row spacing, with explicit saving and recoverable error feedback.
- Added a dedicated New status input in Settings and preserved independent environment drafts.
- Added inventory verification dates, CSV export, and formula-safe exported values.
- Refreshed the official website homepage, download journey, release notes, sitemap, and `llms.txt` content.

## 0.4.0 - 2026-10-24

- Added a Services & Domains inventory linked to existing servers, with versions, ports, access addresses, tags and notes.
- Added optional Windows CurrentUser DPAPI-protected service accounts with masked fields, explicit show/copy and 30-second unlock expiry. Linux/macOS support metadata; credential storage is Windows-only.
- Added directed dependency, replication, failover and load-balancing relations visible from both service endpoints.
- Added domain/hostname parent, registrar, registration/expiry dates, CDN/DNS targets and multiple origin servers.
- Added schema 16 with a pre-migration backup, transactional migration and backup/restore regression coverage. Full SQLite backups include inventory; legacy project JSON does not. Older binaries require the pre-migration database to roll back.

## 0.3.30 - 2026-10-18

- Added managed tag suggestions while preserving free-form tags, plus typed custom Server fields with defaults, ordering, and reversible hiding in storage.
- Added deterministic duplicate handling for CSV/Excel imports: create new, skip existing, or update by Server IP/Site URL.
- Added timestamped validated SQLite backups with configurable 1–30 file retention.
- Published verified Windows EXE/ZIP, Linux DEB/RPM/AppImage, and macOS arm64 DMG packages with a combined SHA-256 manifest.

## 0.3.29 - 2026-10-18

- Added a dedicated Settings workspace with Field Atlas, Slate, Sand, and High Contrast themes.
- Added persistent text size, table density, compact cell-preview, and fit-columns-on-open preferences.
- Added an isolated, resettable Demo workspace containing fictional Servers, Sites, tags, environments, statuses, and custom fields; real workspace data never enters the demo database.
- Added a visible Demo state with Reset and Exit actions so experimentation cannot be mistaken for the real workspace.

## 0.3.28 - 2026-10-18

- Added managed Environment and Status choices with defaults, safe rename semantics, and dropdown editors for Servers and Sites.
- Added additive schema 015 and compatibility-preserving migration of historical Environment and Status values.
- Applied configured Environment and Status defaults to new records and spreadsheet imports.
- Added persistent preference foundations for default sorting, visible standard columns, default ports/tags, import behavior, and backup retention.

- Simplified the visible workspace to two consistent resource tabs: Servers and Sites.
- Added persistent Sites with Name, URL, Environment, Status, Purpose, and Tags fields.
- Added Sites search, bidirectional sorting, copy/edit cells, resizable/auto-fit columns, and hide/restore flows.
- Added Excel/CSV Sites import with automatic column matching, manual mapping, defaults, and a downloadable template.
- Made the toolbar count and Fit columns action follow the active resource tab.
- Preserved existing Projects, Operations, and Work data while removing those unfinished surfaces from navigation.
- Added additive schema 013 and migration/CRUD/import regression coverage.
- Promoted Server Name, Environment, Status, and Purpose to first-class fields alongside IP Address, Ports, and Tags.
- Added schema 014 migration that preserves existing Server data, converts matching custom fields, retains other custom columns, and remaps saved column widths.
- Updated Server create/edit, search, bidirectional sorting, cell editing, Excel/CSV import, defaults, templates, and accessibility contracts for the seven standard fields.
- Expanded native desktop interaction coverage across Server field editing, horizontal scrolling, column resizing, hide/restore, import, and navigation.

## 0.3.27 - 2026-10-17

- Made the large (A+) text size the default for new workspaces, retaining saved text-size preferences.
- Fixed header sort-direction toggling and active arrows; custom numeric values sort by number in both directions.
- Restricted cell previews to truncated values, using delayed compact tooltips rather than oversized instant overlays.
- Fixed column resizing to require a held drag, separated resize and sort hit targets, and added persistent-width regression checks.
- Improved Hide confirmation contrast and primary-action styling.
- Fixed Servers table Auto Fit geometry so column-width changes are counted once and rightmost content remains reachable.
- Renamed Auto to Fit columns and reset horizontal scroll to the left after fitting.
- Added a first-run Guide for adding/importing servers, choosing tags or structured fields, and planning changes in Work.
- Improved the empty Servers state to distinguish a new workspace from filtered results.

## 0.3.26 - 2026-10-17

- Prevented one-hop and two-hop impact traversal from crossing archived assets or archived projects.
- Kept impact preview rows and exact totals aligned for active-resource scope only.
- Added a regression assertion for archived intermediate resources.

## 0.3.25 - 2026-10-17

- Added explicit schema 10 → 12 and schema 11 → 12 migration regression coverage.
- Verified ignored impacts plus Direct and Manual scope origins survive database reopen.
- Added deterministic graph fixtures for certificate, website, domain, service, and server impact paths, including two-hop deduplication and exclusion.

## 0.3.24 - 2026-10-17

- Impact previews now report the exact total when only the first 50 deterministic results are rendered.
- Capped previews show `Showing 50 of N` and keep `Add shown` semantics.
- Added a large relationship fixture test for cap and total behavior.

## 0.3.23 - 2026-10-17

- Persisted Work resource scope origins as Direct or Manual.
- Resources promoted from inferred impact are Manual; resources selected directly are Direct.
- Normal Work saves preserve existing origins instead of silently rewriting them.
- Added additive schema 012 for scope origin metadata.

## 0.3.22 - 2026-10-17

- Added per-Work `Ignore` decisions for reviewed inferred impacts.
- Ignored impacts no longer block Ready and can be restored with `Reset ignored`.
- Added additive schema 011 for durable impact review decisions.

## 0.3.21 - 2026-10-17

- Added checklist move-up and move-down actions with transactional ordering updates.
- Work cannot be marked Done while any checklist item remains incomplete.
- Done protection applies to both detail saves and quick list actions.

## 0.3.20 - 2026-10-17

- Added a usable Work checklist dialog with completion toggles, add, and delete actions.
- Checklist progress updates the Work list immediately and persists in SQLite.
- New Work continues to receive a starter checklist based on its type.

## 0.3.19 - 2026-10-17

- Added quick Work status actions in the list: Start, Done, Resume, Reopen, and Block.
- Quick actions use typed status values, preserve active list filters/sorting, and refresh an open detail view.

## 0.3.18 - 2026-10-17

- Added local due-date states for standard `YYYY-MM-DD` values: Overdue, Due today, Due soon, and Scheduled.
- Empty dates show No due date; non-standard text windows remain supported as Flexible.
- Added clearer list colors for overdue and near-term Work without adding a date library dependency.

## 0.3.17 - 2026-10-17

- Added Work list sorting by priority, due date, status, or title, plus the existing default order.
- Sorting composes with keyword, kind, and status filters and remains active after saves and archives.
- Empty due dates sort after populated due dates.

## 0.3.16 - 2026-10-17

- Added Work list search across title, owner, and notes.
- Added typed Work filters for kind and status.
- Active filters remain applied after saving or archiving Work, with a dedicated no-match empty state.

## 0.3.15 - 2026-10-17

- Replaced free-text Work type and status fields with explicit dropdowns.
- The UI shows friendly labels while persistence continues to use stable typed keys.
- Eliminated common save failures caused by misspelled type/status values.

## 0.3.14 - 2026-10-17

- Added contextual actions to the Work health summary.
- Health actions can guide users to scope review, reset the impact view for review, or focus the missing owner/due-date field.
- Actions never modify business scope without an explicit user confirmation.

## 0.3.13 - 2026-10-17

- Added a Work scope health check with actionable warnings.
- Detects empty scope, projects without concrete resources, resources outside selected projects, unresolved potential impact, and Ready work missing an owner or due date.
- Health checks are advisory except for the existing unresolved-impact Ready gate.

## 0.3.12 - 2026-10-17

- Persisted per-Work impact view preferences locally: depth and resource type filter.
- Reopening a Work restores its last impact view without changing saved scope.

## 0.3.11 - 2026-10-17

- Added an impact summary with total, domain, certificate, website, service, and server counts.
- The summary reflects the current bounded preview and indicates when the 50-result cap applies.

## 0.3.10 - 2026-10-16

- Added a lightweight inferred-impact type filter: All, Domains, Certificates, Websites, Services, and Servers.
- Filtering is presentation-only and never changes saved Work scope or inferred data.
- Bulk promotion follows the active type filter so only the visible category is added when a filter is selected.

## 0.3.9 - 2026-10-16

- Capped inferred impact previews at 50 deterministic results.
- Added an explicit UI notice when the preview is showing up to the limit.
- Direct Work scope and bulk promotion behavior remain unchanged.

## 0.3.8 - 2026-10-16

- Added a one-hop/two-hop impact depth switch, defaulting to one hop.
- Two-hop suggestions remain review-only and are deterministically ordered.
- Existing saved scope and Ready-state safety behavior are unchanged.

## 0.3.7 - 2026-10-16

- Grouped inferred impact suggestions by resource category.
- Added visible category labels for websites, domains, certificates, services, servers, and other resources.
- Kept deterministic ordering and the existing review-only confirmation flow.

## 0.3.6 - 2026-10-16

- Added a safety gate for marking existing Work as `Ready`.
- Work with unconfirmed inferred impact must be reviewed before it can enter `Ready`.
- The message points users to the existing single-item and bulk scope confirmation actions.

## 0.3.5 - 2026-10-16

- Added `Add all` for one-hop inferred impact suggestions.
- Bulk promotion is transactional, deduplicated, and preserves existing direct scope.

## 0.3.4 - 2026-10-16

- Added one-click promotion of an inferred impact into the saved Work scope.
- Promotion is additive and transactional; existing direct scope selections are preserved.
- The impact preview refreshes immediately after a resource is confirmed.

## 0.3.3 - 2026-10-16

- Added a bounded, one-hop impact preview for directly selected resources.
- Inferred neighbors are deduplicated, deterministically ordered, and labeled with the relationship that caused the suggestion.
- Inferred impact remains a review-only suggestion and is never silently added to saved Work scope.

## 0.3.2 - 2026-10-15

- Added direct Work scope links for websites, domains, certificates, servers, and services.
- Work lists and detail views now show selected asset counts alongside projects and server records.
- Added additive SQLite migration 010 for work-to-asset links; existing workspaces remain compatible.

## 0.3.1 - 2026-10-15

- Added explicit Work scope links to projects and server records.
- Added bounded impact counts and selectable project/server scope in the Work detail panel.
- Added SQLite schema 9 with transactional scope replacement.

## 0.3.0 - 2026-10-15

- Added a local Work inbox for changes, tasks, renewals, incidents, migrations, launches, and maintenance.
- Added typed work statuses, priorities, owners, dates, notes, and generated checklists.
- Migrated existing Operations records into compatible Work records without deleting the original data.
- Added the first one-window Work list and detail editor in Slint.

## Unreleased

## [0.2.4] - 2026-10-13

### Added

- Double-click any server-table cell to edit and save its value in place.
- Drag sortable table headers to resize columns manually.
- Added an Auto column-width action that fits columns to visible headers and values.
- Persisted font-size and column-width preferences in the local SQLite workspace.

### Fixed

- Kept table row/header models synchronized while importing custom columns to prevent the desktop window from exiting after import.

## [0.2.3] - 2026-10-05

### Added

- Added persistent Projects and Operations tabs backed by SQLite schema v6.
- Added persistent custom priority and media options.
- Added the ITGLA logo to the native application window and package resources.
- Added enlarged hover previews for dense table fields.

## [0.2.2] - 2026-09-25

### Added

- Added Overview, Traffic, Downloads, and Errors tabs to the protected analytics dashboard.
- Each tab now presents focused core metrics, a trend chart, and the relevant data table.
- Added Windows installer, Linux DEB/RPM/AppImage, and macOS DMG release packages.

### Changed

- Dashboard data is loaded once per reporting-period change and reused across tabs.
- Preserved the existing authentication, retention, privacy, and service-isolation boundaries.
- Added SHA-256 checksums for every release artifact and a combined release manifest.

## [0.2.0] - 2026-09-25

### Added

- Dense server inventory with multi-tag filtering and per-cell copy actions.
- Excel, OpenDocument, CSV, and TSV import with per-column mapping.
- Persistent custom columns and values through SQLite schema v5.
- Alphabetical, IP-address, port, date-added, and custom-column sorting.

### Changed

- The current desktop experience focuses on server tags, IP addresses, ports, and imported tabular data.
- Existing schema-v3 purpose values migrate into tags without deleting legacy records.
- Table cells now copy on click with a compact confirmation notice instead of a full Copy button.
- Sorting now lives in each sortable column header with ascending and descending arrows.
- Saving keeps the current server open, and reversible Hide/Restore replaces the server-facing Archive action.

### Privacy and website

- Added complete English Privacy, Terms of Use, Refund, and Cookie policies.
- Clarified that the public website sets no non-essential cookies and that the protected administrator session cookie expires after eight hours.

### Compatibility

- Existing databases migrate sequentially through schema v5 without deleting legacy tables or records.
- Releases that only support older schemas fail closed when opening a schema-v5 database; back up `itgla.db` before rolling back.
- Windows packages remain unsigned and may trigger Microsoft Defender SmartScreen.

All notable changes to ITGLA are documented here. Versions follow Semantic Versioning.

## [0.1.1] - 2026-09-19

### Added

- Reproducible Windows x86_64 portable packaging with a versioned ZIP and SHA-256 sidecar.
- Windows-native CI tests, release build, and bounded GUI startup smoke validation.
- Official website source and deployment documentation for current platform downloads.

### Fixed

- Windows builds now use the GUI subsystem and no longer open a separate console window.

### Compatibility

- Continues SQLite schema v2 and JSON exchange format 1 without migrations or configuration changes.
- Windows packages are currently unsigned and may trigger Microsoft Defender SmartScreen.

## [0.1.0] - 2026-09-19

### Added

- Cross-project search across project names, resource names, types, tags, and descriptions.
- Working All Resources and Needs Attention views with risk-first ordering and a 100-result bound.
- Result navigation that opens the owning project, selects the resource, and highlights its graph node.
- Distinct empty states for new projects, filtered results, missing relationships, and healthy workspaces.
- Accessible roles, labels, and default actions for custom navigation, filters, resource rows, graph nodes, and relationship controls.

### Changed

- Replaced prototype sync and user placeholders with accurate local-save and version information.
- Project search now includes tags and resets coherently after navigation from global results.
- Empty repository refresh clears all derived graph, relationship, and option models.

### Verification

- Verified global, attention, result-navigation, empty-project, data-safety, and recoverable import-error states with real process interaction and 1440x900 software-renderer screenshots.
- Confirmed a failed import leaves the existing three-project workspace unchanged.

## [0.0.5] - 2026-09-19

### Added

- Versioned JSON export and transactional import for active projects, resources, tags, and relationships.
- Full SQLite backup and same-schema restore from editable local paths.
- Automatic `pre-import.db` and `pre-restore.db` recovery points before replacement operations.
- A data-safety panel with explicit confirmation before import or restore.

### Safety

- JSON imports are limited to 10 MiB and validate format version, counts, identifiers, types, tags, project ownership, and relationship references before writing.
- Invalid imports leave the current workspace unchanged; database restores require schema v2 and a successful SQLite integrity check.

### Known limitations

- File paths are entered directly; a native file picker is not bundled in this release.
- JSON exchange contains active records; use a database backup to retain archived records.

## [0.0.4] - 2026-09-19

### Added

- Typed relationships between resources with create and confirmation-gated remove workflows.
- A data-driven project graph whose nodes select the matching resource row.
- A relationship list that shows source, type, and target for the current project.
- Schema v2 migration with relationship archival and deterministic example relationships.

### Changed

- Database migrations now run sequentially in transactions and reject unknown newer schemas.
- The relationship graph is generated from local assets and relationships instead of static UI content.

### Migration

- Upgrades schema v1 to schema v2 without removing projects, resources, tags, or relationships.
- Releases through `v0.0.3` reject schema v2 safely; back up `itgla.db` before rolling back.

## [0.0.3] - 2026-09-19

### Added

- Create, edit, and archive workflows for projects and resources.
- Confirmation before archival and protection against archiving the final project.
- Comma-separated resource tags with validation, deduplication, persistence, and search.
- Dynamic project navigation with live resource and attention counts.
- Success and recoverable validation feedback that preserves editor input.

### Changed

- Project and resource navigation now derive entirely from local database content.
- Resource rows open the editor and show assigned tags alongside their description.

### Known limitations

- Archived records do not yet have a restore screen.
- The relationship graph still uses the prototype layout and is not editable.

## [0.0.2] - 2026-09-19

### Added

- Versioned SQLite schema for projects, resources, tags, and typed relationships.
- Atomic first-run migration and deterministic one-time sample-data import.
- Platform-aware data directory with an `ITGLA_DATA_DIR` override.
- Typed storage failures and an in-app data-read error state.
- Persistence tests covering migration, reopen behavior, and newer-schema rejection.

### Changed

- Resource lists now load from the local database instead of compiled Rust fixtures.

### Migration

- Creates schema version 1 and imports sample content only when the project table is empty.
- Rolling back to `v0.0.1` leaves `itgla.db` untouched and unused.

## [0.0.1] - 2026-09-19

### Added

- Initial Slint desktop application shell.
- Project-centered resource inventory for websites, domains, certificates, servers, and services.
- Project switching, resource filtering, search, health states, and a relationship overview.
- Typed Rust domain fixtures with focused filtering tests.
- Documented Rust architecture, dependency choices, verification gates, and rollback boundary.

### Known limitations

- Data is compiled sample content and is not persisted.
- Add-resource and global navigation actions are not implemented.
- Relationship layout is a deterministic project overview rather than a user-editable graph.

[0.0.1]: https://github.com/ken-water/itgla/releases/tag/v0.0.1
[0.0.2]: https://github.com/ken-water/itgla/releases/tag/v0.0.2
[0.0.3]: https://github.com/ken-water/itgla/releases/tag/v0.0.3
[0.0.4]: https://github.com/ken-water/itgla/releases/tag/v0.0.4
[0.0.5]: https://github.com/ken-water/itgla/releases/tag/v0.0.5
[0.1.0]: https://github.com/ken-water/itgla/releases/tag/v0.1.0
[0.1.1]: https://github.com/ken-water/itgla/releases/tag/v0.1.1
[0.2.0]: https://github.com/ken-water/itgla/releases/tag/v0.2.0
[0.2.2]: https://github.com/ken-water/itgla/releases/tag/v0.2.2

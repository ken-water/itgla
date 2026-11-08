# ITGLA workspace development plan

Date: 2026-11-05. Native implementation is in progress; see the implementation evidence for actual verified status. Design: [interactive prototype](design/workspace-20261105/README.md). This is an incremental Rust/Slint implementation, not an HTML embed.

## Scope

Provide one path to search an asset, understand hosts and dependencies, find a contact and handle the next action. Deliver local global search, shared directory/details, actionable overview, type-specific add/import, focused relationship map, domain expiry/manual Work reminders and navigation/appearance integration.

Excluded: AI search, discovery, cloud sync, billing, team permissions, automatic certificate scanning, resident notifications and automatic failure propagation. A domain expiry is not a TLS certificate expiry. Update marketing/screenshots only after desktop acceptance. No online download/admin changes, release/version change or production deployment is authorized by this plan.

## Reuse and boundaries

| Capability | Existing source | Treatment |
| --- | --- | --- |
| Servers and validation | domain.rs, storage.rs, server_records | Reuse existing validation and transactions |
| Services/domains/hosts | operational.rs, operational_inventory, operational_hosts | Preserve ports, credential protection and pending SSH creation |
| Businesses/directed relations | relationships.rs, businesses, resource_links | Typed stable resource IDs; existing link validation |
| Maintenance context | asset_context.rs, asset_context | Owner, environment, docs, source and verification |
| Focus/hide/collapse | topology.rs, topology.slint | One graph model, details entry, existing restore semantics |
| Tables | tables.rs, data-table.slint | Shared boundaries, columns and synchronized scrolling |
| Appearance/preferences | controls.slint, appearance.slint, app_settings | Preserve five themes and existing three-size mapping |
| Manual Work | work_items, checklist_items | Project dates; original completion constraints remain authoritative |
| History | resource_history and server history | Show recorded events; never reconstruct unknown history |

GlobalAsset/ResourceKind and work_item_assets belong to a legacy planning model. The workspace projects visible servers/services/domains/businesses; never reinterpret an old Work asset_id as an operational inventory ID. No unified Asset schema rewrite.

## Sequential stages

### S1 - global search and shared details (W01-W04)

Project live data with InventoryResourceRef(kind,id), public fields, host/context summaries and stable ordering. One service stays one result even with multiple hosts. Search names, addresses, tags, purpose and contacts literally; never decrypt/index usernames, passwords or arbitrary secret columns. Default excludes archived resources; existing explicit hidden-record views retain restore access.

Ctrl/Cmd+K opens search outside editors/modals; arrows select results, Enter opens and Escape closes. Shared details are read-only, show host/port/contact/links/history, allow related-resource traversal and Back, and keep the underlying list state. Edit is explicit. Close returns keyboard focus to a useful initiating context. Test viewing without writes and cancellation without data loss.

### S2 - directory and actionable overview (W05-W06)

One directory covers four resource types. Columns: name/address, type, purpose, environment, contact, last checked. Host summaries are visible for services, with full linked details. Reuse fixed headers and column boundaries; Fit is local to a view. Preserve independent view preferences.

Overview separates technical assets and businesses, shows recorded attention items, business context and recently checked assets. Unknown information is not an incident. Empty state offers add and type-scoped import. Demo remains an isolated fictional workspace and clearly marked.

### S3 - validated add/edit/import (W07-W08)

Route new actions to existing type editors and transactions. Keep required valid address/port/protocol fields; maintenance context stays optional. New servers create exactly one pending port22 SSH service; server/context/service failure rolls back atomically. Services can have multiple real hosts, reflected in both directions and on the map after save/restart. Failed validation preserves drafts; repeated submit must not duplicate records.

Reuse current scoped import/export, preview, mapping, duplicate policies and full backup. Cancelling preview writes nothing; failed mapping cannot partially import. Do not invent a generic importer for unsupported types. Preserve existing manual and credential-specific controls.

### S4 - progressive map (W09)

Reuse real directed relations and typed IDs. Focus begins with direct neighbors and includes required structural ancestors/hosts. Expand/collapse handles shared nodes and cycles with bounded traversal. Hide from map changes display only; archived resource state is separate. Restore is visible with count. Nodes open shared details; editing remains explicit. Preserve domain-to-server-to-service paths and business host context. Keep large graphs bounded with a refine-scope message; do not rebuild all models on pointer movement.

### S5 - persistent manual reminders (W10-W11)

Project domain expiry and dated active Work within30 days, including overdue dates. Unknown/invalid dates are excluded rather than guessed. Do not claim certificate, backup or live monitoring coverage. Work remains authoritative and checklist completion constraints cannot be bypassed.

New schema22 reminder_reviews stores typed domain ID, source expiry cycle, reviewed/postponed state, next local date and timestamps. reminder_activity records action and time, with explicit local workspace actor semantics. Actions are transactional; review is acknowledgment, not renewal. A changed expiry is a fresh cycle. Hide suppresses reminders while retaining state; restore recalculates. Hard deletion cleans state/history. Include state in full backup/restore, migration and transaction tests.

Batch state reads. Inject local calendar date in tests; tomorrow means next local calendar day, not 24 elapsed hours. Recalculate at startup and during open-session foreground use. No closed-application notification promise.

### S6 - navigation and appearance (W12-W14)

Overview, Assets, Asset map, Reminders and Settings form the primary structure. Keep familiar Servers/Services/Domains/Businesses/Sites available as asset shortcuts; Work/Projects/Operations remain reachable from reminders. Preserve stable old numeric page IDs and meaningful defaults. Retain every theme and three discrete font steps, applying shared tokens consistently. Preview/save/cancel must retain their existing semantics. Test modal isolation and keyboard focus.

## Performance and experience targets (not results)

Record hardware/build mode. Aim for first results within100ms for1,000 resources and p95 input-to-result within250ms for10,000, including debounce. Measure before adding FTS/dependencies. Do not load full history or decrypt credentials for initial results; bound displayed rows.

Measure native Windows drag/render frames at50/100/200 visible nodes. At60Hz aim for p95 <=16.7ms and report over-budget frame share. A model test cannot substitute for rendering measurements. Inspect all font sizes at1100x720 and1440x900 and real100/150/200% DPI; resizing is not a DPI test. Table-local horizontal scroll is allowed, with aligned headers.

Text contrast >=4.5:1; necessary control boundaries >=3:1. Check every theme and do not encode state by color alone.

## Verification and delivery

[Acceptance matrix](testing/workspace-acceptance-20261105.md); [backlog/dependencies](workspace-development-backlog-20261105.json). Each stage includes source, meaningful tests, locally runnable Windows candidate, change notes, passes and explicit pending items. S5 includes backup/rollback evidence. Prototype browser checks cannot count as native checks.

```powershell
cargo +1.92.0 fmt --all -- --check
cargo +1.92.0 test --all-features --locked -j 2
cargo +1.92.0 clippy --all-targets --all-features --locked -j 2 -- -D warnings
node scripts/check-english-ui.mjs
python scripts/audit-ui.py
git diff --check
```

Use in-memory or unique temporary databases; never the user's active database. Migrate an isolated schema21 fixture, verify the automatic backup and transaction failure behavior, and restore a copy. Rollback uses the old executable plus pre-schema22 backup; old binaries are not promised to open new schema. Restoring the older backup loses subsequent edits, so preserve the new database separately first.

Release requires native regressions, Windows upgrade, Linux build/basic use, both Mac architectures, consistent downloads/checksums and target-user tasks. Windows/Linux builds stay local or192.168.1.7; GitHub builds only two DMGs. Packaging and production release are separate authorized work. Do not mark unrun platform/DPI/performance/user checks Pass.

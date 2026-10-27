# Business and connection-map local preview - 2026-10-26

## Delivered

- Businesses and Connections are explicit destinations before Settings. Services and Domains retain their compact tables and have a Connections shortcut.
- Business records have names, tags and notes, with hide/restore. Domain/service Business fields and graph Owns links use the same persisted entities.
- A four-lane directed map displays Business, Domain/Subdomain, Server and Service. The canvas adapts to the available width with a minimum scrolling width of 960 logical pixels. The current view caps nodes at 200 and reports truncation; business filtering narrows it.
- The initial structure view shows ownership of root domains, parent/subdomain links, origins and service hosts. Hover traces a resource's additional links. All links includes dependencies, replication, failover, load balancing and direct resolution; Relationships opens the complete visible-node edge list. Select an edge there to change its endpoints/type/notes or remove it. Select a node to open its normal editor.
- Host and service relationships are mirrored transactionally into existing table data. Domain parenting and business ownership update their metadata, with cycle validation and rename propagation. An invalid edit rolls back.
- The in-memory demo contains one business, three domains, eight servers, four services and directed relationships, including PostgreSQL primary/replica and fictional Cloudflare metadata. Reset refreshes the current view. Workspace switches clear graph selections. No discovery or remote execution is involved.
- Cookie Policy headings, prose, section rules and table container share one 820px maximum column. The mobile table remains independently horizontally scrollable.

## Verification

Rust 1.92.0 with locked dependencies: format, strict Clippy, check, 85 tests and Windows debug build. Tests include table/graph synchronization, editing mirrored links, invalid-edit rollback, domain cycles, rename propagation, demo reset, business lifecycle, filters and node editor navigation. Final command outcomes are recorded in connection-map-evidence.json.

Playwright runs against the real local Cookies page at 1440/768/390 pixels. All measured header, section, paragraph and table-container left edges and widths match the main column, without page overflow. Evidence: `%TEMP%/itgla-cookies-20261026/results.json` and screenshots. Native screenshots use an isolated fictional workspace, with normal and largest-font dark views.

Native review also covered the business editor at the minimum supported logical width. It caught and corrected a long-note/large-font Edit-button squeeze, navigation label padding and Path aspect-ratio scaling that displaced connection endpoints. Screenshots: `%TEMP%/itgla-connections-final-20261026.png`, `%TEMP%/itgla-connections-dark-final-20261026.png`, `%TEMP%/itgla-businesses-dark-final-20261026.png`, and `%TEMP%/itgla-businesses-minimum-dark-20261026.png`.

## Scope and limits

This is a Windows local preview, not a published release. Linux/macOS builds, production deployment, full assistive-technology testing and large-workspace usability studies were not performed. Layout and data correctness tests do not establish reduced eye strain.

The workspace already contained uncommitted appearance, inventory, migration, dependency and website changes before this request. Those were retained. Machine governance reports the full dirty working-tree scope rather than attributing those earlier changes to this task; any resulting cumulative budget blockers are recorded, not waived or misrepresented as a clean release gate.

The cumulative governance check is blocked by the L3 file/addition/new-file budgets (33 files, 3956 added lines, 19 new files), the preexisting `webbrowser` dependency delta, and the library profile's filesystem allowlist not covering UI/website/scripts/migrations. The user explicitly requested UI/website work; the skill defaults do not withdraw that authorization. This request added no dependency. The machine check is recorded as blocked; the local format, compile, Clippy, tests, native preview and browser checks passed. Full check output is retained at `%TEMP%/itgla-connection-map-governance-20261026.json`.

Rollback: retain the existing workspace's pre-migration backup; this task adds no schema migration beyond the already pending schema 20 work. The demo is in-memory. Do not use a pre-schema-20 binary against an upgraded database.

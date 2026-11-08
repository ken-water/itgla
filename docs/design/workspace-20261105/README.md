# ITGLA workspace interaction design - 2026-11-05

Implementation references: [six-stage plan](../../workspace-development-plan-20261105.md), [backlog](../../workspace-development-backlog-20261105.json), [acceptance matrix](../../testing/workspace-acceptance-20261105.md).

## Purpose and prototype boundary

Help independent developers and small teams find a resource, understand what runs where, identify a contact and decide what to do next. This directory is an interactive browser prototype, with fictional assets, reserved addresses and a fixed demonstration date of 2026-11-05. Data exists only in page memory and resets on reload. It never reads the user database, opens remote connections or sends notifications. The native implementation is separate; prototype checks are not desktop acceptance evidence.

```powershell
python -m http.server 4200 --bind 127.0.0.1 --directory D:/devops/itgla/docs/design/workspace-20261105
```

Open http://127.0.0.1:4200/ . The product and persisted project documentation use English.

## Information architecture

| Entry | Question | Content and action |
| --- | --- | --- |
| Overview | What needs attention now? | Recorded reminders, business context, recently checked assets; Add asset and Review |
| Assets | Where is the resource? | Cross-type table, type and keyword filters; open shared details |
| Asset map | How do resources connect? | Domains, servers, services and directed links; focus, expand, hide, restore |
| Reminders | What must I remember? | Open, reviewed and postponed items; inspect, acknowledge, defer, reopen |
| Settings | How can I read comfortably? | Themes and three discrete text sizes, immediate preview |

Global search stays in the header with Ctrl/Cmd+K. It searches explicitly public names, types, addresses, purposes, tags and contacts locally. Inputs have editable surfaces; buttons have distinct action surfaces, with one solid green primary action. Business is a first-class resource. Databases remain services; people and teams do not require new top-level menus.

## Core journeys

Search PostgreSQL, see port 5432, its host and contact, then inspect its caller. Follow related resources in read-only details, use Back, or View on map. The map must preserve true hosting servers. Viewing never saves; editing is explicit with save/cancel. Details are nonmodal for comparison, with a close action and Escape.

Quick add starts with type/name/purpose. Native persistence still requires valid server addresses, service protocol/port and domains. Service host selection establishes a real hosting relation. Owner/environment/documentation/source and verification belong in optional information. New servers must continue to create one pending SSH service transactionally. Unknown information reads Not recorded; never invent health, addresses or contacts.

Use existing type-scoped import preview, mapping, duplicate policy and transaction handling. Cancelling preview writes nothing. Bulk paste is future work, with confirmation rather than guessed dependencies. The prototype deliberately does not present fake working edit/import/document controls.

Each reminder explains why it appears, its recorded date and contact. Review opens context. Mark reviewed acknowledges an external manual check; it does not claim renewal or backup success. Tomorrow defers locally; reviewed/deferred items remain accessible and reopenable. Native state persists per resource and expiry cycle, with action time and explicit local actor semantics. Startup and foreground date refresh are required. Closed-app notifications are outside this delivery.

## Visual and interaction rules

- Reuse existing colors, official icons and system fonts. Dark mode has its own text, border and state tokens. No external font requests, decorative gradients or glass effects.
- Prototype text sizes 13/15/17px illustrate three discrete steps. Native implementation preserves existing size mapping and all themes. Size choices themselves show their corresponding size.
- Separate headings, supporting text, data and actions. Table headers remain fixed vertically, with shared horizontal geometry, visible row/column boundaries and local Fit columns.
- Keep Add asset dominant. Avoid rows of competing actions. Buttons have no underline. Names/details explain context before editing.
- Search/edit modals block background interaction and control keyboard focus. Search supports ordinary literal terms and clear empty/error states.
- Dates use YYYY-MM-DD; timed activity uses YYYY-MM-DD hh:mm:ss with an explicit timezone.
- Stable map type columns and directed links distinguish structural hosting from service dependencies. Visibility changes never alter stored resources/relations. Restore shows a count. Focusing a service preserves real hosts.
- The prototype has eight assets. Large maps require bounded visible nodes and measured rendering performance; this prototype proves neither scalability nor native frame rate.
- Smaller views allow table/map scrolling within the container. Lists provide an accessible alternative to the map.

## Native implementation and compatibility

Implement incrementally in Rust, Slint and local SQLite; do not embed HTML in a WebView. Reuse server_records, operational_inventory, operational_hosts, businesses, asset_context and resource_links with typed stable IDs. Do not confuse legacy planning assets or Work asset IDs with inventory IDs.

First add search/details, then overview, validated quick add, existing graph focus/hide/restore, persistent manual reminder state and navigation consolidation. Counts separate businesses from technical assets. Missing context is a suggestion, never a fault. Preserve Sites bookmarks, Work, Projects and Operations with clear reachable entries. Preserve credential encryption, reveal/copy and external password-manager references. Never display secrets in search, overview, activity or shared graphics.

## Prototype evidence and remaining validation

On 2026-11-05, Playwright CLI with local Edge checked cross-type search, modal background blocking, service host/contact/port details, focused host preservation, map hide/restore, service quick add and reverse host details, reminder postpone/reopen, dark large text, horizontal header alignment and demo reload reset. At 375/768/1024px there was no page-wide horizontal overflow; desktop screenshots used 1440px. Evidence: output/playwright/workspace-*.png. JavaScript syntax checks passed and no browser exceptions were captured.

These results do not validate Slint, migrations, installation, notifications or large-map performance. Native keyboard/screen-reader review, every dark-state contrast pair and actual user tasks still need evidence. Observe five target users without explaining menus: locate a website host, find a database owner, add a linked service and handle an expiry. Record success, elapsed time, mistakes and explanations; do not predeclare that users will like it.

## Skill adaptations

The ui-ux-pro-max searches included marketing Hero/CTA recommendations, which were not used for desktop information architecture. Only relevant hierarchy, low-decoration, focus, state and performance guidance was applied. Slint has no skill stack preset, so no unrelated framework implementation was imposed.

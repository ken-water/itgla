# ITGLA product architecture

Status: proposed
Date: 2026-10-15

## Positioning

ITGLA is a local-first assistant for people who operate software and infrastructure. It is not a CMDB, an ITIL system, a monitoring agent, or a secrets vault.

Its promise is simple:

> Before a change, know what it affects. During the work, know what remains. After the work, know what changed.

The product should feel like a calm, fast operational notebook that can reason about relationships when the user needs help. It should remain useful for one person with 10–100 machines before it grows toward team features.

## Information architecture

The primary navigation is organized around intent, not resource types:

```text
Today       What needs attention now?
Inventory   What exists and how is it connected?
Work        What am I planning or changing?
Certificates What will expire or need renewal?
Graph       What is connected to the selected thing?
Search      Where is the information I remember?
Settings    How should the workspace behave?
```

Servers, projects, domains, websites, services, and certificates remain resource types inside Inventory and Search. They should not become an ever-growing row of top-level tabs.

## Four mental models

### Resources

Resources describe things that exist:

- Server
- Service
- Website
- Domain
- Certificate
- Project

### Work

Work describes user intent:

- Change
- Task
- Renewal
- Incident
- Migration
- Launch
- Maintenance

### Impact

Impact is a computed and reviewable set of projects and resources affected by a Work item or selected resource. Every member should show why it is included:

- Linked directly
- Inferred through a relationship
- Added manually

### Signals

Signals explain what needs attention and always include a reason and a destination:

- certificate expires in 18 days
- work item has incomplete scope
- production server has no owner
- domain has no certificate relation
- service depends on an archived server

Tags are intentionally weaker than relationships. They are for filtering, grouping, and quick context; they must not be used as the only representation of operational dependencies.

## Core screens

### Today

The default screen is an action surface, not an analytics dashboard:

1. Needs attention
2. Active work
3. Upcoming deadlines and renewals
4. Recently changed

An empty workspace should provide one clear next action, such as “Add your first server” or “Create a change”. A populated workspace should answer “what should I do next?” without requiring the user to interpret charts.

### Inventory

Inventory keeps the current dense table strength. It adds:

- selection-driven detail panel
- type and tag filters
- relationship summary
- “create work from this resource” action
- bounded neighbor preview

The table remains optimized for dozens of rows. Cards are reserved for summaries and exceptions.

### Work

Work uses a list + detail layout:

```text
left:   work items, filters, status and priority
center: selected work details
right:  impact summary, omissions, checklist and timeline
```

The minimum Work fields are title, kind, status, priority, owner, start, due, notes, linked projects, linked resources, and checklist items.

The first complete workflow is:

```text
Create change -> select scope -> preview impact -> review omissions
-> generate checklist -> complete work -> close with summary
```

### Certificates

Certificate detail must show the complete operational surface:

- subject and SANs
- issuer and provider
- expiry and computed urgency
- renewal method
- protected domains
- deployed servers and services
- affected projects
- renewal plan action

“Create renewal plan” generates a checklist but never performs a destructive external change.

### Graph

The graph is contextual. It opens around the selected resource or Work item with a default depth of one and an optional depth of two. Modes are:

- Dependencies: what does this depend on?
- Impact: what would this affect?
- Timeline: what changed and what is upcoming?

Every graph view has an equivalent typed list. A graph is a navigation aid, not the only source of truth.

## Slint interaction system

The desktop advantage is density, immediacy, and predictable local behavior:

- one window for list, detail, and impact review
- dialogs only for import, destructive actions, and scope confirmation
- keyboard command palette (`Ctrl/Cmd+K`)
- `Ctrl/Cmd+N` for a new resource, `Ctrl/Cmd+Shift+C` for a change
- Enter opens the selected detail; Escape closes the panel; Space toggles a checklist item
- selected objects synchronize across table, graph, detail panel, and search results
- all asynchronous-looking states still expose loading, empty, success, and recoverable error states

Slint components should receive presentation models, not query SQLite. New business behavior belongs in application services and typed domain contracts.

## Domain boundary

The current single crate remains appropriate for now, but new code should follow this dependency direction:

```text
Slint views/presenters
        ↓
application services (work, impact, today, renewal)
        ↓
domain entities, IDs, enums, policies
        ↓
repository ports
        ↓
SQLite adapters and migrations
```

New critical states must be finite enums rather than arbitrary strings:

- `WorkKind`
- `WorkStatus`
- `RelationKind`
- `ScopeOrigin`
- `CertificateUrgency`

Use strong ID wrappers for new models. Keep database-specific IDs at the storage boundary.

## Persistence strategy

Migrations are additive and one release at a time.

### 0.3

```text
work_items
work_item_projects
work_item_resources
checklist_items
```

The 0.3.1 slice materializes `work_item_projects` and `work_item_servers`. Version 0.3.2 adds the additive `work_item_assets` link table and a bounded selector for websites, domains, certificates, servers, and services. Version 0.3.3 adds a deterministic one-hop relationship preview; inferred neighbors are visibly labeled and never silently persisted as direct scope. Version 0.3.4 adds single-item promotion and 0.3.5 adds transactional bulk promotion; both preserve the distinction between suggested and confirmed scope. Version 0.3.6 prevents an existing Work with unresolved inferred impact from being marked Ready. Version 0.3.7 groups the preview by resource category without changing persistence semantics. Version 0.3.8 adds an explicit one-hop/two-hop switch, with one hop as the safe default. Version 0.3.9 caps displayed inferred results at 50 and labels the capped state. Version 0.3.10 adds presentation filtering by resource category, and bulk promotion respects that active filter. Version 0.3.11 adds a compact count summary for the bounded preview. Version 0.3.12 persists depth and category filter preferences per Work as local UI state only. Version 0.3.13 adds advisory scope-health checks for missing, inconsistent, or unresolved context. Version 0.3.14 adds contextual recovery actions without automatic scope mutation. Version 0.3.15 replaces Work type and status free text with typed dropdowns. Version 0.3.16 adds in-session Work search plus typed kind/status filters. Version 0.3.17 adds composable sorting by operationally useful fields. Version 0.3.18 derives local urgency states for ISO dates while preserving flexible text windows. Version 0.3.19 adds typed quick status actions in the Work list. Version 0.3.20 exposes the persisted checklist as an editable execution surface. Version 0.3.21 adds ordered execution steps and prevents completion while any step remains unchecked.

The next usability slices keep the simplified local workspace focused. Version 0.3.28 adds managed Environment and Status options, defaults, and dropdown-based entry for Servers and Sites. Version 0.3.29 adds curated appearance/table preferences and an isolated local Demo workspace. Version 0.3.30 adds tag suggestions, configurable field definitions, import duplicate policy, and local backup controls.

#### 0.3.29 demo workspace contract

The Demo action is an activation tool, not a seed operation against the user's workspace.

- Show `Explore demo` in the empty-workspace action area and first-run Guide. Keep `Open demo workspace` available from Settings after the workspace contains real data; do not crowd the dense table toolbar with a permanent extra command.
- Open a separate local demo database. Never insert, update, hide, or delete records in the user's active database when entering, editing, resetting, or leaving Demo.
- Keep a persistent, visually distinct `Demo workspace` banner with `Reset demo` and `Exit demo` actions. Exiting restores the previous real workspace, selected tab, filters, and scroll state.
- Allow normal editing inside Demo so users can safely learn copy, sort, filter, resize, hide/restore, and field editing. `Reset demo` recreates the known fixture transactionally.
- Use a coherent fictional product story rather than unrelated sample rows: customer portal, API, worker, primary database, backup host, staging host, public website, status page, and internal admin site. Include Production/Staging/Development environments, Active/Maintenance/Attention/Offline statuses, useful tags, and a few custom fields such as Provider, Region, Project, and Owner.
- Keep Demo local-only and deterministic. It must not require an account, network access, analytics, or a production service.
- Demo records and banners must be clearly labelled as fictional and must never appear in export/backup operations for the real workspace.
- Cover enter, edit, reset, exit, database isolation, keyboard focus, 1000×640 layout, high-DPI layout, and repeated-entry behavior with automated tests.

Demo succeeds when a new user can understand within two minutes how ITGLA answers: what a machine is for, which environment it belongs to, whether it needs attention, and which sites or projects share its operational context.

Existing `operations` should be migrated or presented as a compatibility view, not silently discarded.

### 0.4

```text
certificate_details
certificate_domains
certificate_deployments
```

The existing `assets(kind='certificate')` record remains the stable identity while detail tables add structured fields.

### 0.6

```text
change_history
workspace_snapshots
```

The existing `server_records` and `assets(kind='server')` duplication must be resolved through an explicit mapping and validation period before either representation is removed.

## Release stages

### 0.3 — Work inbox and impact preview

Ship when a user can create a change in under one minute, select projects/resources, see scope counts and concrete members, review omissions, and generate a checklist.

### 0.4 — Today and certificate renewal

Ship when the default view identifies actionable attention items and a certificate protecting multiple domains/servers produces an editable renewal plan.

### 0.5 — Focused graph

Ship when graph, list, and detail selection remain synchronized, dense graphs stay bounded, and keyboard/list alternatives exist.

### 0.6 — History and safety

Ship when local edits have before/after history and restore operations remain schema-compatible and backed up.

### 0.7+ — Smart import and opt-in integrations

Ship parsers and integrations only after preview, confidence labels, cancellation, bounded input, and transactional commit exist. The local workspace remains complete without a network connection.

## Success measures

- A user creates a change in less than one minute.
- Before a high-risk change is marked Ready, the user can see every included project/resource and every omission warning.
- A certificate renewal plan is generated in one action and can be completed as a checklist.
- Today gives a useful next action in an empty and populated workspace.
- The product remains comfortable for 10–100 servers and does not feel like enterprise administration software.

## Explicit non-goals

- full CMDB or ITIL implementation
- agent monitoring and command execution
- mandatory cloud sync or team tenancy
- credentials, private keys, or secrets storage
- unbounded graph rendering
- automatic DNS, certificate, or server changes

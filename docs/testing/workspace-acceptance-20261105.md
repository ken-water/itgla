# ITGLA workspace acceptance matrix

Date: 2026-11-05. This matrix defines checks, not results. Record actual Pass/Pending/Blocked status in the implementation evidence. Unrun checks never count as Pass.

## Fixtures and evidence

Use only in-memory or unique temporary workspaces: empty, current schema21, isolated old fixture and synthetic large inventories. Never copy real credentials into screenshots. Include two businesses, identical IDs across types, shared/multi-host services, directed/cyclic links, archived assets, missing contacts/dates, long names and Unicode. Use unique credential sentinels to prove exclusion.

Measure1,000/10,000 resources and50/100/200 visible map nodes, recording hardware, build mode, DPI and time. Inject local calendar dates for overdue/today/tomorrow,30-day boundary, month end, leap day, year end and timezone/DST interpretation. Record candidate/platform/method/result/evidence per check.

| ID | Stage | Check | Acceptance | Layer |
| --- | --- | --- | --- | --- |
| WS-01 | S1 | Typed cross-type search | Four resource types, identical numeric IDs, shared services; unique kind+ID with no duplicate host copies. | Rust |
| WS-02 | S1 | Literal public-field search | Name/IP/IPv6/domain/purpose/tag/contact matching, stable ordering; quotes, % and _ are literal. | Rust |
| WS-03 | S1 | Secrets and archived resources | Hidden excluded by default; credential usernames/passwords and arbitrary secret columns absent from projection/logs. | Rust and log review |
| WS-04 | S1 | Debounce and latest input | Latest input wins; empty/cleared/no-result states explicit. | Slint |
| WS-05 | S1 | Search keyboard and modal boundary | Ctrl/Cmd+K, arrows, Enter, Escape; edits block search; focus restored. | Slint and native Windows |
| WS-06 | S1 | Details and Back | Correct hosts/ports/contacts/links/history; related traversal preserves underlying list state. | Rust and Slint |
| WS-07 | S1/S3 | Read-only and cancellation | Viewing writes nothing; cancel preserves source; validation errors preserve draft. | Transaction and Slint |
| WS-08 | S2 | Consistent source and counts | Search/directory/details/overview agree; technical assets and businesses counted separately. | Rust |
| WS-09 | S2 | Tables and local Fit | Header alignment under scroll/resize/all text sizes; independent view preferences. | Slint and screenshots |
| WS-10 | S2 | Empty and demo workspaces | Add/import reachable; fictional demo isolated from real database and visibly marked. | Rust and native |
| WS-11 | S3 | Validation and required defaults | Invalid address/port rejected; required defaults visible; unknown fields never fabricated. | Rust and Slint |
| WS-12 | S3 | New server and pending SSH | Exactly one pending SSH service; server/context/service roll back together. | Rust integration |
| WS-13 | S3 | Multi-host services | Forward/reverse details and map agree after save/restart; repeat submit does not duplicate. | Rust and native |
| WS-14 | S3 | Import/export/backup | Type-scoped entry; preview cancellation writes nothing; mapping failure atomic; full restore retains all data. | Isolated files |
| WS-15 | S4 | Structural/business paths | Domain-server-service direction and headings agree; required hosts retained; no invented links. | Rust and native |
| WS-16 | S4 | Cycles/shared nodes/expansion | Traversal bounded; expand/collapse preserves required hosts and correct counts. | Rust |
| WS-17 | S4 | Visibility versus archive | Map hide/restore changes no resources or stored relations. | Rust and Slint |
| WS-18 | S4 | Native graph performance | Measure rendered frames at50/100/200; refine over limit; no pointer-driven full rebuild. | Windows performance |
| WS-19 | S5 | Dates and manual tasks | Correct due/overdue sorting; unknown dates excluded; manual backup check never claims backup success. | Injected-date Rust |
| WS-20 | S5 | Review/defer/reopen/restart | Persistent state; original expiry unchanged; failed action atomic. | Rust integration and native |
| WS-21 | S5 | Expiry cycle changes | New date opens fresh cycle; old reviewed state cannot suppress it; no duplicate cycle. | Rust |
| WS-22 | S5 | Work remains authoritative | No checklist bypass, duplicated Work completion state or legacy/new resource ID confusion. | Rust and native |
| WS-23 | S5 | Calendar boundaries | Tomorrow uses local date across month/year/leap/DST boundaries; foreground refresh; no closed-app notification promise. | Injected-date Rust and native |
| WS-24 | S5 | Hide/delete/restore | No ghost reminder; hiding retains history; restore recalculates current cycle. | Rust |
| WS-25 | S5 | Migration/backup/failure/restore | Valid pre22 backup; schema transaction atomic on failure; full restore preserves reminders and relations. | Isolated schema21 fixture |
| WS-26 | S6 | Navigation compatibility | Stable page IDs, meaningful startup; Sites/Work/Projects/Operations reachable without deleting data. | Slint and upgrade |
| WS-27 | S6 | Appearance/save/cancel | All themes retained; three discrete sizes; consistent scaling; preview/save/cancel semantics. | Slint and native |
| WS-28 | S6 | Layout/DPI/keyboard/accessibility | Max size retains save/close; visible focus; text/control contrast meets targets. | Windows and accessibility tools |
| WS-29 | S1-S6 | Mutation and stale details | Unified refresh after edits; deleted selection unavailable; never edits a different resource. | Rust and Slint |
| WS-30 | S1-S6 | Existing safety/platform regression | Credential reveal/copy, protection, migration, tables, backup and connect behavior preserved. | Rust and platform review |
| WS-31 | Release | Actual target-user tasks | Five target users locate a server/contact, add a relation and handle a reminder; record time and explanations. | User observation |

## Execution

Meaningful Rust projection/reminder/transaction tests live beside the owning modules. Use Slint's official test backend for component and keyboard behavior; source-text assertions cannot substitute for interaction. The test runner workspace profile selects workspace and reminder module tests; quick alone does not cover this feature. Update runner tests when profiles change. Full commands are in the development plan.

Playwright checks only the HTML prototype or public site. It does not validate the Slint application. Resizing a window does not test system DPI; headless callbacks do not prove graph frame rate or installer upgrades.

## Stage exits

- S1: WS-01 through07, quality gates and search performance.
- S2: WS-08 through10 and earlier regressions.
- S3: WS-07,11 through14,29 and earlier regressions.
- S4: WS-15 through18 and business/host regressions.
- S5: WS-19 through25, isolated full backup/restore and full Rust suite.
- S6: WS-26 through30, native visual/keyboard review.
- Release candidate: all required stage checks, explicit platform gaps and WS-31, followed by authorized installer/release acceptance.

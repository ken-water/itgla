# Download detail delivery and bounded pagination — 2026-11-01

The customer's screenshot shows HTTP 200 headers but a detail response still transferring until the 15-second client timeout. Retrying another 200-row payload can repeat the same failure. HTTP 200 alone is not evidence that a body finished.

The new production timing records show the recent `/admin/api/download-events` origin requests sent approximately 74,397 bytes with request/upstream durations of 7–29 ms. They do not identify which downstream network/proxy/browser component stalled, but do not support database slowness for these samples. No speculative index or pre-aggregation change is made.

## Implementation

Add an authenticated `/admin/api/download-events-page` endpoint returning at most 25 records plus a continuation cursor. Preserve both prior endpoints for compatibility. Use deterministic `(occurred_at,id)` descending keyset pagination. The cursor is an exact bigint ID, not a JavaScript-rounded timestamp, so records sharing microsecond timestamps are neither skipped nor repeated. Validate cursor syntax/range before SQL and bind it as a parameter.

The UI loads the first batch and offers a compact Load more button. Batch errors retain loaded records and provide a local retry; other charts and metrics stay available. Refresh reloads the first batch, retaining existing same-period rows during network waits. Generation guards discard obsolete results, button state prevents duplicate concurrent batch clicks, and row IDs prevent duplicate appends.

## Evidence and release

- Read-only repeatable-read production comparison traversed all 1,302 matching records across 53 pages and matched the full ordered query exactly. Maximum page JSON size was 9,978 bytes, versus approximately 74 KB for the legacy 200-row response.
- Browser regression suite passed 18 tests. Subsequent focused interaction tests cover the final local-error presentation. Tests include failed second-page retention, retry without duplicate rows, refreshed first page, independently available funnel/charts, date formats, hover and session restoration.
- Production deployment reuses backup/rollback, database snapshot, syntax checks, Nginx validation, service health and authenticated smoke checks. Code changes require an analytics restart; schema and aggregation semantics are unchanged.
- Public browser acceptance verifies 25 rows initially, 50 after Load more, consistent metrics, chart hover and cache-disabled session reload. Temporary test sessions are removed.

Existing user authorization for corrective submit/push/publish and server access applies. This reduces transfer exposure; the exact downstream cause of the customer's intermittent stalled response remains unproven.

Evidence: `output/admin-delivery-logs.json`, `output/admin-page-contract.log`, `output/admin-page-tests.log`, `output/admin-page-final-tests.log`, `output/admin-page-deployment.log`, `output/admin-page-live-result.json`.

Production acceptance: backup `/root/itgla-admin-paged-20261101T075804Z`; all eleven authenticated report endpoints returned 200. The first page was 9,795 bytes and app processing 4.7 ms in the concurrent smoke run. Proxied browser acceptance confirmed 25 initial rows, 50 after Load more, correct visitor metrics, hover, full date format and authenticated reload, with no console/page errors. Temporary test session removed. Syntax/whitespace checks passed; full browser suite passed 18 tests and final local-error interaction suite passed 5 tests.

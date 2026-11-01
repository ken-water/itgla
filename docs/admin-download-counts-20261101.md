# Download request counts and unique visitors — 2026-11-01

The Downloads tab's headline metrics had become focused on unique visitors after correcting the conversion calculation. Existing request-count trend/country charts were not labeled clearly enough to distinguish them. Present both units explicitly.

- Request metrics: download page views, successful package requests, failed package requests, total package requests (success + failure).
- Visitor metrics: distinct download page visitors, distinct successful download visitors, distinct visitors with failed requests, and page visitors with no successful package request. A visitor can appear in multiple metrics.
- Request trend: download page views, successful requests and failed requests, with exact hover values and an accessible data table.
- Separate daily visitor trend: download page visitors and successful download visitors. Period unique visitors come from distinct visitor keys over the entire period, not summed daily uniques.
- Country tables and trends explicitly label successful/failed request counts.
- Replace duplicate funnel cards with a compact conversion explanation. Conversion uses page visitors who also had a successful response, calculated as page visitors minus visitors with no success. Successful visitors who used a direct package link need not have visited the download page, so successful-visitor totals alone are not a conversion numerator.

Repeated requests, retries and HTTP range requests count separately. Successful responses are not proof of completed file transfer or installation. Keep the existing meaningful-traffic filters, country definitions, authentication, safe timestamps and 25-record detail pagination.

## Verification and release

Browser tests distinguish 87 requests from 30 unique visitors, verify separate three-series request/two-series visitor charts, exact first-day counts (18 requests vs 10 visitors), and the visitor-based conversion explanation. Existing mobile/desktop, slow-network, pagination and session tests continue to apply.

A temporary-table PostgreSQL transaction exercises the actual candidate overview/daily/summary SQL with repeated visitors, legacy download events, failures and a probe. Expected results: 3 page views / 2 visitors, 3 successful requests / 1 visitor, 2 failures / 1 visitor. Production data is unchanged and the transaction is rolled back.

Deploy only current analytics code and web assets under the user's existing corrective release authorization, using the established code/config/database backup and rollback script. No schema, native installer, dependency or tracking changes. Verify both units and graphs over public HTTPS through the local proxy, and remove temporary test sessions.

Evidence: `output/admin-count-contract.log`, `output/admin-count-tests.log`, `output/admin-count-final-tests.log`, `output/admin-count-deployment.log`, `output/admin-count-live-result.json`.

Production acceptance: backup `/root/itgla-admin-counts-20261101T081106Z`. All eleven authenticated endpoint checks returned HTTP 200. Public browser verified separate request metrics (1,087 page views, 189 successful requests, 26 failures, 215 total package requests) and visitor metrics (433 page visitors, 119 successful visitors), three request-series and two visitor-series, exact header/funnel agreement, 25/50-row detail pagination and authenticated reload with zero console/page errors. Values are a point-in-time snapshot. Temporary test session removed. Full browser suite passed 19 tests, final interaction suite passed 6 tests, SQL contract test and syntax/whitespace checks passed.

# Admin reliability, chart interaction and date consistency — 2026-11-01

## Evidence and interpretation

The latest screenshot shows the download activity request exceeding the frontend's 15-second deadline while the download trend and summary metrics succeed. This is separate from the previously stalled Chart.js download. The recent timeout boundary changes an indefinitely pending request into a visible error; it does not establish that SQL consumed those 15 seconds.

Read-only production inspection found 6,463 events, of which 5,995 were meaningful, with approximately 9.6 MiB including indexes. Existing occurred_at, event_name/occurred_at and path/occurred_at indexes are present. EXPLAIN ANALYZE confirms use of the time and event-name indexes. Download detail SQL execution was 0.483 ms for 30 days; funnel summary execution was 3.632 ms. A 365-day window remained below 5 ms for the funnel. Overview/daily aggregations measured roughly 8–18 ms. No database lock wait was observed in the sampled activity. Host load was 0.02 and available RAM approximately 6.7 GiB.

Three origin-host samples returned the existing 74,415-byte download response in 13–132 ms over HTTPS; direct backend samples took 12–64 ms. These measurements do not reproduce or explain the customer's intermittent transport timeout. Previous logs lack full request/upstream durations, so a historical network/database attribution cannot be made honestly.

## Changes

- Separate download funnel summary from the detailed event list into authenticated `/admin/api/download-summary` and `/admin/api/download-events`. The summary is approximately 88 bytes. A stalled detail response cannot suppress the funnel. Keep the existing `/downloads` response for already-open pages.
- Retry an idempotent report read once after transport failure or 502/503/504. Do not retry login, logout, mutations, authorization failure or ordinary 4xx errors. Keep retries bounded and discard results from obsolete tabs.
- Include `Server-Timing: app;dur=...` on admin JSON responses. Record safe slow/aborted request timings and pool counts, without cookies, credentials, query strings or response content. Existing Nginx access records now include request and upstream durations for separating application time from delivery time.
- Smooth native SVG lines with monotone cubic segments that do not introduce extrema. Add immediate whole-plot pointer inspection, same-date values for all series, crosshair and highlighted points. Support touch and keyboard arrows/Home/End/Escape. Avoid any external chart dependency or animation delay. Keep exact accessible data tables.
- Display full `YYYY-MM-DD` dates in chart axes, tooltip and chart tables. Format event tables, latest data and feedback as `YYYY-MM-DD hh:mm:ss`, with zero padding and 00–23 hours. Preserve the existing browser-local time zone for timestamps; daily report buckets retain the server-provided dates. Reuse one timestamp formatter for feedback.

## Statistics strategy

No new index, materialized view, rollup table or cache is justified as the immediate remedy by these measurements. Daily UV values cannot be summed to obtain monthly unique visitors; download conversion requires matching visitors over the complete selected period. Premature rollups can change the meaning of current metrics.

Use the added timing evidence to distinguish application/pool delay, upstream delay and downstream transfer/client abort. If repeated production samples show expensive SQL or aggregate application latency, next consider bounded short-lived report caching with request coalescing, then purpose-specific partial indexes based on EXPLAIN. Add daily PV/download/error rollups only when measurements justify them, preserving exact range UV and conversion semantics. These are future decision criteria, not deployed behavior or a claim that the intermittent cause is resolved.

## Validation and operational scope

- Playwright: mobile/desktop chart and report flows, immediate hover anywhere in plot, touch/keyboard inspection, full date/time formats including midnight and feedback, blocked chart assets, valid-session cache-disabled reload, expired sessions, failed logout, blocked details with available funnel, bounded read retry and credential safety.
- Backend unit tests and JS syntax checks. Read-only repeatable-read production comparison verifies identical old/new summaries and event sets for 7/30/90/365-day windows.
- Deployment backs up code, web assets, Nginx config and database; validates syntax/config; restarts analytics and reloads Nginx; runs authenticated endpoint checks. Failure restores code/config and restarts service. Database restoration is deliberately not automatic, to avoid losing newly ingested events.
- Public browser acceptance through the user's local proxy verifies charts, hover values, date formats, downloads and session reload. Temporary test sessions are deleted.

Corrective production deployment follows the user's existing commit/push/publish and server authorization. No native installers, user workspace data, dependencies, schema or tracking policy changes are part of this fix.

Evidence: `output/admin-db-diagnose.jsonl`, `output/admin-report-contract.log`, `output/admin-hover-split-tests.log`, `output/admin-interaction-deployment.log`, `output/admin-interaction-live-result.json`.

## Additional correctness issue

The download header previously estimated “No download after visit” as distinct page visitors minus raw successful request count. These are different units and repeat downloads make the subtraction invalid. The header now uses the exact visitor-set funnel summary, the same source as the funnel cards, and explicitly labels successful download visitors. It remains independent of detailed event transfer. A regression assertion checks the known fixture's exact 124 non-converting visitors, including when the detail endpoint stalls.

## Production acceptance

- Main code/config/database backup: `/root/itgla-admin-interaction-20261101T074005Z`. Final metric correction web-asset backup: `/root/itgla-admin-native-session-20261101T074318Z`.
- Nginx configuration validation and service health passed. All ten authenticated report endpoints returned 200; app timings were 4.6–33.4 ms during the concurrent origin smoke run.
- Proxied, cache-disabled browser acceptance passed: immediate hover date and both series values, 200 download detail rows, exact header/funnel metric agreement, standardized dates, authenticated reload, no external chart request and no console/page errors. Browser samples saw approximately 370–570 ms to first byte, versus approximately 2–32 ms of reported application time; these are observations, not proof of the earlier timeout's cause.
- Final Playwright run: 17 passed in 12.0 s. Backend unit tests: 8 passed. Read-only old/new query contract comparison passed for all four periods. Syntax and whitespace checks passed. Temporary live-test sessions removed.

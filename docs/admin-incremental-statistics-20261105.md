# Incremental statistics and manual refresh — 2026-11-05

## Findings

The production importer had consumed the complete access log (offset 14,130 matched file size at inspection) and was updating its checkpoint. At 2026-11-05 08:04 Asia/Shanghai the newest meaningful website event was 07:54; later records were admin API reads, which intentionally do not count as public website visits. The admin page also lacked automatic refresh. Latest event time must be distinguished from statistics computation time.

## Behavior

The existing analytics service now includes a background statistics task running every 60 seconds without any browser needing to be open. It imports pending access logs, refreshes daily facts and publishes 32 saved reports (eight reports across 7/30/90/365-day windows). Requests read saved JSON snapshots rather than performing all aggregations on demand. Publishing all snapshot rows uses one transaction; failed computation keeps the prior successful version.

The first run initializes historical daily facts once. Normal runs refresh today's UTC date only. PostgreSQL event-change triggers persist dirty dates for late records, classification/country correction and deletion; those dates are additionally repaired on the next run. Restart classification/privacy maintenance avoids no-op updates that would otherwise dirty every historical day.

Daily facts retain weighted request counts and anonymous visitor keys. Request/PV counts sum weights; UV and conversion remain exact distinct visitor/set operations across the full range. Counts are not reconstructed by summing daily unique visitors. To preserve the existing rolling "last N days" semantics, reports use saved complete-day facts plus raw records from today's partial day and the first partial boundary day. This is deliberately not a claim that every operation touches exactly one source date. Detailed event tables remain indexed live reads with existing 25-record pagination.

Manual Refresh calls authenticated same-origin POST `/admin/api/refresh`, joins any running computation instead of spawning duplicate jobs, then reloads current reports. Button shows Updating and prevents duplicate clicks. Background page checks read existing snapshots every minute only while visible, and recheck when returning to an older page. The header shows Latest event and Statistics updated with full dates/times. If the saved snapshot is older than two minutes, it explicitly reports delayed updates rather than presenting the fetch time as a new computation time.

The normal target is a new server snapshot each minute. A visible client checking once per minute can display an older snapshot until its next check (up to roughly two scheduling intervals plus execution/network time). Do not promise a hard end-to-end 60-second guarantee under service or network failures. Manual Refresh requests an immediate computation and reload. Existing source logs are still ingested every ten seconds, with single-flight ingestion to prevent overlapping checkpoint writes.

## Data and operations

Additive tables: analytics_daily_facts, analytics_dirty_days, analytics_rollup_state, analytics_report_snapshots. Trigger marks affected dates. UTC day grouping preserves existing report bucket semantics; displayed timestamps retain browser-local format. No new personal data; daily facts retain only existing anonymous visitor keys and country/path/event aggregate dimensions. Retention deletions mark dates dirty so daily facts are cleared in the next run. Snapshots are overwritten and do not accumulate old versions.

Deploy with database dump, old code/schema/web backups and rollback. Rollback restores old code/schema files while retaining additive tables for recovery; do not restore the whole database and lose newer events. The old app ignores additive tables and functions. Existing authenticated endpoint and public health checks run after restart.

## Validation

- Isolated PostgreSQL test schema cloned existing event data, then rolled up 44 dates. The next pass refreshed only 2026-11-05. A simulated historical correction refreshed only 2026-11-02 and today. All 24 baseline-vs-rollup report/window comparisons matched. Snapshot publication wrote 32 rows with one computation timestamp. The isolated schema was removed; customer event data was unchanged.
- 21 browser tests passed, including automatic minute checks, real manual POST, full timestamp formats, count-vs-visitor separation, pagination retention, stalled requests and authenticated reload.
- Eight backend unit tests and syntax/whitespace checks passed. Live HTTPS acceptance and a subsequent timer-driven snapshot generation are checked separately.

Evidence: output/admin-snapshot-sql-tests.log, output/admin-snapshot-tests.log, output/admin-snapshot-deployment.log, output/admin-snapshot-live-result.json, output/admin-snapshot-schedule.log.

## Production acceptance

Deployment backup: `/root/itgla-admin-snapshots-20261105T042614Z`; final browser timer adjustment backup: `/root/itgla-admin-freshness-20261105T043253Z`. All eleven authenticated report checks returned 200, with snapshot reads approximately 2–5 ms of application time. All 32 snapshots had one publication version.

Public browser acceptance verified that manual POST refresh advances the saved computation timestamp, both freshness labels display correctly and a reload retains authentication. A separate monitor with no active browser or manual requests observed the background version advance from `2026-11-05T04:28:15.572Z` to `2026-11-05T04:29:15.565Z` (UTC). Service logs confirm ordinary scheduled passes refreshed only `2026-11-05`. Temporary sessions were removed.

The persisted optional integration test `server/analytics/snapshots.integration.test.mjs` uses `ITGLA_TEST_DATABASE_URL`, creates an isolated random schema with synthetic repeat visitors, and removes it afterward. It compares 24 aggregate/window cases, tests late-date repair, atomic publication, retention of the previous snapshot on simulated publication failure, and coalescing of concurrent refreshes. It passed on PostgreSQL. Run with an explicit test database URL and `node --test server/analytics/snapshots.integration.test.mjs`; it skips when no URL is configured. Normal backend unit tests passed (8); the browser suite passed (21); after the timer alignment adjustment, all 8 affected interaction tests passed again.

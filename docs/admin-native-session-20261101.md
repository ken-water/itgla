# Admin charts and refresh session recovery — 2026-11-01

## Observed failure and scope

The customer's browser showed `/admin/chart.umd.min.js` pending at zero bytes while analytics JSON and tables completed. The previous recovery waited indefinitely when that import never settled. A successful fast-network test did not cover this case.

The initial HTML also displayed the sign-in form before checking the existing session. The session bootstrap treated any `/me` network error as a logged-out session. Neither symptom proves the server deleted the cookie; the backend retains an eight-hour session in PostgreSQL.

## Changes

- Render report lines, axes, country legends and point titles directly as native SVG in the existing module. No Chart.js request, dependency or animation is required. Collapsible chart-data tables expose exact values and support keyboard readers.
- Initially show session restoration, keeping sign-in hidden. A successful authenticated check opens the dashboard. Explicit unauthenticated/401 responses show sign-in. Transport errors, malformed responses and timeouts offer session-check retry without removing cookies or requesting credentials.
- Keep existing HttpOnly, Secure, SameSite=Strict cookies, server authorization, eight-hour expiry and CSP. Credentials are never saved in browser storage or URLs.
- Report logout failure accurately. Successful logout clears the session; fixture cookie handling now matches this behavior. Feedback code loads only when its tab is opened.

## Validation and release

Targeted Playwright tests cover mobile/desktop reports, blocked chart resources, independently stalled analytics, exact chart values, cache-disabled authenticated reloads, timed-out session restoration and retry, expired sessions, logout success/failure and credential safety. The fixture applies production CSP to catch unsupported inline styling.

Static web assets require no compilation; JavaScript syntax and diff checks apply. No native installer, schema, analytics aggregation or server authentication policy changes.

Authorized scope: corrective deployment under the user's existing submit/push/publish and production-server authorization. Deploy only admin.html/admin.js/admin.css, backing up all three and rolling back on failed authenticated smoke checks. Keep the analytics service and databases running. Verify existing service health, real HTTPS API responses, browser charts, cache-disabled reload and absence of chart-library requests through the user's proxy. Temporary live-test sessions are separately removed.

Evidence: `output/admin-native-tests.log`, `output/admin-native-deployment.log`, `output/admin-native-live-result.json`, `output/playwright/admin-native-live.png`. A local browser reload cannot prove the customer's browser extensions or cookie policy; server-side expired/revoked sessions still correctly require sign-in.
## Observed production acceptance

- Deployment backup: `/root/itgla-admin-native-session-20261101T071918Z`.
- All eight authenticated analytics endpoints returned HTTP 200 (128–457 ms in this smoke run). Nginx and analytics remained active.
- Proxied Chromium with cache disabled and the Chart.js route permanently blocked rendered two overview series (62 points total) and six country series. Reload retained the authenticated dashboard. No Chart.js requests or browser errors occurred.
- All 13 targeted Playwright tests passed in 16.7 seconds. Temporary production test session removed after verification.

# Admin charts: slow load and retry recovery — 2026-11-01

Production reports loaded but charts displayed unavailable. The 10-second loader timer settled the shared promise as failed even while its module download continued. A later successful transfer initialized Chart.js but did not redraw the current chart. A failed import could also remain cached by the browser, making an identical-URL retry fail again.

The loader now observes the actual transfer until it succeeds/fails. After 10 seconds each chart displays a still-loading notice; late success redraws automatically. Tables and metrics remain independent. Actual failed transfers show an unavailable state and Refresh uses a new retry URL. Existing generation guards prevent a late chart from drawing over a different tab. The table/data load does not wait for chart completion.

The production Chart.js response was 208,341 bytes with no gzip content encoding despite an Accept-Encoding:gzip request. It is served as text/javascript, which was absent from the existing gzip MIME list. Add gzip only in the static /admin/ asset location. The separate /admin/api/ location retains its existing compression settings; authenticated JSON/cookies are not newly compressed. Compression changes transport only, not the library bytes.

Validation: 11 Chromium tests passed in 11.2 seconds, including actual canvas/Chart instance checks, a delayed library that arrives after the notice, successful automatic repaint, failed-import retry, mobile/desktop tab charts, partial data failures, and sign-in safety. JavaScript syntax and scoped whitespace checks passed. No desktop/schema/dependency changes.

Deploy only admin.js and the ITGLA virtual-host configuration, preserving both exact originals under /root/itgla-chart-recovery-*. Check nginx -t before reload. Verify gzip response/decompression equality, current script marker, authenticated reporting endpoints and unrelated VeloWrite health. Restore the two originals and reload Nginx on failure. Keep credentials/databases and historical installation packages unchanged. Existing user publication authorization covers this corrective update.

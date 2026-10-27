# Appearance optimization preview ? 2026-10-26

Implemented in order: full-width inventory workspace with on-demand editors; compact navigation/actions and pinned headers; appearance controls beside a live preview; website copy and real Windows preview screenshots.

New/Edit opens the editor. Closing preserves the current fields. Tag filtering is available on demand. Sites now use a scrolling editor with a pinned save action. The existing five text sizes, density options and themes remain immediately previewable; persistence uses Save settings.

Validation: cargo +1.92.0 build --locked; cargo fmt check; 83 binary tests passed; scripts/audit-ui.py passed 22 UI contracts, five modal bounds, six contrast pairs. Native screenshots reviewed for Services, Servers, Settings and largest-font dark server editor. Playwright checked 1440/768/390 widths: all images loaded, no page errors or horizontal overflow, button height checks passed.

Evidence images are in the Windows temporary directory (itgla-*-20261026.png); website browser evidence is in TEMP/itgla-web-20261026. Committed website assets contain fictional data only. New screenshots describe the upcoming desktop update; current public downloads remain unchanged.

Scope: local preview only. This does not establish complete accessibility conformance or cross-platform visual parity. Existing dead-code/unused-component compiler warnings remain. Earlier uncommitted functional work was preserved.

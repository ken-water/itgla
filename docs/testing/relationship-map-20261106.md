# Relationship map and website screenshot review

Date: 2026-11-06. Scope: local native candidate and local website only.

## Changes

- Retained deterministic business/domain/server/service lanes and existing repository relationship semantics.
- Added categorical node borders and accents, relationship colors, a visible legend and selected-path emphasis. Light and dark colors are separate.
- Reduced row spacing from 96px to 84px and moved cross-lane tracks inside the remaining gap. Route tests still verify that links do not cross node interiors.
- Added text filtering by type, name, IP and service protocol/port. Search covers all workspace resources, including those outside the current business or 200-node rendering budget.
- Search navigation explicitly restores the view and focuses the chosen asset with its connections. Existing collapse, hide, restore and one-level expansion remain available. These operations do not modify asset records.
- Replaced the homepage connection image and old asset-details image with native software-rendered screenshots of isolated fictional demo data. New filenames avoid stale image-cache reuse.

## Verification

- `cargo +1.92.0 fmt --all -- --check`: passed.
- `cargo +1.92.0 --config output/workspace-cargo-network.toml test --offline --locked -j 2 topology::tests:: -- --nocapture`: 6 passed.
- Scale fixture adds 1,024 domains to the 16-resource demo. The overview remains bounded to 200 nodes. Searching `scale-1023` finds the last domain and navigation displays that requested domain.
- `target/debug/deps/itgla-ff2cb78eade3d856.exe connection_headers_follow_real_canvas_on_resize_font_changes_and_horizontal_scroll --nocapture`: passed.
- `npx playwright test tests/site.spec.js --grep 'homepage is usable' --workers=1`: 3 passed, covering 390px, 768px and 1440px widths.
- `node scripts/check-english-ui.mjs`: passed.
- Browser inspection confirmed the new connection image is loaded at 1400 × 1100 and the new details image at 1400 × 900. Desktop document overflow was 0px.
- Native captures were inspected in light mode and dark mode with the largest application font. Evidence is under `output/relationship-review/` and `output/playwright/home-map-20261106.png`.

## Limits and follow-up

- This is a native Slint diagram, not an embedded D3.js renderer. The scale fixture verifies searchable access and bounded rendering; it does not establish frame rate or usability for a dense thousand-node diagram.
- Full-window screenshot generation uses the freshly linked `target/debug/deps/itgla.exe` copied to `output/relationship-review/itgla.exe`. Cargo could not replace `target/debug/itgla.exe` because an existing user preview was running; that window was preserved.
- Clippy did not complete: its subprocess selected the global TUNA registry and offline resolution found calamine 0.36.1 instead of locked 0.38.0. No dependency or global Cargo configuration was changed.
- Website changes have not been deployed, committed or pushed during this work.

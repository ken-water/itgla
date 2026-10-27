# Connections interaction review ? 2026-10-26

Local Windows desktop refinement, using ui-ux-pro-max interaction/accessibility guidance. Native stack: Rust + Slint 1.18.1 software renderer. No database migration, added dependency or publication in this refinement.

## Delivered behavior

- Click or activate a node to select it and trace incident relationships. A separate Edit record action opens the existing validated editor. Full selected name/detail appears below the map.
- Drag empty canvas to pan. Top, Page up and Page down provide alternatives to dragging. Lane headings remain outside the scrolling canvas.
- Locate resource dropdown jumps to a visible resource. Business filter still limits the graph. Selection and locator reset on model refresh to avoid stale IDs across datasets.
- Relationship controls appear only after Add link or choosing a listed relationship. From/Relationship/To have explicit labels. The Relationships list follows the selected resource.
- Each node side distributes incoming and outgoing connections across distinct ports. Gutter tracks differ per edge. Links spanning multiple columns travel through empty row gaps, avoiding unrelated node interiors. Arrows retain the stored source-to-target direction, including reverse and same-column relationships.
- Replace canvas-sized Paths with narrow rectangular segments and small arrowheads; cull offscreen nodes/edges and remove global hover-driven invalidations. Refresh caches labels rather than issuing two name queries per edge. Unselected lines use muted text ink for legibility.

## Validation and limits

The existing graph/UI integration test now exercises actual accessible selection and explicit edit navigation, locator model correspondence, and page controls. All demo edge segments are checked against every node interior; node attachment points must be unique. Existing demo reset, graph/table synchronization, relationship validation and history tests remain in the suite.

Native OS pointer check: 40 moves dragging the empty gutter, Windows 125% scaling. Verified viewport moved while lane headings stayed fixed, no resource was selected and no editor opened. Dark largest font and the disclosed relationship form were inspected. Test data uses the isolated in-memory demo.

Performance workload: scripts/measure-graph-preview.ps1, 160 synchronous wheel messages, alternating direction every 20, 12 ms requested gaps, same 16-resource Demo, 1400 ? 950 physical window, debug software renderer. Results are single-run CPU and message-dispatch measurements; they are not FPS, frame timing, a release-build benchmark, or a 200-resource stress test. See connection-map-interaction-performance.json for the raw before/after results. Timing varies with machine load and selection state.

The 200-resource display cap and business filtering remain. This is a structured lane map with panning; free node rearrangement, zoom and a minimap are future work. Dense graphs can still have line crossings; unique ports/tracks prevent the shared endpoint pattern reported in the screenshot.

No commit, push or deployment was requested for this refinement. Broad working-tree governance accounting includes earlier pending changes; its budget/path results must not be confused with functional test results.

Final validation: fmt, UI audit, native debug build, strict Clippy, all-target check and all 85 tests passed. Broad-tree library governance remains blocked by earlier pending migrations/dependency/UI paths and cumulative L3 budget; functional passes are not a release approval.

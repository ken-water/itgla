# Connection map interaction/performance refinement - 2026-10-26

Scope: ui/topology.slint and bounded topology model construction; preserve pending workspace work. No schema, dependency or release changes.

Plan: replace canvas-sized edge paths with narrow orthogonal segments and small arrowheads; avoid global hover-driven edge invalidation; allow explicit node selection before editing; keep lane headings fixed, offer drag panning plus page/top buttons; disclose relationship form only on demand with visible field labels; show selected resource links and resource dropdown lookup. Cache labels during refresh instead of per-edge SQL. Give every incident edge a distinct node port and gutter track; route links that skip columns through empty row gaps. Use muted text ink for unselected lines so they remain visible. Keep graph and table mutations on the existing validated repository callbacks.

Acceptance: selection stays on Connections, Edit opens correct record; mouse drag does not open editors; viewport operations never refresh repository data; arrow endpoints remain aligned; demo geometry checks reject duplicate ports and segments crossing any node interior; keyboard/single-pointer alternatives exist; normal/dark largest-font screens fit supported desktop bounds. Compare identical native wheel workloads before/after, reporting CPU and message-dispatch measurements without claiming FPS. Test UI selection/editor navigation and existing graph/table synchronization.

Failure/rollback: keep DB and migration unchanged, leave legacy relationship callbacks intact. Restore only this refinement's UI/model deltas if needed; retain earlier dirty work. Data is isolated in the in-memory demo during checks.

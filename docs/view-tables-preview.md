# Graph views and inventory tables — local preview

Implemented:
- Connections: select a resource, focus its neighbours with structural ancestors, collapse/expand a branch, temporarily hide and restore a resource, or Show all. These are session-only display controls. Existing business filters still apply.
- Shared services stay visible when another hosting server remains expanded.
- Services and Domains: separate field columns, fixed header, sort, resize, local Fit columns and hosting-server details including the latest 100 recorded events. Passwords are excluded from display models.
- Businesses: searchable compact table with a detail editor.
- Work: table/list switch. Opening a row preserves the existing work editor.

Validation: 98 Rust tests passed, including graph recovery, preserved relationships, shared hosting and independent Fit columns. Format and English-only UI checks passed. Clippy completed with denied lint warnings; Windows reported incremental-cache finalization warnings. Optimized Windows build completed. Native inspection covered Services and Domains in light theme, earlier Services/Connections in dark theme, and the hosting-server detail action showing server notes and recorded history. The native inspection found wrapping/clipped host links; these were changed to single-line ellipsis and rechecked in the final release binary. Largest-text and every interactive combination have not been exhaustively inspected.

Limits: no multi-select, saved graph views or persistent visibility preferences in this iteration. Graph still has the existing 200-node rendering limit. A hidden resource and an archived record are different operations. Visual inspection is not a usability study.

Preview: `D:/devops/itgla/dist/view-tables-final/ITGLA.exe`

SHA256: `551cc553b2bb139a6c729dae6fe94b34b65dee05a9ac6f85e548a0f2bfb11e0b`

Final isolated launch: `test-results/acceptance-20261030T163254Z-5b5f1f85/launch.json`. Demo mode is enabled for local inspection. Test/build/clippy logs: `output/view-tables-{tests,build,clippy}.log`.

Governance aggregate check did not pass: it evaluates the entire existing dirty worktree (including website, migrations and generated artifacts) against this local feature scope. Details: output/view-tables-governance.log. Individual format, build, clippy and test checks passed; no release approval is inferred.

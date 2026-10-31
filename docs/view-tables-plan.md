# Scoped graph views and consistent inventory tables

Local implementation, no schema or dependency changes. Graph visibility is workspace-session UI state and must never archive assets or mutate links. Focus includes direct neighbours and their structural ancestors; collapse hides owned descendants without hiding shared services attached to an expanded host. All visibility restrictions have explicit recovery controls. Limits are applied after filtering, with accurate visible/total counts.

Use one compact, keyboard-operable grid for services, domains, businesses and optional work table view: fixed headers, sorting, draggable columns, fit-to-window, independent view state, distinct cells and a small row action. Keep secret values out of grid models. Host details remain reachable from service/domain rows. Preserve existing search, editing, restoration and work filters.

Acceptance: demo graph hide/restore, focus/reset, collapse/expand including shared hosts; models reflect separate inventory fields; grid sorting works after refresh; format, check, clippy, Rust tests; native light/dark/largest-text screenshots of graph and grids. Rollback is restoring only this change's hunks; previous uncommitted work is preserved. Optimized local preview is isolated from user data.

# ITGLA P0-P2 UI and accessibility report

Date: 2026-10-17

## Outcome

Provisional product UI score: **86/100**, up from **67/100**.

The score is provisional because this Linux build environment cannot execute a physical Windows Narrator/NVDA session or inspect native ClearType rendering. Automated Windows-target, DPI-event, keyboard, accessibility-tree, visual, interaction, and package checks passed.

| Area | Score | Evidence |
|---|---:|---|
| Typography and cross-tab consistency | 19/20 | Semantic caption/body/control/title tokens; standard widgets inherit `default-font-size`; no functional 8px/9px text. |
| Hierarchy, density, and first-use clarity | 17/20 | Shared headers/labels, readable A+ default, compact tables, fixed Work title/search/filter region. |
| Interaction states and feedback | 17/20 | Visible custom focus, bidirectional sort state, keyboard equivalents, explicit Saved/Save failed messaging. |
| Keyboard and screen-reader readiness | 17/20 | Forward/reverse tab activation tests plus role/name/action checks for tabs, cells, headers, filters, rows, and scope choices. |
| Platform and release validation | 16/20 | 125/150/200% DPI simulation, three viewport captures, Windows cross-build, installer/package audit; physical Windows sign-off remains. |

## Automated validation

- 47 Rust tests passed.
- Formatting, compile, Clippy, advisories, coverage, English-only UI, release build, and whitespace gates passed.
- 22 static UI contracts, five modal bounds, nine contrast pairs, and desktop target-size checks passed.
- Native pointer, text editing, save, column fit/drag, font scaling, guide, import, tab, scroll, hide, restore, and persistence checks passed.
- Visual captures passed at 1000×640, 1240×800, and 1440×900.
- Windows installer and ZIP plus Linux DEB/RPM/AppImage package audits passed.

Full report: `test-results/20261017T154219Z/report.md`.

## Physical Windows sign-off still required

Run this matrix on a physical Windows 11 device before calling the platform validation final:

1. At 125%, 150%, and 200% display scaling, inspect Servers, Projects, Operations, Work, Guide, Import, Hide, and Edit Cell at 1000×640 and a maximized window. Confirm no clipped text, overlapping controls, lost columns, or blurry app icon.
2. Starting from the Servers tab, use only Tab, Shift+Tab, Enter, Space, arrow keys, Escape, and standard text-edit shortcuts. Confirm every visible action is reachable, focus never disappears, dialogs do not leak focus, and Work opens with its title/search/filters visible.
3. With Narrator and NVDA separately, confirm names, roles, values, selected/checked state, sort direction, save results, and dialog context. Pay particular attention to table cells (Space copies, Enter edits), table headers, tags, Work rows, and scope choices.
4. Install, upgrade over the current version, launch from the desktop shortcut, uninstall, and verify Windows security prompts and native font rendering.


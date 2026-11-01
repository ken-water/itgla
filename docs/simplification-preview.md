# Simplification preview — 2026-10-31

Audience: IT staff and independent developers looking up an existing resource or recording their first server.

Keep the existing field-atlas visual language, three text sizes, table boundaries and keyboard navigation. Reduce simultaneous choices rather than deleting stored information. No schema, dependency, credential protection, scanning, or release changes.

Implement in order:
1. Services/domains open a read-only detail panel. Editing is explicit; cancelling reloads the selected record. Copy and host details remain available without editing.
2. Fit tables using content-informed preferred widths. Show core inventory columns initially; allow additional columns locally without changing other tables.
3. Keep basic service/host fields first. Disclose optional context, account editing and relationship editing in named groups. Shorten the first-run guide to the first useful task.
4. Group navigation by inventory, relationships and bookmarks. Make graph tools contextual and distinguish hiding from the current diagram from hiding an asset record.
5. Explain one lookup journey on the homepage, show a readable mobile crop and a connection-map example, fold extended capabilities and handover scenarios, and move the optional website account away from the homepage primary navigation.

Acceptance: opening details exposes no editable record fields or save action; edit/cancel cannot silently persist changes; new records retain basic fields and explicit save; hidden optional sections retain values; primary columns and headers align at all three text sizes; graph focus/hide/recovery remains usable; website has no horizontal page overflow at 390/768/1440px and retains download, privacy, platform and manual-inventory disclosures.

Validate using focused Slint headless interaction tests, the Rust suite and strict Clippy, website browser regressions, and local visual evidence. Use isolated fictional workspaces. A local preview does not establish installer lifecycle, real-user usability or production readiness. Revert the scoped source changes to roll back; existing databases remain compatible.


Implemented review notes:
- Services and domains now open read-only details with explicit Edit record. Optional context, account editing and relationships stay collapsed until requested; cancelling an existing edit reloads persisted values.
- Primary inventory columns use bounded content-informed preferences. More columns is local to the inventory view. All table headers retain visible boundaries.
- Navigation is grouped; the first-run guide focuses on adding a server; graph tools are disclosed when useful. Existing graph focus/hide/recovery logic is unchanged.
- The homepage explains one lookup journey with actual Windows preview captures. Extended inventory, handover and spreadsheet explanations use native keyboard-operable disclosures.
- Sites and Servers both had an inverted horizontal header offset: ScrollView content-x is already negative when scrolling right. Both headers now use the same signed offset as the rows. Services, Domains, Businesses and Work use the shared DataTable offset binding. Regression coverage exercises horizontal movement, vertical header stability and Fit columns reset.

Visual evidence: output/simplification-website-desktop.png, output/simplification-website-tablet.png, output/simplification-website-mobile.png; native captures in the system temporary directory named itgla-simplification-services.png, itgla-simplification-details.png and itgla-simplification-details-dark.png. Public website WebPs contain only native application pixels and a plain surrounding canvas. The mobile image is explicitly a table-detail crop.

Limitations: headless Slint interactions are not equivalent to a real Windows pointer/installer lifecycle test. Native graph combo-box automation could not reliably select items in this environment, so no manual graph hide/recovery success is claimed. No user study, Linux/macOS visual validation or installer builds are included in this local preview.


Final local validation (2026-10-31):
- Rust workspace/all-features suite: 104 passed, 0 failed. Includes actual horizontal scroll regression checks for Sites/Servers at three font sizes and shared Services/Domains/Businesses/Work table checks after column resizing.
- cargo +1.92.0 fmt --all -- --check; strict Clippy across all targets/features; cargo check across all targets/features; Windows debug build; git diff --check: passed. Existing Slint unused-component and deprecated Cargo-config build warnings remain.
- Website Playwright suite: 18 passed. English UI and shared footer checks passed. Native details panels were visually reviewed at normal and maximum text size/dark appearance; homepage screenshots reviewed at 390, 768 and 1440px. Native HTML disclosure keyboard open/close check passed.
- Fresh isolated fictional Windows preview opened on Sites, with UI Automation confirming its controls. A fresh screenshot returned the Windows lock screen, so it is excluded from visual evidence. Scroll assertions use the Slint interaction backend; no manual pointer validation of the final build is claimed.
- Preview executable: D:/devops/itgla/dist/simplification-preview/ITGLA.exe (debug build, no installer). SHA256: 0c4250f0a18844dd415a86e90db1bb2bcebf07ca8e7b2967d2a3a30fc81fff33. Local website: http://127.0.0.1:4183/ .

Governance evidence records the scoped source/doc/image changes. UI governance covers interface and website validation. Rust governance checks the Rust sources and preview document only: its fixed filesystem allowlist does not support ui/ or website/, so an initial combined-language check was rejected and retained as output/simplification-rust-combined-governance.json. Complete task paths and hashes remain in the source manifest. These checks use explicitly declared paths, not a full-repository audit; preexisting untracked output, .agents and .playwright-cli directories are excluded from this task's diff. No commit, push, deployment, version bump or installer build was performed in this preview round.

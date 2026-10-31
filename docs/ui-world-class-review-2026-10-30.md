# ITGLA UI review and improvement design

Date: 2026-10-30. Status: design proposal, not implemented product acceptance.

## Conclusion and scoring method

The desktop is a useful inventory with a recognizable identity, but it does not yet deliver an effortless overview at large text sizes. The website communicates the proposition more clearly than the desktop delivers it visually. Prioritize readable resource context and predictable actions before adding visual effects.

**Desktop: 6.1/10. Website: 7.1/10.** These are rounded, subjective design-review scores, not usability measurements, accessibility certification, independent critic results, or release readiness. No score is assigned to an unbuilt improvement. The website score covers the local homepage and the reviewed supporting pages, not every public route. The desktop assessment is particularly informed by the demanding largest-text/narrow-window state; it is not an average over every theme and size.

| Dimension | Weight | Desktop /10 | Website /10 | Reason for deduction |
|---|---:|---:|---:|---|
| Task hierarchy | 15% | 6 | 7 | Desktop navigation and toolbars compete with records; website repeats related promises across many sections. |
| Information comprehension | 15% | 6 | 7 | Important host/protocol/context values truncate; homepage product proof does not show the full current business chain. |
| Typography | 10% | 6 | 7 | Five text sizes exist, but largest size strains layout; website screenshots and small captions are hard to read on phones. |
| Color and brand | 10% | 7 | 8 | Consistent green identity; desktop borders and equally prominent actions create noise. |
| Spacing and composition | 10% | 6 | 7 | Large desktop chrome reduces useful viewport; website has a long, repetitive vertical rhythm. |
| Adaptive layout | 10% | 5 | 7 | Settings navigation reaches the edge at 1100x720; graph service column needs horizontal navigation. |
| Interaction clarity | 10% | 6 | 7 | Graph controls need interpretation; appearance preview/persistence needs a more explicit transaction. |
| Accessibility evidence | 10% | 6 | 7 | Keyboard, screen reader, DPI and full size/theme matrix remain incomplete; do not infer compliance from screenshots. |
| Distinctiveness | 5% | 6 | 6 | The field-atlas direction is credible but needs a resource-chain identity beyond standard hero/screenshot sections. |
| Detail and consistency | 5% | 7 | 7 | Existing shared controls are a strength; screenshot versions and public current-version wording need alignment. |

Weighted totals before rounding: desktop 6.05; website 7.05. The rubric is defined here for this project; the PDF does not prescribe these weights. Scores below 5 indicate a task-breaking design, 5-6 a usable design with friction, 7-8 a coherent design with gaps, and 9-10 require strong craft plus repeated task and accessibility evidence. Aesthetic scores never override release blockers.

## Evidence and PDF interpretation

Source: the user's world-class UI PDF in `C:/Users/dell/Downloads/`, 32 image-based pages. The requested title differs slightly from the actual filename. All pages were reviewed as rendered images; no extractable text layer was available. Rendered evidence is under `test-results/ui-review-20261030/`. An initial wildcard matched a different four-page document; those intermediate images were replaced using the exact requested file before this review.

Applicable ideas:

- Pages 1-9: Discover, Define, Deliver; choose an explicit visual world and bring human judgment into selection. Three directions considered: an operations console, an editorial asset atlas, and a handover dossier. Select the **asset atlas** for continuity with the accepted green identity and everyday lookup; borrow the dossier's responsibility/history treatment in details. A control-console emphasis would overstate execution/monitoring capabilities.
- Pages 10-14: judge screenshots against a brief and quality references; distinguish implementer and critic. This review is **not** a separate independent-agent critique. Future independent review should receive only screenshots, the brief and reference images. No new third-party reference-image comparison was performed here.
- Pages 15-20: generated media and motion are optional tools. This product needs legible real records and honest relationship diagrams; generated product screenshots and decorative 3D would weaken that evidence.
- Pages 21-25: perform a subtraction pass; remove duplicate labels, containers, and competing calls to action.
- Pages 26-32: use bounded review iterations and human final judgment. Do not keep iterating until an arbitrary self-score is reached.

Skill: `C:/Users/dell/.codex/skills/ui-ux-pro-max/SKILL.md`. Verified search matches: `data table responsive overflow` -> Table Handling and Overflow Hidden; `focus not obscured` -> WCAG 2.2 minimum/enhanced distinction. The broad inventory-hierarchy search returned generic headings/breadcrumbs, so it is not used as inventory-specific evidence. The design-system search returned Product Demo + Features / Minimalism & Swiss Style. Adopt restrained hierarchy and truthful demonstrations; reject its generic orange accent, external font download and template section sequence in favor of the established field-atlas palette and system fonts. Slint desktop and plain HTML/CSS website are the detected stacks; no React/Tailwind guidance was substituted for either.

Current-state evidence: `test-results/complete-20261030T124838Z/`, particularly `native-inventory.png`, `native-settings.png`, `native-connections.png`, `native-*-editor.png`, `homepage-{390,768,1440}.png`, `cookies-{390,768,1440}.png`, and `report.md`. Desktop captures use physical 1100x720, font step 4, dark theme; Linux captures include default light mode. These are existing captures from the immediately preceding test run. The full-page web capture did not explicitly scroll lazy images into view; an empty tour image in that capture is **not** a confirmed broken image. Website HTML still references v0.5.0 screenshots while the download version is v0.6.0; identify screenshots accurately and recapture the next verified candidate.

## Design brief: an atlas for the next person

Audience: IT operators and independent developers looking up infrastructure, preparing a move, or transferring responsibility. The first question is: what supports this business, where is it, how can I reach it, and who last confirmed it?

Keep the warm neutral canvas, forest-green brand, restrained separators and system typography. Use monospaced text only for addresses, ports and commands. Maintain one primary action per task region. Inputs use a quiet surface and persistent labels; primary buttons use a filled brand surface, secondary buttons a clear border, and low-frequency actions use a labeled menu. Navigation selection uses a slim accent and background rather than styling every tab as an action button. Avoid underlining buttons, repeated bordered cards and all-caps duplicate page labels.

Maintain existing saved text-size values (11/12/13/15/17 logical pixels) for compatibility in the first implementation. Do not silently remap users' settings. A future default-scale change needs a separately tested migration. The prototype defaults to a readable web demonstration size and is not a replacement Slint renderer. At every native step, derive control min-height from text metrics and padding; fixed height must not cut off labels. Row density cannot shrink below the selected font's comfortable minimum. Use measured text contrast >=4.5:1 for normal text and >=3:1 for meaningful controls; the prototype is not a contrast certification.

## Desktop improvement specification

### Navigation and service records

Use an adaptive vertical resource navigation with Settings at the bottom. Prefer a 168-200 logical-pixel rail at normal desktop widths; at constrained widths use a labeled resource selector rather than clipping labels or introducing unlabeled icons. Businesses, Servers, Sites, Services and Domains remain separate destinations; Connections is a view, not another asset type. Preserve per-view search/filter/column preferences. Show the current workspace once, with a small explicit Demo label.

The Services header contains the title/count, one New service action and an overflow destination for Export/import capabilities actually supported by that view. Search and filters form one compact second row. Avoid putting Fit columns or resource-specific import at global scope.

Service table defaults: service name + version, endpoint (protocol and port), host name + address, responsibility/review state. Freeze the header and identity column within the table's own scroll container. Keep column controls reachable. Fit columns allocates widths from available space and measured content; it must not promise that arbitrary custom columns can fit without scrolling. Offer a concise default set and optional columns. For long values use wrapping or an explicit detail affordance; a tooltip alone is insufficient. At narrow widths, keep identity/endpoint/host accessible and move optional details into the record panel. No invisible truncation of the only access action.

Single click selects and previews; explicit Edit opens the editor. Host details preserve the return position and include notes, owner, source and audit history. Show genuine stored event types/timestamps only; use 'No recorded history' instead of fabricating pre-migration events. Never expose or copy protected credentials automatically when opening details. Keep protocol-aware actions: HTTP(S) opens a browser, SSH copies a command or uses an explicitly configured client, database protocols offer address/parameter copy or a configured client. Do not route all ports into a browser. Preserve single-port service records and the server's pending default SSH service confirmation flow.

### Connections

Default to one selected business and its recorded path: Business --includes--> Domain --points to--> Server --hosts--> Service. Domain/subdomain hierarchy, CDN targets and dependencies are **different relationship types**, with text labels and line styles. This reading order does not replace the data model or imply all relationships are a strict tree. Shared hosts and multiple origins remain visible. Missing host information produces a visible unresolved segment; do not invent a server or silently skip it. A real domain-to-service relationship may remain as an additional typed edge.

Keep only business selection, resource search, Fit view and Add in the main toolbar. Put advanced relationship display options in a disclosure. Node selection opens contextual details; relationship selection exposes its meaning and edit action. Default to structural links; Dependencies is opt-in. Provide a synchronized text/list route and keyboard navigation. Fit view must not reduce text below the selected readable size: switch to a vertical path or focused neighborhood when four readable columns cannot fit. Real graph work must retain pan/zoom and CRUD; the small static-path prototype does not validate these capabilities.

Performance implementation: update the viewport transform during dragging, avoid DB writes/layout recomputation per pointer event, cache node bounds/edge routing, and cull offscreen drawing. Persist positions after gesture completion. Suggested budgets, to be agreed on hardware: p95 pointer-to-paint <=33ms at 100 visible nodes, no >100ms task during continuous pan; test 100/1000/10000 total resources with a bounded visible neighborhood. These are targets, not measured results.

### Appearance and forms

Keep Appearance globally reachable as a compact action. Use a local preview transaction: opening snapshots the saved settings; theme/text-size/density changes preview immediately; **Apply** persists all three; **Cancel/Escape** restores the snapshot. On persistence failure retain the draft and show an inline error with Retry. Dirty state and close behavior must be consistent in Settings and the quick panel. Offer five visibly different text-size ticks plus actual sample records. Do not ship an auto-save claim while the underlying behavior still requires Save settings.

Editors need an adaptive two-column form at wide sizes and one column at largest text/narrow widths, scrollable content, sticky action area with reserved scroll padding, visible labels and field-level errors. Host selection summarizes chosen hosts and opens an inline selectable list. Keep metadata, access credentials and relationships in separate labeled sections; disclose optional fields. Modal focus must stay inside, background navigation must be inert, and closing returns focus to the opener. Validate actual Slint accessibility behavior; web dialog behavior is not a substitute.

## Website improvement specification

Keep **IT in a glance.** Lead with one concrete outcome: 'Know what runs where. Keep the context for whoever comes next.' State local-first/manual maintenance near the demonstration. Use a full-width readable resource path followed by an accurately versioned real screenshot; on phones show one explained task at a time and an optional full-size screenshot. Never imply the design prototype is a shipped application screen.

Reduce the homepage to: promise and download -> one resource-chain example -> find/check/copy demonstration -> daily work and handover preparation -> privacy/platform limits -> FAQ and final download. Consolidate repeated coverage/workflow/scenario prose. Keep the five handover situations as short selectable examples or a dedicated page rather than five repeated marketing sections. Use real local-record capabilities; packaged handover exports and acceptance workflows remain planned.

Download cards must use the same button treatment for Windows, Linux, macOS Apple silicon and macOS Intel. Make architecture, format, version and checksum visible; maintain unsigned-package and platform credential limitations. Retain canonical itgla.com URLs. Move optional website Account away from the primary evaluation path without removing legal/privacy routes. Align shared footer copy with businesses, servers, services and domains rather than a server-only description. Version metadata, sitemap, llms.txt and screenshot provenance must be checked together against the release manifest.

Cookies/legal pages: one shared reading-width token (approximately 68-76ch for prose), predictable left alignment, consistent heading spacing, and a labeled locally scrolling table where needed. Do not force a comparison table into unreadably narrow cells to avoid its internal scroll. Prior evidence shows no page-level overflow; measure zoom and focus separately before claiming full remediation.

## Test findings integrated into delivery order

| Priority | Work | Completion evidence |
|---|---|---|
| P0 | Resolve or document applicability of RUSTSEC-2026-0414, pulldown-cmark 0.13.4; fixed >=0.13.5 | Locked build/tests and advisory gate; not a visual redesign and not fixed here. |
| P0 | Replace stale X11 coordinates/removed Servers Ports interaction | Current native semantic actions or robust geometry; server -> pending SSH -> confirmation -> service CRUD rerun. Existing failure is a harness defect, not confirmed product CRUD failure. |
| P0 | Navigation, largest text, visible essential table values, reachable editor actions | Native 1000x640/1100x720/1440x900, all five text sizes, both primary light/dark themes; no clipped navigation or inaccessible actions. |
| P1 | Service table and contextual detail panel | Find host/port/version, inspect history, copy protocol-appropriate value; restore view state. |
| P1 | Business-focused readable graph and equivalent list | Shared-host and missing-host fixtures; correct typed links; keyboard alternative; recorded frame timings. |
| P1 | Transactional appearance preview | Apply persistence, Cancel/Escape rollback, failed-save retry; actual Slint tests. |
| P1 | Admin totals reconciliation | Same filters/timezone/event definition; explicit Unknown bucket; fixture 87 = 42 + 18 + 27 Unknown and an assertion. Never patch live totals to match fabricated fixture values. |
| P1 | Current product evidence and shorter homepage | Versioned genuine captures, download manifest, sitemap/llms alignment, Chromium/Firefox/WebKit, 390/768/1440 widths and 200% zoom. |
| P2 | Warnings and full acceptance closure | Remove unused Slint components when safe; real installers, Mac hardware, DPI, screen reader, 60-minute soak and timed user tasks. |

Acceptance targets for a future five-person task study: at least 4/5 independently identify a service's host/protocol/port within 30 seconds, trace a business-to-service path within 45 seconds, distinguish preview from saved appearance, and choose the correct package without coaching. Record errors/time and compare to the current baseline; these targets have not been achieved by a study yet.

## Reviewable artifact and scope

Open `docs/design/ui-review-20261030/index.html` through a local server. It contains an interactive service-table concept, a readable business path with details, transactional appearance preview, and a shorter website composition. All records and history are explicitly fictional. Search, view switching, record inspection and appearance controls operate within the prototype; it does not implement database editing, native graph drag performance, secure storage or an installer.

This iteration adds design documentation/prototype only. It does not change Rust/Slint production behavior, dependencies, analytics production data or published website files. The prior full test report remains blocked. No commit, push or deployment is included.

## Prototype verification

Chromium Playwright checks passed for service search, empty results, SSH-specific copy labeling, immediate theme preview, Escape rollback, Apply/reload persistence of theme/text size/spacing, the four-level resource path, dependency display, equivalent text path and modal keyboard focus containment. An initial focus-wrap failure was corrected with explicit dialog Tab cycling and the checks rerun successfully.

At widths 390, 768, 1100 and 1440, product and website views had no page-level horizontal overflow. The 390-wide service table intentionally retains 330 pixels of local horizontal scrolling; other tested service widths had none. Connections passed at 390 and 1100. Largest text with dark theme was checked at 1100x720. Screenshots under `output/playwright/ui-review-20261030/` were visually inspected for service tables, connections, appearance and website desktop/mobile layouts. These checks validate this small HTML prototype only; native Slint behavior, actual graph drag performance, assistive technology and user task success remain unverified.

## Native and website implementation preview ¡ª 2026-10-30

This follow-up implements the first visual changes in the real application and local website. It supersedes the prototype-only scope above for this iteration. No commit, push or deployment has been performed.

- Desktop: persistent left navigation with Settings last; clearer active state; readable service rows with wrapping names and smaller Edit actions; responsive graph toolbar; graph panning retained at large text sizes rather than shrinking node text.
- Appearance: right-side modal with immediate preview, Apply, Cancel and Escape rollback. Background navigation is blocked while it is open. Failed saves retain the draft and expose the error.
- Website: shorter practical introduction, visible business/domain/server/service path, optional handover scenarios, clearer download actions and responsive typography. Two genuine captures from this build replace older images and are explicitly marked as unreleased local previews with fictional data.

Validation: cargo +1.92.0 test --locked --bin itgla: 94 passed. cargo +1.92.0 build --locked: passed. The three existing homepage viewport tests passed. Playwright CLI checked 390/768/1100/1440 widths for page overflow and scenario disclosure. English-only gate, 18-page footer consistency and git diff --check passed. Actual native captures were inspected at 1400x900 (light), 1100x720 (largest text/dark) and with Appearance open. Unused Slint component warnings remain.

Evidence: test-results/ui-implementation-20261030/ and output/playwright/ui-review-20261030/. Review binary: dist/ui-preview-20261030/ITGLA.exe; Open-Preview.ps1 launches a separate fictional demo workspace and records its hash/PID in launch.json. Local website: http://127.0.0.1:4198/.

Limits: this is a Windows development build, not an installer or published release. The broader acceptance blockers documented above, real assistive technology checks, all DPI/font combinations, graph frame-time measurements and user task studies are not closed by these checks. Service notes/status can still truncate in narrow views; opening the record exposes the complete details. Large graph layouts intentionally need internal panning.

## Follow-up: footer, discrete text size and aligned table grid

The shared website footer now uses a light sage background with dark readable links and brand. Appearance retains only former sizes 2/3/4, presented as 1/2/3; pointer input snaps to label centers and supports arrows/Home/End. Legacy extreme preferences clamp to the closest remaining size. Server/site grids now use visible theme-aware cell dividers and explicit zero-padding layouts, including the scrolling row containers, to align data with pinned headers.

Validation: 94 Rust tests passed, including pointer/keyboard size selection and six column position/width comparisons. Latest Windows debug build passed. Playwright verified the actual footer background with cache disabled; shared footer consistency passed across 18 pages and English-only UI checks passed. Native screenshots were inspected in light at 1400x900 and dark/largest supported size at 1100x720. Review binary is dist/ui-preview-20261030-r2/ITGLA.exe with a separate fictional demo workspace. This is a local preview only.


## Follow-up: clearer sections, compact hosting groups and optimized preview

Website sections alternate white and pale green reading surfaces. Removed ornamental disclosure/path dividers and the dark download band. Footer uses the header canvas (#f1f3ed); brand and tagline share a wide row and navigation has its own row. Fixed the literal question-mark separators in the resource path. Shared CSS cache version is 20261030-clarity-r4.

Graph resources now read inventory once, index records/host ranks and cache sort keys. Hosting groups reserve rows per server; services appear beside their primary visible host. Row pitch reduced from 112 to 96 logical units, with inter-row routing kept outside 66-unit nodes. Multiple hosts remain represented as saved links. A regression covers multi-service groups; existing distinct-port/no-node-intersection and business filtering tests pass. Server/site editor controls retain a 40-unit minimum height.

Validation: 95 Rust tests passed; cargo fmt and clippy -D warnings passed (existing unused Slint component build warnings remain). Four homepage browser tests passed. Playwright checked 390/768/1100/1440 overflow/disclosure, adjacent section backgrounds, and header/footer color equality with browser cache enabled. English UI and shared footer checks passed across 18 pages. Native light graph and dark largest-size screenshots inspected.

An optimized Windows review build retains debug assertions solely to enable isolated audit/demo launch fixtures: dist/ui-preview-20261030-r4/ITGLA.exe, with Open-Preview.ps1 and launch.json. This is a local test build, not a published installer. Release compilation was rerun after the editor-height correction.

Performance probe: same 16-resource fictional demo, 1400x950 physical window, software renderer, 160 synchronous wheel messages, optimized r4 versus unoptimized r2. Optimized P95 dispatch 4.03 ms / CPU 828.1 ms / RSS 35.5 MB; baseline 59.16 ms / CPU 7531.2 ms / RSS 49.2 MB. These single runs occurred during background compilation, changed both build profile and layout, and measure message round trips rather than FPS or frame latency. They do not establish large-inventory performance or a precise speedup. Evidence scripts/captures: test-results/ui-implementation-20261030/; raw probe JSON copied there.

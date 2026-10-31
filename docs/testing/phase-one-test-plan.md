# ITGLA complete local test and acceptance plan

Prepared 2026-10-30 for the phase-one responsibility inventory, schema 21. This is a test specification, not a statement that the product has passed acceptance or is released.

## Objective and boundary

An operator must be able to determine what a business uses, where a service runs, who maintains it, where its documentation came from and whether that information needs another check. Editing, importing, hiding or upgrading must not lose the workspace or leave an obsolete check presented as current.

The scope includes businesses, servers, services, domains, responsibility context, linked hosts, pending SSH, existing connection graphs, search, tags, CSV, SQLite recovery, appearance, native controls, installer compatibility and related website regressions. Formal recipient acceptance, offline handover packages, cloud collaboration and new asset categories remain future work; they must not be advertised as implemented.

The executable catalog is [phase-one-cases.json](phase-one-cases.json). Each case contains a stable ID, priority, steps, expected outcome and an explicit automation boundary. `AUTO+MANUAL` means partial automated evidence plus required acceptance, not full automation. Uncovered field combinations and real hardware checks remain visible.

## Entry conditions and fixtures

1. Record source revision, uncommitted changes, binary/package SHA256, schema, OS, renderer, physical window size and display scale. A hash of an existing binary does not establish that it was built from current source.
2. Use a unique disposable data directory, a new OS test profile or a VM snapshot. Never point tests at an operator's real database. Retain originals and a pre-test backup before any fixture upgrade/restore/delete.
3. Use fictional Commerce / Origin / API, example.com documentation and TEST-100 sources, Alex/Casey contacts, 192.0.2.x addresses and made-up credentials. Do not connect to servers or services to verify inventory; these tests exercise the product's local records and controls.
4. Prepare fresh, populated, hidden, checked, due, empty-contact, long-content and multi-host fixtures. For date-boundary acceptance use disposable fixture timestamps rather than changing the OS clock.
5. Obtain a genuine schema-20 fixture made by the matching older binary. Resetting only `user_version` on a schema-21 file is useful replay coverage but is not proof of genuine old-version compatibility.
6. Rust 1.92.0, Python 3.10+, Node and Git are needed for local automation. Full checks need Clippy; release preparation also needs cargo-deny, cargo-llvm-cov and llvm-tools-preview. Installation is separate; the runner never silently installs tools. Website automation needs existing npm dependencies, Playwright Chromium and a working `python3` command.

## Environment matrix

| Surface | Required coverage | Evidence |
|---|---|---|
| Windows | Current supported Windows x64; new install and upgrade; software renderer | Installer/binary hash, OS version, logs and reviewed screenshots |
| Desktop layout | 1100x720 and 1440x900 physical pixels; widths around the actual editor breakpoint | All five font stops, light/dark, long text, empty/populated/error/open-modal states |
| Native display | Real 125%, 150%, 200% scale and mixed-DPI displays | Scale, physical dimensions, pointer/focus behavior |
| Linux | Supported DEB distro and AppImage environment on local authorized build hosts | Distribution, architecture, desktop/session, dependencies and package hashes |
| macOS | Intel and Apple Silicon on matching hardware | Both DMGs, launch/upgrade, signing/OS prompt records |
| Website | Current Chromium minimum; 390x844, 768x1024, 1440x900 | Playwright logs plus reviewed layout; Safari/Firefox checks before claiming support |
| Assistive technology | Narrator or NVDA on Windows; VoiceOver on macOS | Manual names/states, keyboard focus and modal behavior |

Font/theme layout is a separate dimension from real display scaling. Headless Slint assertions, a resized screenshot and a live OS DPI test prove different things.

## Execution commands

Run from the repository root in PowerShell or a shell with Python:

```text
python scripts/run-test-plan.py --profile quick
python scripts/run-test-plan.py --profile full
python scripts/run-test-plan.py --profile release --list
python scripts/run-test-plan.py --profile full --website
python scripts/test_test_plan_runner.py
python scripts/start-acceptance.py --binary dist/phase-one-preview/ITGLA.exe --launch
```

| Profile | Required automated checks | Intended use |
|---|---|---|
| quick | Runner self-tests, format, English text, static UI, whitespace, focused asset-context tests | Feedback after context/UI changes |
| full | Quick static checks, complete Rust suite instead of focused subset, all-target/all-feature Clippy | Local regression candidate |
| release | Full plus dependency advisories, coverage >=60% repository baseline, locked optimized build | Automated release preparation only |
| `--website` | Add existing npm language/footer/Playwright suite | Related website changes; optional flag does not claim native UI coverage |

Use `--toolchain` only when deliberately testing another installed toolchain. `--list` prints exact commands and performs no execution or file writes. No profile builds installers, triggers GitHub jobs, pushes code or publishes a site. Windows and Linux packaging remains on local hosts / 192.168.1.7; only the two DMG builds use the designated GitHub workflow when separately authorized.

`start-acceptance.py` is separate from the automated profiles: it records an exact binary hash in a new acceptance directory and sets a new `ITGLA_DATA_DIR`. Without `--launch` it only prepares that directory; with `--launch` it opens the candidate and records its PID. It does not populate data automatically, terminate other previews or infer a passed test from process launch. Use Open sample data in that isolated application for fictional demo records.

Every actual invocation creates `test-results/plan-<UTC>-<unique-id>/` containing:

- `summary.json`: commands, exit status, timings, environment, source revision and dirty-file list, case-catalog hash, existing optimized binary hashes.
- `report.md` and per-gate `.log` files: inspect the first failed diagnostic, not just the summary.
- `acceptance.csv`: every catalog case starts at `NOT_RUN`; automated command success never pre-fills acceptance.
- `workspace/` and `temp/`: isolated application directory and subprocess scratch files retained as evidence. The runner does no recursive cleanup. Delete only the explicitly reviewed run directory when evidence is no longer needed.

Each subprocess has a finite timeout. A timed-out/interrupted process tree belonging to the runner is terminated; an independently running preview is left alone. Logs stream to disk. Exit 0 means all selected commands passed; 1 means failure, timeout or interruption; 2 means a blocked/missing prerequisite without another failure. A tool discovered but reporting a missing subcommand/toolchain is a failure, not a successful skip. Release readiness is always `NOT_EVALUATED` because manual and package gates require separate evidence.

The runner isolates `ITGLA_DATA_DIR`, temporary paths and inherited native audit hooks. Cargo still uses its usual shared compilation cache; avoid concurrent builds to prevent cache locking. It does not strip normal toolchain/proxy environment configuration. Fixtures and retained screenshots must contain no real secrets.

### Website execution qualification

The existing website config uses localhost ports 4183 and 4174 and can reuse already-running servers. Before a website gate, verify both ports serve the candidate source and local synthetic fixture; otherwise stop the unrelated servers or use a separate checkout/environment. Do not treat green tests against an old preview as candidate verification. The wrapper discovers npm beside the Node executable to avoid Windows shell interpolation; installations with a different npm layout need that prerequisite fixed before this gate can pass. Website screenshots/traces remain in the suite's usual `website/test-results` location; archive that directory with the run report before another website run overwrites it.

## Test sequence and traceability

| Order | Case family | Risk and oracle |
|---|---|---|
| 1 | DATA-01..06 | Schema/data identity, migration snapshots, atomic failures and recovery |
| 2 | CTX-01..07, CHK-01..06 | Optional states, independent drafts, timestamp/reviewer semantics and invalidation |
| 3 | FIND-01..02, CSV-01..03 | Exact filter membership, literal spreadsheet output and partial-format limits |
| 4 | REG-01..05 | Pending SSH, independent service ports, business graph host retention, details/import regression |
| 5 | UI-01..06 | Font/theme matrix, editor reachability, table preferences, focus, assistive technology and DPI |
| 6 | PERF-01..02 | Timed native search/drag, dataset size and session behavior |
| 7 | PKG-01..03 | Actual installer lifecycle on each platform/architecture |
| 8 | WEB-01..05 | Candidate copy/demo, downloads/checksums, canonical route agreement and synthetic analytics |
| 9 | UAT-01..02 | Correct lookup and onboarding tasks with representative users |

Run P0 first within each family. Use repository tests for deterministic repository/UI-model rules, headless Slint for the covered component paths, native inspection for actual paint/focus/OS integration, and real users for terminology and time-to-value. Playwright covers the website, not the Slint desktop application. Existing Bash audit scripts remain useful Linux supplements (`run-full-audit.sh`, `capture-desktop-audit.sh`, `audit-desktop-interactions.sh`, `audit-packages.sh`); review their report for skipped gates rather than interpreting exit success alone as complete coverage.

For each manual case record status (`PASS`, `FAIL`, `BLOCKED`, `NOT_RUN`, or justified `NOT_APPLICABLE`), tester, environment, exact reproduction and expected/actual result, evidence path and issue reference. An automated case also requires its mapped test to appear in the Cargo log; zero matched tests is not evidence. A partial mapping never waives unexecuted steps.

## Exit criteria and issue handling

- P0: data loss, invalid restore/migration, incorrect check status, unusable primary action, wrong host relationship or broken advertised download blocks a release candidate. Every applicable P0 must pass on the required platform.
- P1: important search, readability, accessibility, installation/usability or consistency issues need a fix and rerun; any proposed noncritical deferral must name an owner, reason and expiry. Do not silently waive an inaccessible supported layout.
- P2: refinements and longer observations can be scheduled with explicit tracking and a documented acceptance decision.
- All selected automated commands must pass; a blocked, skipped or not-run required check means incomplete verification. Review dependency advisory evidence and apply the project's remediation policy. Do not lower the coverage baseline merely to make a run pass; the runner uses the repository's existing 60% threshold, while the Rust skill recommends 80%, so record and resolve that policy difference before asserting skill-wide release compliance.
- A full source build is not an installer test. Native platform, accessibility, migration/recovery and artifact-checksum evidence must accompany any release recommendation. Preserve current data before rollback; schema-21 edits cannot be assumed to exist in the schema-20 backup.
- Proposed product targets need measured evidence: at least 4/5 users correctly look up required context within 30 seconds and finish the fixture setup within ten minutes. Proposed native performance budgets in PERF-01 require an agreed baseline machine; neither target is a current measured claim.
- A failure report includes case ID, candidate revision/binary hash, environment, fictional fixture, steps, actual/expected behavior, severity and artifacts. After a fix rerun the failed case and affected shared-component/data tests, then the full candidate regression once.

## Starting evidence and remaining gaps

The preceding implementation recorded 93 passing Rust tests, format/Clippy/English/static UI success and an optimized Windows build. It inspected selected native forms at 1100x720 physical pixels with maximum font size. Those logs are historical evidence under `test-results/asset-context`; rerun automation for the current candidate rather than carrying forward a pass.

The genuine old-binary migration pair, 89/90-day boundaries, all-four-field invalidation combinations, full native appearance/DPI matrix, actual installer upgrades, screen readers, both Mac architectures, real-user timing and painted-frame performance remain acceptance work unless this run supplies evidence. The existing migration replay test is not a substitute for a genuine schema-20 snapshot. cargo-deny and coverage tools were previously absent; do not describe dependency/coverage checks as passed without new logs. Existing skill governance limitations are recorded in [phase-one-local-preview.md](../phase-one-local-preview.md); this test runner does not override those policies or grant release authority.

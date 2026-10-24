# ITGLA homepage visual direction - local draft

Date: 2026-10-24
Initial scope: homepage hero only. Subsequent iterations below expand the local implementation to the public website. No deployment.

## Experience and direction

Audience: IT operators and independent developers evaluating a private desktop inventory. The first decision is whether ITGLA helps them find services, access details, domains and hosting servers, followed by choosing a platform download.

Selected direction: **Infrastructure field atlas**. Paper-colored introduction, precise typography, a dark horizontal relationship illustration and restrained colors distinguishing domain, application and database. Preserve the existing green identity and system fonts. Use native HTML text rather than a raster image for names, ports and relationship labels. No new dependencies, animation, external fonts or tracking.

Considered alternatives: a generic split-screen desktop mockup repeats the old hero and obscures the expanded inventory; a cinematic server-room image adds atmosphere but little evidence of the product's job. The field atlas explains the inventory model directly.

The source PDF's workflow informs exploration, a concrete design brief, screenshot review, one targeted correction and subtraction. This draft does not claim an independent design score or a user-tested conversion improvement.

## Decisions and truthfulness

- Keep ITGLA as the H1 and "Know what runs where" as the supporting promise. Explain services, domains and hosting servers in the first paragraph.
- Keep the Windows installer as the dominant action; explicitly label the adjacent all-platform and checksum destination.
- Use fictional addresses and label the diagram as an illustration, not an app screenshot. It is a conceptual illustration of recorded inventory relationships, not a claim of a shipped topology UI or live discovery.
- Remove the fake desktop window, cosmetic coordinate labels, background grid and decorative numeric proof strip.
- Preserve remaining sections for this bounded draft. They need a later content alignment pass if this direction is accepted.

## Local review and limitations

Browser: Playwright Chromium. Homepage checks pass at 390x844, 768x1024 and 1440x900. Initial screenshots exposed overly narrow mobile columns; the correction uses vertically ordered objects with larger text and downward relationship arrows. Recheck includes 320px width, keyboard focus and visible action targets. Final screenshots and local gate evidence are stored under `.planning/homepage-direction/`.

The language and shared-footer checks pass. The full suite passes 14/15 tests; the existing release artifact check fails because v0.4.0 packages are absent from the local static directory. This draft preserves official download paths and does not provide locally mirrored installers. Native screen-reader usability and real-user comprehension are not established by these automated checks.

## Second iteration: the rest of the homepage

The accepted field-atlas direction now extends through the homepage. Audience and download destination remain the same. The evaluation journey is: understand the promise, inspect a concrete record, understand services/domains/relationships, see routine tasks, resolve privacy concerns, choose a download. This is a local implementation, not a production release.

The previous lower sections told a server-table-only story and used a large decorative diagram. Replaced them with an explicitly illustrative service record, an editorial list of supported inventory types, and a three-step server-table workflow. Import, search and cell-copy claims remain scoped to server records rather than implying that every inventory view has identical behavior. Credential copy identifies Windows-only protection and metadata-only support on Linux/macOS.

The existing raster workspace asset depicts an early Chinese interface and is unsuitable as proof of the current English release. It is not used in the page body; removed the homepage social-image references to it as well. A current screenshot and a corresponding social preview remain to be produced from a verified release. Fictional records are labeled; no customer endorsements or performance claims were invented.

Implementation uses a homepage-only stylesheet, existing palette and system fonts. Other pages and the shared footer markup remain unchanged. Reduced competing containers, made both download destinations buttons in the final decision section, and placed the optional feedback form behind a keyboard-operable native disclosure. Fixed the skip-link destination to accept focus. A tablet screenshot exposed cramped diagram columns; narrowed connectors and reduced node padding, then captured all widths again.

Verification: language and shared-footer gates pass; 14 existing browser cases passed with the known local artifact case excluded. After the feedback test and final content changes, all 5 focused homepage/content cases pass, including keyboard disclosure, simulated submission failure, preserved input and successful retry. These feedback checks intercept requests and do not send real messages. At 320, 390, 768 and 1440 CSS pixels: zero horizontal overflow, one H1 and visible main actions at least 44px tall. A computed-style contrast check covered 133 rendered text elements with the feedback form expanded and found no failures. It is a scoped solid-background check, not a complete accessibility certification. Screenshots and check outputs are in `.planning/homepage-continuation/`.

## Remaining work, in priority order

1. **Real product evidence:** capture a verified current English release with fictional data; show finding a service, inspecting its host and copying a field. Prepare a truthful social preview. The older asset must not stand in for the current product.
2. **Complete the evaluation journey:** extend the accepted visual system to downloads and inventory/import guides, including platform choice, architecture, unsigned-package expectations and checksums. Validate real release artifacts in the deployment environment; installers are absent from this local static preview.
3. **Validate with intended users:** ask IT operators and independent developers to explain the product after viewing the page, find a relevant capability and select the appropriate download without coaching. Record misunderstandings before changing copy. No user study has been performed here.
4. **Cross-browser and accessibility review:** verify Firefox, Safari, native screen-reader navigation, zoom and real-device touch behavior. Chromium checks and contrast calculation are evidence for this iteration only.
5. **Performance and release gate:** measure loading on a constrained network once final product media exists; perform final link/artifact and privacy checks before production delivery. No new analytics were introduced.

## Third iteration: prioritized website implementation

The previous remaining-work list is historical. The October 24 continuation implements the product evidence, positioning, downloads, shared public-page styling and account clarity items. The current primary job is still to understand the desktop app and choose a suitable package; no account backend or desktop source was changed in this continuation.

### Product evidence and hierarchy

The hero now leads with **Know what runs where**, explicitly identifies IT operators and independent developers, and explains manual maintenance. The ITGLA brand remains in the header. Replaced the conceptual hero diagram and lower fictional record card with real Windows 0.4.0 client-area captures and a concrete workflow. The screenshots use an isolated workspace, reserved example addresses, a fictional `.example` hostname and no credentials. They were converted to WebP with no interface retouching. The two images total 98,428 bytes. Full-size links preserve readability on small screens, and HTML instructions carry the essential information independently of the screenshots.

Screenshot provenance: the unmodified executable came from `https://itgla.com/downloads/v0.4.0/itgla-v0.4.0-windows-x86_64.zip`. Its SHA-256 matched the published manifest: `25acf757c9af8c5e585d08c8f65428663edc55c0bf2ae7182448968e5b6266fb`. The recorded actions were searching Checkout, opening Edit, inspecting app-01 and invoking Copy ports; the app displayed Copied. Captures are `website/assets/itgla-v0.4.0-service.webp` and `website/assets/itgla-v0.4.0-servers.webp`. A new social image uses the real capture; obsolete Chinese screenshots are no longer referenced by the changed homepage/download metadata.

### Download and reading experience

- One public marketing stylesheet shares the existing palette, type hierarchy, spacing, focus styles and editorial sections across 15 public pages. Authentication application functionality and shared footer contents are preserved.
- Download buttons remain uniformly styled. OS jump links, package descriptions, both Mac architectures, unsigned-package expectations and platform-specific checksum commands appear beside the relevant decisions.
- Download links point directly to canonical `https://itgla.com/downloads/v0.4.0/` files, so the local preview does not depend on locally mirrored installers. The seven packages and checksum manifest returned successful HEAD responses with nonzero content length. Only the Windows ZIP was fully downloaded and hash-verified; HEAD responses are not integrity certification for the other packages.
- Windows PowerShell uses `Get-FileHash`; Linux uses `sha256sum --ignore-missing -c sha256sums`; macOS uses `shasum -a 256`. The page explains comparing the same filename and that a checksum does not replace code signing.
- Server inventory and import guides now connect to the current product workflow. The import guide includes a downloadable fictional CSV and distinguishes server imports from service/domain entry.
- Marketing navigation exposes one optional website account link rather than competing login/signup actions. The sign-in and download pages explain that website accounts do not enable desktop synchronization.
- Homepage FAQ structured data is generated from the same visible answers for this update. LLM discovery describes manual infrastructure inventory; the existing sitemap URL set is unchanged and significant page dates are updated.

### Validation and corrections

Commands: from `website`, run `npm test` with `ITGLA_RELEASE_ORIGIN=https://itgla.com` to verify local UI plus official release HEAD responses. Without that variable, artifact checks target the local server and require the real packages to be present. No fake packages or successful download fixtures were introduced.

The complete suite passed **18/18**. The final targeted keyboard-table check also passed. Chromium and Playwright WebKit rendered the homepage, downloads, inventory/import guides and sign-in at 320, 768 and 1440px without horizontal overflow; automated WCAG A/AA checks found no violations on those wide layouts. An additional axe-core 4.10.3 check covered all 15 public pages at 320px and the expanded feedback form. It initially found an unfocusable horizontally scrollable example table. Added a named, keyboard-focusable table region, tested ArrowRight scrolling and repeated the audit: zero reported violations. Skip-to-main focus works. A narrow header wrapping issue was also corrected and rechecked.

Cold Chromium mobile lab measurement: 390x844, 1.6 Mbps download, 150ms latency, 4x CPU slowdown. Homepage FCP/LCP 1.508s, transferred 163,466 bytes; downloads FCP/LCP 0.980s, transferred 49,102 bytes. Both measured CLS 0. These are individual local lab runs, not field percentiles or a conversion claim. Screenshots include fully loaded deferred images; no animations or external font dependency were added.

Working evidence lives in `.planning/site-polish/`: capture provenance, browser/axe reports, performance JSON and reviewed screenshots. Screenshots were inspected for hierarchy, narrow wrapping, download buttons and command readability; no independent reviewer score is claimed.

### Open verification and delivery boundary

Firefox was downloaded, but Windows cannot activate its `mozglue` assembly and reports a side-by-side configuration error. It is **not** counted as a passed browser. Playwright WebKit on Windows is engine coverage, not actual Safari on Apple hardware. Real-device touch, native screen-reader testing and actual participant validation remain open. The task/recording template is ready in `docs/website-usability-test.md`; no people were contacted and no sessions were fabricated. No production deployment, new analytics, commit or push was performed.

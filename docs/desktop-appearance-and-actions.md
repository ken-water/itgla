# Desktop actions and appearance

The desktop's white action buttons were too similar to its input fields. Actions now share `ui/controls.slint`: a muted green fill and visible border for secondary actions, a dark green fill for the main add/save action, and explicit hover, pressed, focus, selected and disabled states. White native inputs retain their existing editing behavior. Reading surfaces use a slightly softened white. Inventory action groups have gaps between neighboring buttons.

`ui/appearance.slint` owns the shared appearance choices and live sample. The always-visible **Appearance** toolbar button opens them over the current workspace. Theme swatches, a five-position text-size slider and row-spacing buttons replace hidden dropdown choices. Both the sample and workspace update immediately; only **Save settings** persists them. Closing the popup keeps the current preview, and Escape dismisses it. Failed saves retain the preview and display the error before the generic unsaved hint.

The palette now covers canvas, panels, tables, selection, labels, actions, dialogs and status text. Field Atlas is green, Slate is cool blue, Sand is warm beige, Contrast emphasizes monochrome surfaces, and Dark uses dark slate surfaces with light text and mint primary actions. Native inputs, dropdowns and checkboxes explicitly follow the selected light/dark mode rather than the operating-system preference. Theme names retain their existing stored indices; Dark adds index 4. Contrast is a visual option, not an accessibility certification.

The slider snaps to Small, Medium, Large, Extra large and Largest. Body roles use 11, 12, 13, 15 and 17 logical pixels respectively; existing saved indices 0–2 keep their previous sizes and Large remains the default. Keyboard arrows move one step; the current name and “n of 5” remain visible. Tables, toolbar actions and service/domain details use the shared type roles. Appearance labels, actions, hints and sample headers now follow the shared text scale. Scrollable appearance content keeps the save action reachable. The five slider ticks use their actual target sizes (11, 12, 13, 15, 17 logical pixels). Larger headings have additional separation and Services and Domains have separate direct navigation tabs. Dense tables may still require horizontal scrolling at larger text sizes.

The shared action component preserves the properties used by its consumers (`text`, `enabled`, `primary`, `checked`, `clicked`) and exposes named button semantics. Appearance choices additionally expose checkable state. Disabled actions reject both pointer and accessibility activation. Space/Enter activate focused actions, and focus has a separate outer ring.

Verification uses Rust 1.92.0, the existing locked dependencies and Windows software rendering. All 76 tests pass; Slint tests cover navigation, endpoint CRUD and migration, selection exclusivity, preview without persistence, save parameters, disabled actions, modal Guide blocking and visible save errors. Windows UI Automation checks keyboard activation, immediate preview, explicit persistence and Escape. A real SQLite writer lock also verifies visible save failure, retained selection and successful retry after unlocking. All native checks use isolated fictional databases.

Playwright in Chromium and WebKit reviews the actual Windows captures through a local evidence viewer. It samples the rendered primary/secondary/input colors, compares before/after header pixels, and checks capture presentation at three viewer sizes. This is not a browser implementation or a Playwright E2E test of the native executable. Native captures include wide and narrow windows at Windows 125% scaling. Evidence lives in `.planning/desktop-actions/`.

Changed action text contrast is at least 7.4:1 across normal/hover/pressed states; action borders are 3.12:1 against their fill. These measurements and the quieter visual hierarchy support readability, but do not establish reduced eye strain in a user study. Native screen-reader sessions, other operating systems and production packaging were not performed in this local UI iteration.


## Independent server ports and simpler navigation

Servers now opens a dedicated **Ports (count)** manager instead of treating every port as one copy/edit cell. Each row stores its own port number, transport (Unspecified/TCP/UDP), protocol/application and purpose. Each address can be copied independently, including correctly bracketed IPv6 addresses. This iteration prepares protocol-specific actions; it does not initiate remote login or network connections.

Existing servers open Ports from the editor too. New servers can still start with numeric ports, including imported CSV/JSON data. Those entries begin with unspecified transport and protocol: port numbers alone do not establish the application. The manager saves immediately, rejects duplicate port/transport pairs and invalid ports, retains at least one endpoint and limits each server to 32 entries. The legacy numeric field remains a distinct-port projection for existing search, sorting and portable imports/exports; per-port metadata is retained by whole-workspace SQLite backups. CSV/JSON aggregate export cannot carry endpoint descriptions. A later aggregate import preserves metadata for retained numeric ports and removes entries for ports it removes.

Schema 18 automatically backs up schema 17 before migration and backfills one entry per numeric port. Changes and projection updates occur in one transaction. Before downgrading, make a whole-workspace backup, then restore `backups/pre-schema-18.db` with the old binary; endpoint changes after that snapshot are absent from the old backup.

Services records running software, versions, access details and hosting servers; Domains records hostnames, registration/CDN and origin servers; Sites remains the list of URLs. Settings stays last. Guide uses a modal popup surface, blocks background pointer and keyboard navigation, supports Escape and returns focus to Guide when closed. Its body scrolls at larger text sizes. Other existing rectangle dialogs also consume background pointer clicks; a full keyboard-modal conversion of those dialogs is outside this change.
# Service access points and server forms

- Servers shows host information. Port configuration lives in Services, linked through
  hosting servers. Each new server creates a pending SSH proposal (port 22, note `ssh`);
  this is a suggested configuration, not a network scan or a verified running service.
- Review a pending record, edit its port/protocol/account as needed, and explicitly
  choose **Confirm service**. Add separate service records for additional ports.
- Protocol, transport, version, notes, protected credentials and host associations are
  editable in Services. HTTP/HTTPS opens a browser; SSH copies a command; databases
  and other protocols copy the connection address. Pending services cannot connect.
- Schema 19 backs up the workspace before converting legacy endpoints into services.
  Existing matching services and credentials are retained; conversion and new-server
  service creation are transactional. Legacy server port values remain for old exports.
- The server editor uses stacked fields with font-dependent heights and scrolling.
  Save/confirm actions remain fixed at the bottom of their panel at every text size.
- Shared buttons use one border and fill with hover/focus feedback; the extra bottom
  edge decoration was removed for all variants.
# Contextual, compact actions

- Global header: Guide and Appearance only. Servers and Sites own their import
  and column-fit actions; unsupported pages do not show disabled table actions.
- Each table has an independent Auto-fit on open preference, saved immediately
  without saving an unrelated appearance preview. Legacy fit settings initialize
  both preferences until the user chooses them independently.
- Services/Domains use Active records / Hidden records. Hidden records are kept
  and can be restored; they are not deleted.
- New and Export share the inventory heading. A More actions menu appears only
  for the selected record; blank tags and notes do not reserve lines.
- Quiet secondary actions preserve hover, keyboard focus and accessible labels.
  Form save actions stay visible, align right, and use a compact primary button.
- Settings appearance controls use a readable maximum width. Custom-field and
  appearance-popup save buttons no longer span their containers.

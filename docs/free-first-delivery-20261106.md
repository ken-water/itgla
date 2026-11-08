# Free-first delivery

Date: 2026-11-06. Authority: sequential local implementation requested by the product owner. No production deployment, new tracking, customer outreach, payment enablement or release is included.

## Outcome and sequence

Help an independent developer or small IT operator understand a real environment without registration. Preserve the existing local workspace implementation and data model.

1. Reliability: reuse the existing regression suite; fix observed defects, test backup recovery and input preservation. Native frame timing, real DPI, screen-reader and platform checks require actual environments and remain explicit acceptance gates.
2. First use: offer isolated sample data, adding a server and importing a table directly in the guide. Show a short, actionable overview path without a mandatory wizard.
3. Low-cost entry: retain scoped templates, mapping, duplicate policy and validation. Route onboarding through the same validated commands; do not duplicate importer state.
4. Useful context: make server/service/domain/business relationships and global search accessible from the overview. Do not count automatically proposed SSH records as confirmed user assets.
5. Return value: show recorded domain dates and manual Work. State that reminders are recalculated while the application is open; they are not a closed-application notification service.
6. Ownership and handover: expose portable full backup and confirmed restore. Export a readable metadata-only handover summary, excluding credential fields, custom columns and free-form notes. It is not a complete database backup or formal acceptance record.
7. Website and help: explain the free local workflow and distinguish the downloadable release from the local candidate. Keep current download version unchanged. Add a first-use guide, plan boundaries and matching sitemap/llms links.
8. Learning: prepare task sheets and an anonymous local observation template. Do not add desktop telemetry or invent user sessions.

## Design decisions

Use existing Slint components and semantic theme tokens. Keep actions compact. The overview first-use area presents three understandable choices rather than another navigation system. Advanced integrations do not compete with basic local work. A restore confirmation explains replacement, the pre-restore recovery point and application closure; the next launch loads all restored models afresh. Cancelling never writes. Demo mode cannot restore a real database.

Full SQLite backups include sensitive information and are not encrypted containers. Windows-protected credentials remain tied to the original Windows profile. Metadata exports omit dedicated secret fields but names, addresses and contacts can still be sensitive; users must review files before sharing.

## Commercial boundary

The owner has chosen three tiers: Free, Pro and Team. Free includes local inventory, basic relationships/search/manual reminders, import/export and backup/recovery. Do not introduce a small asset-count paywall. Pro is intended for API and automation, cross-device continuity and optional encrypted cloud backup. Team is intended for shared workspaces, permissions and audit. Paid capabilities are proposals until implemented and validated. Existing internal website/admin endpoints are not the paid product API. No prices or launch dates are approved.

This decision supersedes the old roadmap's proposals to charge for basic local reminders/history/recovery. The earlier $5/month and $49/year examples are unvalidated hypotheses, not published commercial commitments.

## Verification and remaining gates

Use isolated in-memory or unique temporary workspaces only. Run fmt, relevant Rust tests, Clippy, UI/language/footer checks, website browser checks and a Windows candidate build. Record exact results in the delivery evidence; never call compilation an installer or cross-platform acceptance result.

Observe five target users doing: add/import real records, confirm one proposed service, link a domain to its real host, search and follow context, record a reminder, export and recover a test copy. Proposed activation: at least three real confirmed assets, one meaningful relationship and one search. Initial goals are four of five completing a useful query within ten minutes, twenty activated users and ten returning within fourteen days. These are learning targets, not measured results.

## Delivery trace

- Reliability: restore path-alias protection and recovery-point tests in storage.rs; full UI/storage regression suite retained.
- First use and context: ui/app.slint guide and ui/workspace.slint overview; workspace.rs routes to validated import/editor actions. Existing workspaces see a compact Getting started toggle rather than a mandatory wizard.
- Entry: the overview importer now invokes the shared import-reset command, clearing stale ready/error/site-import state. Existing templates, preview and duplicate policies remain authoritative.
- Reminders: the existing recorded-date and Work workflow remains; the page now explicitly states its open-application limitation.
- Ownership: src/free_workspace.rs supplies full backup selection, confirmed restore with process closure and allowlisted plain-text handover. Restore rejects an active-database alias and a pre-restore recovery-point alias. Handover uses a new file only and excludes hidden records, credential fields, custom fields, documentation links and notes. Server purpose is included; service/domain/business notes are deliberately omitted. Review metadata before sharing.
- Website: getting-started.html, plans.html, free entry links, sitemap and llms. Existing downloadable release claims remain separate from the local candidate. A screenshot review caught and corrected the plan CTA's low-contrast inherited text color.
- Learning: docs/testing/free-first-pilot-20261106.md and the blank local CSV; recruitment, sessions, return observations and release acceptance remain unrun.

For measured results and remaining gates, see [implementation evidence](testing/free-first-evidence-20261106.md).

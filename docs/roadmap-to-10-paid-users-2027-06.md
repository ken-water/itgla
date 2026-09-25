# ITGLA Roadmap To 10 Paid Users

## Target

By 2027-06-30, ITGLA should have 10 paying customers who actively maintain at least one real project. The target is deliberately small: prove that a solo founder with one or a few servers will pay for reduced operational risk before building a broad ITSM product.

The initial commercial offer is **Founder Pro: $5/month or $49/year**. The first ten customers may receive a founding price of **$39/year**, in exchange for a short feedback session and permission to use anonymized product feedback. No lifetime plan is offered until hosting and support costs are understood.

## Product Boundary

The free local product remains useful without an account:

- server records, tags, IP addresses, ports, custom columns;
- CSV/Excel import, local SQLite storage, export, and backup;
- English desktop application and verifiable Windows/Linux packages.

The paid value must be protection and continuity, not the number of servers:

- encrypted cloud backup and restore;
- optional sync across two devices;
- change history and recovery points;
- expiry reminders for certificates/domains;
- priority support and a guided data-review session.

Do not implement team permissions, monitoring, automatic discovery, or a general hosted API before ten customers have paid and used the first paid workflow.

## Funnel And Success Metrics

The June target is 10 paid users, not 10 downloads. Use this minimum funnel as the operating model:

| Milestone by 2027-06-30 | Target | Definition |
| --- | ---: | --- |
| Qualified conversations | 40 | Founder/developer with a real project and infrastructure to maintain |
| Activated users | 30 | Records one real server and returns at least once within 14 days |
| Paid trials or founding-plan invitations | 15 | Explicitly asks for backup, sync, reminders, or support |
| Paying users | 10 | Successful payment and an active project in the product |
| 30-day retained paid users | 8 | Uses or updates the inventory in the following 30 days |

Weekly scorecard: qualified conversations, activated users, returning users, paid invitations, payments, retained paid users, top requested workflow, and support hours per user. Do not optimize page views or downloads in isolation.

## Month-By-Month Plan

### 2026-09-25 to 2026-10-31: Positioning And Interviews

- Keep v0.2.0 stable; fix only release-blocking defects.
- Recruit 10-15 solo founders, indie hackers, and small SaaS operators who manage at least one server.
- Run 8-10 interviews using their real infrastructure notes. Ask what they fear losing, forgetting, or exposing; do not lead with a pricing question.
- Add a lightweight feedback link and a privacy-safe interview note template. Do not add tracking to the desktop app.
- Success gate: at least 5 people independently describe backup, cross-device access, expiry reminders, or handoff as a recurring problem.

### 2026-11: Activation And Retention Baseline

- Release the smallest onboarding path: create first server, import a table, export a backup, and reopen the data.
- Publish a one-page “start with one server” guide and a short video or GIF using synthetic data.
- Add an in-app, local-only “backup reminder” and a visible export/recovery path before any cloud work.
- Personally observe 5 users completing first value in under 10 minutes.
- Success gate: 15 activated users, 8 returning within 14 days, and 3 users asking for a paid continuity feature.

### 2026-12: Paid Problem Validation

- Offer a manual Founder Pro pilot to the 5-8 strongest users. The first version may use encrypted backups delivered through a controlled, documented process; never handle plaintext server credentials.
- Test two messages: “recover your infrastructure notes” and “keep projects available on every device.”
- Charge only after the user confirms the paid workflow and refund terms. Use a hosted payment provider; do not store card data.
- Success gate: 3 paid users or 5 written commitments to pay. If neither happens, revise the paid problem before building sync.

### 2027-01: Minimum Paid Product Design

- Freeze the paid contract: encrypted backup, restore, device limit, retention, support response, and deletion behavior.
- Write the data-flow and threat model for local encryption, key recovery, account/session handling, and backup deletion.
- Design the smallest cloud service boundary: account, workspace, encrypted backup blobs, metadata, audit events, and health checks.
- Update Privacy, Terms, Refund, and Cookie policies before accepting recurring payments.
- Success gate: 5 total paying users or 5 signed pilot commitments, plus a reviewed recovery and deletion procedure.

### 2027-02: Private Paid Beta

- Implement a narrow beta for up to 10 invited users: account sign-in, encrypted backup upload/download, two-device restore, and support contact.
- Keep the desktop local-first: cloud use must be explicit, cancellable, and exportable.
- Add billing state, trial expiry, cancellation, refund handling, rate limits, audit logs, and backup restore tests.
- Do not expose a public API or team sharing yet.
- Success gate: 5 active paid users, 90% successful restore attempts, no critical privacy/security defect, and support workload under 2 hours per week.

### 2027-03: Public Founder Offer

- Open the Founder Pro offer to the waitlist with a clear $5/month or $49/year price and no hidden server-count limit during the pilot.
- Publish a comparison page that states local free features, paid continuity features, retention, cancellation, and refund rules.
- Ask every new paid user to complete one backup and one restore during onboarding.
- Success gate: 7 paying users and at least 5 users completing a restore successfully.

### 2027-04: Reliability And Conversion

- Improve onboarding from observed failures, not broad feature requests.
- Add certificate/domain expiry records only if at least 3 paying users request them; otherwise keep the roadmap focused on backup and continuity.
- Add weekly recovery verification, backup age, storage usage, and clear failure messages.
- Run a pricing check with annual vs monthly plans; do not change price more than once during the month.
- Success gate: 8 paying users, 70% monthly active paid users, and zero unresolved data-loss incidents.

### 2027-05: Retention And Referral

- Ask each paid user for one referral to a founder with a real infrastructure problem.
- Add a simple referral credit only if it does not complicate billing or create abuse risk.
- Conduct 5 retention interviews: what would make them cancel, what they trust, and what they still keep elsewhere.
- Prepare June operating runbook: payment failure, restore failure, deletion request, refund, incident, and rollback.
- Success gate: 9 paying users and at least 7 retained for 30 days.

### 2027-06: Reach And Prove The Target

- Close the final qualified users through direct demos and referrals, not a broad discount campaign.
- Reach 10 paying users by June 30 and verify every account has a real project plus a successful backup/restore event.
- Publish a compact customer-informed roadmap for the next quarter.
- Decide whether to scale the paid beta, remain intentionally small, or stop cloud development and return to local-only improvements.
- Success gate: 10 paying users, 8 retained 30-day users, documented revenue, and support/recovery costs understood.

## Product Release Shape

- `v0.2.x`: stabilize the current dense local inventory and improve activation.
- `v0.3.0`: local onboarding, backup/recovery clarity, and user-observed usability fixes.
- `v0.4.0`: private encrypted backup beta for invited users only.
- `v0.5.0`: Founder Pro billing, restore reliability, policy updates, and operational runbooks.
- `v0.6.0`: post-target iteration based on retention evidence; do not promise team/monitoring scope in advance.

These version labels are planning markers, not a commitment to ship cloud features before the validation gates pass.

## Stop Conditions

Pause cloud development and reassess if any of these occur:

- fewer than 3 users report the same paid problem after 40 qualified conversations;
- users download the app but do not record a real server or return within 14 days;
- restore failures or privacy defects cannot be resolved without broad platform work;
- support and recovery operations exceed the revenue from the pilot;
- users consistently prefer a free export-only workflow.

The strategic fallback is still valuable: a polished local inventory can remain free while paid revenue comes later from support, private deployment, or a narrowly defined sync product.

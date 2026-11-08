# Free-first pilot and acceptance

Status: prepared; no real user observations or retention results recorded. Use the candidate binary hash and a separate workspace for every session. Never open a participant's only database with a test candidate.

## Native release checks before recruitment

- Exercise Windows at actual 100%, 150% and 200% DPI, all three text sizes, light/dark themes and a small viewport. Confirm that dialog actions remain reachable and table headers follow horizontal scrolling. Resizing alone is not a DPI test.
- Run the complete task path with keyboard only. Test the restore modal: background navigation is blocked, Escape/Cancel leave the database unchanged, confirmation is required, an invalid source shows a recoverable error, and a successful restore closes the process.
- Install/upgrade an isolated Windows profile and run Linux and both macOS builds. Verify package checksums, preservation of data, backup restore and rollback with the correct old binary/schema backup.
- Measure search input-to-visible-results and rendered map frame p95 at realistic sizes. Report hardware and build. Pure search timing and Windows message dispatch do not establish frame latency.

## Five observed sessions

Recruit independent developers, small-team IT operators or technical service providers who actually maintain an environment. Obtain consent for notes; recording is optional. Use synthetic names/addresses or a copy sanitized by the participant. Do not collect credentials, private keys, account numbers, real infrastructure files or personally identifying notes.

Give the participant goals, not button-by-button instructions:

1. Start without an account. Explore sample data and return to the real workspace. Explain whether their own data changed.
2. Add or import a server and a domain. Review the proposed SSH service. Confirm it only if accurate; correct its protocol/port if needed.
3. Link the domain to its real host and identify the server's service. Explain what an infrastructure change might affect, using recorded relationships only.
4. Search for a resource and find its host, purpose and responsible contact without assistance.
5. Record a known domain expiry or dated manual Work check. Explain what happens when ITGLA is closed. Review/postpone/reopen as appropriate.
6. Save a full backup, export a handover summary, change a disposable record, restore the test backup and reopen. Identify which file can recover the full workspace and which is intended for reading.

Record completion, elapsed time, facilitator intervention and exact friction. A task requiring help is not an independent success. Ask: What information would you still keep elsewhere? What would bring you back next week? Which missing automation or collaboration workflow would you pay for? A stated interest is not a purchase.

## Local observation format

Use `free-first-observations.csv`. Give each person a random participant code; do not record names or contact details here. Blank fields mean unknown, not zero. Store notes locally and delete when no longer needed. There is no desktop telemetry.

- `confirmed_assets`: active real server/service/domain records; exclude fictional demo, hidden records and pending auto-created services. Businesses provide context and do not inflate this count.
- `meaningful_relations`: reviewed real links, excluding links whose only significance is an unconfirmed auto-created service.
- `search_completed`: 1 only after a nonempty query leads to a useful resource result; otherwise 0 or blank if unobserved.
- `activated`: confirmed_assets >= 3 AND meaningful_relations >= 1 AND search_completed = 1. Compute from observations, never from downloads.
- `query_minutes` and `independent_query`: initial target is four of five independently reaching a useful answer within ten minutes.
- `returned_within_14_days`: record only after the observation window or a confirmed return. Do not equate a download or launch with useful return activity.

First cohort learning targets: twenty activated users and ten confirmed useful returns within fourteen days. These targets are hypotheses, not statistical proof or current results. If sessions reveal repeated blockers, fix those before increasing acquisition.

## Distribution and feedback

After release acceptance and authorization, use the getting-started page, short synthetic-data demo and existing website feedback form. Personally observed sessions and referrals should precede paid acquisition. Prepare outreach copy but do not send messages without the owner's authorization. Do not implement referral rewards, analytics tracking or a billing service in this phase.

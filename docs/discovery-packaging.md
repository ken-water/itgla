# Discovery: consent and plan packaging

Discovery requires two separate decisions:

1. **Target consent**: before any active check, the operator confirms that they
   are authorized to inspect the exact targets and ports. This applies to every
   plan and does not depend on whether a customer pays.
2. **Product entitlement**: the plan determines which discovery workflows are
   available. A paid entitlement never substitutes for target consent.

## Proposed product direction

| Capability | Free | Pro direction | Team direction |
| --- | --- | --- | --- |
| Manual inventory, review, search, map, local export and backup | Included | Free capabilities | Free capabilities |
| Manually initiated URL discovery and a useful small-scope active check | Proposed | Free capabilities | Free capabilities |
| Larger or recurring discovery, API and integrations | Not decided | Proposed automation | Pro capabilities |
| Shared review, roles and team audit | Not decided | Not decided | Proposed |

These discovery inclusions are product hypotheses, not a public offer or a
commitment that a paid plan is available. Pricing is intentionally unpublished. Paid packaging may be evaluated later after the free product proves useful.
Validate willingness to pay and ease of first use before launch. Technical
limits on an individual check protect the network and app; they are not
subscription quotas.

## Proposed asset capacity

The shared planning configuration is `config/plans.json`; it is not currently
consumed by the product or the website and does not enforce any limits.

The product owner selected the following inclusive managed-asset limits for
planning: Free 100, Pro 500, and Team 2,000. These apply to total confirmed
managed assets, regardless of whether they were entered manually or discovered;
they supersede the earlier proposal to leave local inventory unlimited. Pricing,
workspace scope and migration details still require decisions. This is not yet
implemented or an offer to purchase.

Count each independently managed business, server, service, domain or subdomain
as one asset. A database counts once when represented as an independent service
or database asset, not again for both labels. Ports, IP addresses, relationships,
tags, notes, contacts and reminders do not count separately. A certificate
counts only if represented as an independent managed asset. Hidden assets still
count; permanently deleted assets release capacity. A merged duplicate counts once.
These rules require product validation before enforcement, especially identity,
deletion and future asset types.

The current release has no asset-count limit. Any numeric capacity discussion is future research only.
Before introducing a Free limit, reconcile that promise, communicate any change
and determine a fair transition for existing users. Existing records must remain
readable, editable, exportable and recoverable even when their count exceeds a
new limit; never silently delete records or retroactively lock users out.

## Align discovery with inventory capacity

Each plan's discovery and managed-asset capacity must work together. A user
should be able to discover, review and manage assets up to that plan's available
inventory capacity. Discovery breadth and inventory limits must not be set
independently in a way that routinely leaves discovered assets unmanageable.

Discovery results are candidates until the user confirms them. Before import,
deduplicate against existing assets and within the candidate set; updates and
relationships between existing assets should not consume additional asset
slots. Show the current count, remaining capacity, eligible new assets and any
duplicates before confirmation. If candidates exceed available capacity, let
the user select which to add, retain the remaining results for later review, and
explain the available options without silently dropping or importing records.
Do not promise a paid upgrade until a purchasable plan exists.

When future limits are introduced, never delete existing assets on downgrade or
expiry. Keep existing records, relationships and exports accessible; explain
any restriction on adding new assets. Define and test how each asset type and
merged record counts before enforcing a commercial limit.

The current desktop app is free and works without an account. Pro and Team
cannot currently be purchased, and no desktop entitlement verification exists.
Do not add an invented plan selector, local boolean, or simulated paywall as a
security or billing boundary. Development previews must identify incomplete
features accurately. Existing local records, exports, backup and recovery must
remain accessible when a future subscription ends or a license is unavailable.

## Before shipping paid restrictions

- Define entitlement sources, offline validity and expiry behavior, and test
  expired, unavailable and downgraded states.
- Enforce privileges at the execution boundary as well as presenting them in
  UI; do not rely on disabled buttons or user-supplied tier names.
- Show the operator their target scope, selected checks and estimated work, and
  require a separate explicit target-consent action before starting a check.
- Clearly label which workflows are available now and which are proposed.
- Preserve access to existing inventory and exports after downgrade, offline
  periods or billing problems.
- Test plan and consent independently, including that paid access alone cannot
  initiate a check without target consent.

## Current implementation status

The Stage 3 TCP kernel is unfinished and has no UI, consent flow, persistence,
licensing or production entitlement enforcement. Its current technical limits
are implementation safeguards only. DNS resolution and cancellation behavior
must also be bounded and tested before the kernel is exposed to users.

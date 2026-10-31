# ITGLA product direction: IT in a glance

Date: 2026-10-29. Status: product direction and phased proposal, not a shipping-feature checklist.

## Product promise

**IT in a glance. Know what you have, how it connects, and what to pass on.**

Answer eight practical questions: what exists, which business it supports, where it runs, who is responsible, how to access and operate it, what needs attention, what a change affects, and what the next person still needs.

The website can express this direction through existing business/server/service/domain records and relationships. Packaged handover reports, formal acceptance, remote collaboration and transfer workflows are planned, not current capabilities. An understandable overview requires layers of information, not every field and graph node on one screen.

## Asset scope

Initial customer hypothesis: independent developers, small IT operators and technical service providers. Enterprise MSP procurement requirements are not automatically first-release scope.

| Category | Examples | Useful context | Sequence |
| --- | --- | --- | --- |
| Business and project | Customer systems, products, internal applications | Purpose, customer, environment, business owner, technical contact, importance | Existing business entity; extend its context first |
| Compute and hosting | Servers, VPS, VMs, hosted instances | Address, provider, account holder, location, OS, purpose, renewal, hosted services | Existing server records; add common context before deep cloud models |
| Services | APIs, web applications, databases, caches, queues, scheduled tasks | Version, endpoints, protocol, port, host, dependencies, deployment and recovery instructions | Existing services and relations; add operating context first |
| Domains and entry points | Domains, subdomains, DNS, CDN, proxy, load balancer, certificates | Registrar, manager, expiry, origin, resolution, certificate deployment | Existing domain fields first; dedicated entry/certificate models later |
| Data and recovery | Datasets, object stores, backup targets | Ownership, location, backup coverage, restore procedure, last tested restore, retention | Start with notes/links; structured objects later |
| Access and control | Cloud accounts, registrar accounts, Git organizations, service identities | Account holder, administrators, emergency access, vault reference, transfer and revocation state | Start with links; dedicated access register later |
| Code and delivery | Repositories, CI/CD, images, artifacts, scripts, runbooks | Location, ownership, license, build/deploy instructions, runtime version, external requirements | Business-level references first; structured objects later |
| Vendors and agreements | Hosting, SaaS, licenses, subscriptions, support | Contract holder, payer, cost/currency/period, renewal, contact, transfer restrictions | Later; a vendor is not a service running on a server |
| People and responsibilities | Business owner, operator, contractor, backup contact, recipient | Role, contact details, coverage, successor | Lightweight local contacts first; not user accounts or cloud collaboration |
| Endpoints and network equipment | Workstations, routers, switches, NAS | Location, identifiers, warranty, network/service impact | Custom fields initially; dedicated models only with demand |

People are responsibility records, not saleable assets. Secrets are controlled access material, not general notes. Documentation and checks supply operating context. A SaaS subscription should not require an invented server record.

## Minimum information model

Source inspection: `relationships.rs` already provides business/server/domain/service records and directed links. `operational.rs` provides verification dates and pending confirmation. `HistoryEvent` supplies an operation timeline. Businesses currently have name, tags and notes, without structured owners or handover acceptance. Existing history is not an immutable compliance audit.

First extend shared context where applicable: stable identifier, business links, environment, technical/backup contact, lifecycle, source, last verification time/person, notes and documentation links. Require only a name and essential linkage on creation; progressively collect the rest.

Separate business ownership, technical responsibility and account/legal ownership. One person may fill all three roles, but one ambiguous owner field must not silently stand for them. A contact does not imply a product login.

Typical relationship view:

```text
Business <-> domains / services / servers / future accounts, repositories, vendors
Domain -> subdomain -> optional CDN / proxy / LB -> server -> service -> data
Service -> dependency / external SaaS
Service -> repository / deployment instructions
Account -> controlled resources; contact -> assigned resources
Backup -> protected resource; instructions -> applicable resources
```

This is a common path, not a mandatory topology. A service can run on several servers; servers can be shared across businesses; managed services may have direct domain targets. Preserve true relation semantics. Show the server layer for conventional self-managed hosting instead of skipping it in business-filtered views.

Business association expresses operational scope, not legal ownership. Shared infrastructure must not be silently included in a company sale merely because a business uses it.

## Everyday work and five change scenarios

| Situation | Smallest useful workflow | Completion evidence |
| --- | --- | --- |
| Daily management | Review business -> resolve pending/stale/due items -> update -> record verification | Explicit verification date; missing information never appears healthy |
| Staff changes | Select responsibility scope -> assign successor -> share redacted context -> verify access -> transfer responsibility -> record revocation | Recipient and exceptions per item; documentation delivery and permission removal tracked separately |
| Server changes | Inspect dependencies -> record old/new location -> migration checklist -> verify access, data and DNS -> update links -> retain rollback context | Pre-change snapshot, verification results, open issues; no automatic migration claim |
| Service handover | Choose scope -> collect endpoints/hosts/dependencies/version/instructions -> recipient walkthrough -> capture gaps | Recipient completes agreed tasks without undocumented oral knowledge |
| Company sale | Establish asset/control/contract scope -> staged disclosure -> mark exclusions/shared resources -> record actual transfers -> confirm | Information delivery, actual control transfer and contractual procedures are separate |
| Technology transfer | Inventory code/artifacts/licenses/build environment/dependencies -> clean-environment build -> deployment/recovery walkthrough -> record restrictions | Reproducible agreed delivery; a screenshot of an existing running server is insufficient |

These are proposed workflows. Start with templates and manual confirmation, not a transaction-management or legal-approval platform.

## Phase 1: establish a usable business inventory

- Entry points: create a business, import an existing server table, or open sample data. Accept partial records.
- Keep the existing pending SSH service on server creation; default port 22 is a suggestion, never verified evidence.
- Add lightweight responsibility, environment and documentation references to existing entities.
- Distinguish blank, unknown, not applicable and pending verification. Retain verification dates.
- Prove three tasks: locate a service endpoint, identify businesses using a server, and find the responsible contact.

Proposed acceptance target: four of five target users create a real redacted example with one business, one server, two services and one domain within ten minutes; each lookup task takes under thirty seconds. These are test targets, not existing measurements.

## Phase 2: business-level understanding and handover preparation

- Business detail combines resources, contacts, entry points, relationships, verification and instructions. A readable list is the default.
- Keep table and graph selection aligned. Expand the relevant subgraph when investigating dependencies or impact.
- Add short deploy/restart/recovery templates and links to existing wikis instead of forcing document migration.
- A preparation view selects scope and highlights missing, shared and unverified items, with explicit applicability.
- Produce a redacted offline HTML/PDF information package and structured export with working relative links, timestamp, version and scope.
- Full recovery must preserve businesses and relations. Existing CSV or legacy project JSON is not automatically a complete recoverable workspace.

Acceptance: a recipient finds the entry point, host, contact, deployment and recovery instructions without asking the maintainer. A restored workspace preserves identifiers and links; an independent machine can open the offline package.

## Phase 3: close the handover loop

- A handover records scope, sender/recipient, due date, snapshot, gaps and acknowledgments.
- Track separately: information supplied, control transferred, recipient verification and old access revoked. No single action silently completes all four.
- Surface shared hosts, shared databases and non-transferable third-party accounts as exceptions.
- First implement local, manually recorded confirmations and exportable checklists. Authenticate remote parties only when remote collaboration becomes a validated requirement.
- Packages include a manifest, generation time, scope, exclusions and checksums. Unauthenticated local acknowledgments are not electronic signatures or compliance attestations.

Acceptance: run observed staff, server-migration and service-handover exercises. Detect omissions, retain previous context and exclude secret values by default.

## Phase 4: expand asset coverage with evidence

- Add account, vendor, repository/deployment, certificate and backup templates based on observed handover gaps. Promote templates to dedicated entities only when usage repeats.
- Prefer read-only import and difference previews. Preserve user-verified values and record provenance/time.
- Support company-sale and technology-transfer scope templates, staged disclosure and exclusions; do not claim automated registrar/account/contract transfers.
- Monetize validated automation, recovery and collaboration. Keep basic complete export available. Do not sell unimplemented cloud features as finished functionality.

## Interface principle

Use a business overview as the intended default entry while retaining direct global server/service/domain tables. Do not require users to rebuild their current inventory.

Show purpose and contacts first, then key entry points/resources, actionable gaps and freshness, recent changes, and handover readiness. Reveal complete fields/history on demand.

```text
Customer portal    Production    Maintainer: Alex    Backup contact: missing
app.example.com    2 servers     4 services          Verified: Oct 29
Needs review: 1 pending service / missing recovery instructions
Resources | Connections | Instructions | Changes | Handover preparation
```

Manual verification is not live monitoring. No recorded alert does not mean healthy. Documentation completeness is not handover completion. Counts include scope and freshness.

## Industry practices to borrow

Official pages retrieved on 2026-10-29; this is a focused model/workflow comparison, not a full competitive evaluation.

| Official source | Observed model/capability | Adaptation for ITGLA |
| --- | --- | --- |
| [NetBox Application Services](https://netboxlabs.com/docs/netbox/models/ipam/service/) | Device/VM-linked services with explicit protocol/port mappings | Separate service, deployment host and access endpoint. Do not undo the current one-port-per-row user workflow merely to copy another product |
| [NetBox Contacts](https://netboxlabs.com/docs/netbox/models/tenancy/contact/) | Individual/group contacts associated with objects and administrative context | Reusable contacts and responsibility scope; avoid implementing an enterprise identity system first |
| [Hudu](https://hudu.com/) | Structured assets, knowledge base, checklists, connected credentials/infrastructure | Operating instructions next to resources and reusable procedures; not an immediate RMM/password-vault/integration expansion |
| [IT Glue Runbooks](https://www.itglue.com/features/runbooks/) | PDF documentation packages for one, several or all clients and visibility into asset relationships | Business/customer-scoped readable delivery; distinguish PDF documentation from complete backup and ownership transfer |
| [Atlassian Assets object schemas](https://support.atlassian.com/assets/docs/what-are-object-schemas/) | Types, attributes and references organize assets | Defined types/relations and reusable fields; no complex schema design prerequisite for adding a server |

## Relation to earlier plans

`product-architecture.md` and `roadmap-to-10-paid-users-2027-06.md` remain background proposals. This document defines the brand, asset boundaries and handover sequence. Earlier version numbers are not a current implementation calendar.

Near-term sequence: verify existing capabilities -> lightweight responsibility/context -> business overview and offline information package -> manual handover checks -> evidence-led asset expansion. Public marketing must distinguish today's inventory capabilities from planned formal handover workflows.

# ITGLA discovery roadmap

## Goal

Turn a small amount of authorized input into a useful, reviewable map of business, domains, network edges, hosts, services, owners, and dependencies. Discovery produces evidence and proposals; it does not silently create or rewrite confirmed inventory.

Every observation must carry source, collection time, last-seen time, confidence, verification state, and evidence. Keep the distinction between an observed fact, an inferred relationship, and a user-confirmed record. Never infer credentials or claim that a CDN address is an origin server.

## Delivery status (2026-11-14)

| Stage | Local implementation | Next acceptance boundary |
|---|---|---|
| 1. Public URL | Implemented; validation results are recorded in `discovery-verification.md` | Try an owned public website and review its actual findings |
| 2. Review and enrich | Review inbox plus typed DNS/TTL, public/private suffix scope, TLS/SANs, HTTP/link evidence, bounded sitemap traversal and opt-in CT implemented locally | Live/native acceptance; field-level conflict resolution remains deferred |
| 3. Authorized checks | TCP kernel implemented; UI wiring and TLS/HTTP profiles pending | Explicit target scope, stop control and bounded TCP/TLS checks |
| 4. Internal collection | Planned; follows review improvements | One-shot Linux/Windows read-only collection and offline import |
| 5. Integrations | Planned | Read-only cloud/DNS imports with credential references and conflict review |
| 6. Ongoing value | Planned | Bounded refresh, change review, reminders and handover export |

The delivered slice does not claim a complete automatically generated architecture map. A public page can reveal only a subset of domains, and a DNS address alone cannot establish an origin server or a deployment relationship. Internal collection and provider data supply those missing facts in later stages.

## Stage 1 — Public URL, passive-first

- User supplies a full HTTP(S) URL.
- Fetch the page and common public files: `robots.txt`, sitemap, `llms.txt`, and `security.txt`.
- Extract scoped hostnames using an offline Public Suffix List, including private suffixes to keep shared-hosting tenants separate. Resolve bounded candidate names.
- Store the scan and observations as pending candidates with source and confidence.
- Reject domains that resolve to private, loopback, link-local, or reserved ranges. Pin HTTP requests to the validated address to reduce DNS-rebinding exposure. Do not follow redirects automatically.
- Label DNS addresses as candidates only; shared hosting, CDN, WAF, anycast, and load balancing make origin inference unreliable.

Current limits: this cut uses system DNS and the OS `curl` executable. Optional historical certificate-transparency lookup requires explicit consent. It does not perform a port scan, promise exhaustive subdomain enumeration, crawl authenticated pages, or identify origins behind a CDN. Missing optional files are expected and shown as warnings, not as proof that an asset is absent.

## Stage 2 — Review and promote

- Add a dedicated Discovery inbox with grouping by scan, evidence, confidence, and status.
- Allow confirm, ignore, mark stale, merge with an existing record, and retry.
- Promote a confirmed domain into Domains, a confirmed public URL into Sites, and create only evidence-supported edges.
- Normalize DNS records (A, AAAA, CNAME, MX, NS, TXT, CAA, SRV), TTL, resolver, and observation time.
- Add opt-in certificate transparency lookups, TLS certificate SANs, HTTP headers, redirects, canonical links, sitemap locations, and third-party references.
- Distinguish first-party subdomains, external vendors, and unrelated hosts.

### Current delivery boundary

The local implementation covers Stage 1, Stage 2 review and its evidence-enrichment increment:

- Overview has a collapsed **Discover from URL** entry. The URL and a reserved-width action share one row; guidance spans the section above them. Scanning disables URL editing and duplicate starts. Sample mode cannot scan or review real candidates.
- A stage-based progress bar reports DNS resolution, five public-page checks, candidate-host DNS resolution and saving. It measures completed work stages, not elapsed time or an ETA. Optional-file failures advance the run with warnings; 100% is set only after saving succeeds. The bar and current step are accessible to assistive technology.
- Public HTTP(S) URLs use default ports only. Credentials, query parameters, fragments, literal IPs, control characters, and oversized input are rejected. Do not paste secret-bearing URLs.
- Up to twelve page requests (five initial plus seven sitemap requests), sitemap depth two, 2 MiB per response, eight seconds per HTTP request, and 32 additional hostname DNS lookups. DNS waits at most three seconds per lookup. System DNS worker threads cannot be forcibly cancelled; no further request waits for a timed-out lookup.
- HTTP uses the OS `curl`, bypasses inherited proxy settings, ignores curl configuration, validates DNS addresses before connecting, pins the target IP, and accepts only successful HTTP responses. Redirects are reported for review rather than followed.
- The inbox supports 20-observation SQL pages across retained history, run selection, state/type filters and literal text search. The recent 100 runs are listed individually; older observations remain accessible through All runs. Warnings belong to the selected run; All runs explicitly labels warnings from the latest run. Source, collection time, confidence and recent review history remain visible.
- Before confirmation, identity matches show the existing asset's name, notes/purpose and hidden status. Confirm and link associates evidence with the selected matching asset without overwriting its fields. Domain names normalize case/trailing dots; web URL identity normalizes host/default ports while preserving path semantics. Retry URL starts a new reviewable run, preserving the previous evidence.
- Confirm, ignore, mark stale and reopen decisions are persisted with a review history. Confirming a domain creates a Domains record; confirming the input web page creates a Sites record. Existing records, including hidden records, are preserved and never overwritten. DNS addresses and optional public files can be confirmed as evidence but do not create servers or hosting edges.
- Confirmation and the review decision share a database transaction. Repeating confirmation or confirming the same asset in another run does not duplicate it. Marking evidence stale or ignored does not delete an asset already created from it.

The enrichment increment adds public/private suffix scope, typed DNS/TTL/resolver metadata, verified TLS leaf metadata/SANs, selected HTTP headers/redirect/canonical/link evidence, bounded recursive sitemap discovery and optional crt.sh lookup. Typed evidence stays readable in the review view. Still pending in Stage 2: field-level conflict resolution, resolver selection/failover and broader live interoperability acceptance. Evidence association deliberately does not overwrite user fields. No automatic origin, business ownership or deployment relationship is asserted from a webpage or DNS lookup. See discovery-stage-two.md for review and discovery-enrichment.md for collection bounds and dependencies.

### Migration and rollback

Schema 23 adds isolated discovery runs/observations. Schema 24 adds warning and review history tables so a locally created schema-23 database also upgrades safely. Neither migration rewrites existing asset tables. The repository takes its existing automatic pre-migration backups. Older clients reject newer database schemas; to roll back, close the application and restore the matching pre-schema backup using the existing recovery procedure. Preserve the newer database separately if discovery history is needed. Do not decrement `user_version` on a live database.

Schema 25 adds evidence-to-asset links and indexes for run paging/review history. It takes a pre-schema-25 backup first. Existing asset data is unchanged. Repeated migration execution is idempotent, including historical migration regression fixtures.

### Acceptance

1. Enter a valid public URL; observe loading and a final result or actionable warning.
2. Inspect a domain's evidence before confirming it. It appears in Domains once; no server or host relationship is inferred.
3. Confirm the input page. It appears in Sites once; optional files remain evidence only.
4. Ignore, mark stale, reopen, restart and review the same persisted candidate.
5. Confirm repeatedly and across scans; existing names, notes, hidden assets and host links are not overwritten.
6. Invalid promotion or an oversized report rolls back entirely. Opening schema 23 preserves candidates and adds schema-24 review support.
7. Check all supported font sizes in the collapsed entry, expanded inbox and evidence view. Confirm pointer resizing and horizontal scrolling still align table headers and rows.

Automated fixture tests cover parsing/scope bounds, reserved-network rejection, forced-pending writes, atomic rollback, idempotent promotion, decision persistence and schema-23 upgrade. Public network correctness and native visual appearance require separate live/manual acceptance; unit tests do not prove those.

## Stage 3 ? Authorized public checks

- The first local slice requires exact domains, IPs, or CIDR ranges, explicitly selected ports, and a TCP profile. See `discovery-stage-three.md`.
- Scope is bounded at 64 expanded targets, 16 ports, and 256 target-port pairs. CIDR expansion is explicit; URLs and credential-bearing inputs are rejected.
- Checks run sequentially with a 1?30 second timeout, visible progress, partial observations, and cancellation before each pair. A transport seam provides offline tests; `TcpTransport` performs the production TCP connect.
- Results remain observations. No server, service, domain, or relationship is created automatically.
- TLS certificate, selected HTTP, richer audit persistence, and UI wiring remain the next slice.
- Respect exclusions, per-host limits, timeouts, and maintenance windows. No password guessing, exploit checks, stealth, or arbitrary internet-wide scanning.
- Identify CDN/WAF/load balancer as a network edge, with evidence and uncertainty. Never attach its address as a server by default.

## Stage 4 — Internal hosts and authenticated collection

- Prefer a signed, one-shot, read-only collector that the owner runs inside the network. Also support explicit SSH/WinRM collection when the operator chooses it.
- Show exact commands, permissions, fields collected, destination, retention, and dry-run preview before execution.
- Discover OS/hostname, interfaces, routes, listening sockets, processes, packages, systemd/Windows services, Docker/Podman, Kubernetes context, and web-server virtual hosts.
- Map ports to processes and software with version and evidence. Keep bind address, protocol, and reachability distinct.
- Do not collect file contents, environment secrets, private keys, password hashes, or database rows. Redact credentials in config-derived evidence.
- Support disconnected/offline networks through encrypted, signed result bundles that can be imported for review.

## Stage 5 — Cloud and infrastructure integrations

- Read-only cloud APIs for AWS, Azure, and Google Cloud; then DNS/CDN providers and registrars.
- Virtualization and platform sources: VMware, Proxmox, Hyper-V, Docker, Kubernetes, and common backup/monitoring systems.
- Import Terraform state, Ansible inventory, and supported CSVs as source-tagged observations. Show stale syncs and conflicting values.
- OAuth or scoped API credentials remain in OS keychain or external secret manager. Store only a credential reference in ITGLA.

## Stage 6 — Continuous map and handover

- Scheduled, bounded rechecks with change history and drift review.
- Expiry, certificate, backup, orphaned DNS, unknown listener, and stale-owner reminders.
- Graph expands one hop at a time, supports type/environment/business filters, clustering, search, and impact paths.
- Generate handover packages with confirmed assets, relationships, owners, evidence dates, unresolved candidates, and explicit exclusions.
- Answer impact questions from confirmed graph edges, such as what depends on a host before retirement.

## Network and privacy coverage

| Environment | Preferred discovery | Boundary |
|---|---|---|
| Public web | URL, DNS, public files, TLS and optional certificate transparency | Passive by default; public IP checks only after confirmation |
| Home or office LAN | Local collector, explicitly selected subnet, SNMP where enabled | No subnet scan without user selecting the range |
| VPN or split DNS | Collector inside VPN, resolver-specific results | Record network context; do not merge identical private IPs across networks |
| Cloud VPC/VNet | Cloud read-only API and in-network collector | API scope and account visible; no inferred access |
| Air-gapped | Signed offline collector bundle | Manual transfer and review; no telemetry requirement |
| Managed third party | Provider API or user-supplied export | Mark owner and source; avoid probing without authorization |

## Product sequencing

1. Finish Stage 1 safely and make the findings reviewable.
2. Build Stage 2 before expanding sources. This prevents uncertain data from polluting the asset map.
3. Add the one-shot collector in Stage 4 for the user's central server-inventory need.
4. Add cloud APIs and periodic sync only after consent, credentials, and conflict handling are clear.
5. Use pricing boundaries around collection scale, integrations, scheduled sync, team review, and handover controls. Keep manual inventory, basic search, and a useful small-team map available in Free.

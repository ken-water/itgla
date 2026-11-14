# Stage 2 enrichment design

## Contract and acceptance

The URL entry collects evidence, never silently promotes assets. This increment adds offline public-suffix scope calculation (including private suffixes), typed DNS evidence from a system-configured resolver, certificate metadata/SANs from the verified HTTPS leaf certificate, selected HTTP headers and links, and bounded sitemap traversal. Optional certificate transparency is off by default; the UI names crt.sh and explains that the domain is sent to it.

Public addresses remain candidates, not verified origin hosts. Third-party references remain evidence, are never traversed, and cannot be promoted as first-party domains. Wildcard certificates are patterns rather than concrete discovered hosts. CT records are historical, not proof of a current deployment. No automatic asset ownership or relationships are inferred.

Discovery remains bounded: at most 12 public page requests, sitemap depth two, 32 candidate host lookups, eight root DNS record types (A/AAAA/CNAME/MX/NS/TXT/CAA/SRV), two-second DNS waits per query, eight-second curl limits, two MiB page bodies, bounded headers/metadata/certificates, 100 warnings, 1,000 observations and a three-minute overall deadline checked between operations. No automatic redirect or external-link fetch. Every new HTTP target is scope checked, resolved, checked against reserved IP ranges and pinned before connecting. Query/userinfo-bearing URLs are excluded from saved/fetched links. Proxy support is explicit and only used for the fixed third-party CT provider; target probing keeps direct pinned connections.

Evidence is serialized in the existing bounded observation evidence field, with version/type, owner, resolver, TTL, collection method and validation boundary where applicable. Observation time is already stored by the repository. No schema migration is required; schema-25 data remains compatible.

Acceptance: public/private suffix fixtures; URL traversal/secret rejection; sitemap cycles, namespaces, depth/count bounds; cross-domain redirects and canonical/vendor classification; DNS transaction/question/response validation and typed records; malformed/oversized PEM/DER; certificate wildcard and external SAN handling; CT default-off and scope filtering; failures never abort useful partial results; monotonic progress and deadlines; persistence/review retains new evidence types without creating servers.

## Dependency rationale

- url 2.5.8 (MIT/Apache-2.0), base64 0.23.1 (MIT/Apache-2.0), quick-xml 0.42.0 (MIT): already locked transitive dependencies, promoted to direct dependencies for URL joining, PEM decoding and bounded XML event parsing. Avoids ad-hoc URL/XML/entity parsing.
- psl 2.1.253 (MIT/Apache-2.0, generated Mozilla Public Suffix List data under MPL-2.0): offline public/private suffix rules. Avoids an incomplete handwritten suffix table or a runtime lookup disclosing domains. Refresh with app updates; no guarantee of day-zero suffix changes.
- x509-parser 0.18.1 (MIT/Apache-2.0): decode the verified leaf certificate, without its optional signature-verification feature. Curl/OS performs trust and hostname verification; parsing does not imply additional cryptographic verification. Avoid custom ASN.1 parsing or a runtime OpenSSL requirement.
- hickory-proto 0.26.3 (MIT/Apache-2.0), std only: DNS wire parsing/encoding, without async runtime, DNSSEC or TLS features. Avoid localized command-output parsing and hand-written compression-pointer handling. std UDP/TCP handles bounded queries to the system resolver. DNS answers are not DNSSEC-validated and are not proof of reachability.

All use Rust and require no new native library. Exact registry checksums and versions are retained in Cargo.lock. Optional cryptography features are disabled. Protocol parsers consume untrusted remote data under explicit size limits. Fresh advisory tools remain a separate gate; do not infer an audit pass from compilation.

## User flow

1. On Overview, open **Discover from URL** and enter an owned public URL.
2. Leave certificate transparency off for direct public-page/DNS/TLS evidence. Opt in explicitly to send the registrable domain to crt.sh. The optional proxy is limited to this provider; `http://127.0.0.1:5670` is supported.
3. Observe stage progress through public files, sitemaps, typed DNS, optional CT and candidate resolution. Saving must finish before the UI reports 100%.
4. Filter the inbox by DNS records, TLS certificates, HTTP/link evidence, external references, scope or certificate patterns. Open Review to read TTL/resolver or certificate issuer/validity in a human-readable layout.
5. Confirm a domain or the input site only after checking its identity and existing-asset preview. Other kinds can be confirmed as evidence without creating inventory, hosts or deployment links.

## Known limits and rollback

The first usable configured DNS resolver is selected. SRV is queried at the input hostname; no arbitrary service-label enumeration is claimed. DNSSEC is not independently validated. OS DNS workers remain bounded by the existing worker limit and caller timeouts. TCP DNS fallback uses per-operation timeouts. The three-minute deadline is checked between operations, so a started operation may finish after it.

The public suffix list is shipped offline and refreshed by app updates. Certificate metadata depends on the installed curl supporting certificate output; missing support produces a warning while other results remain usable. A certificate summary shows up to eight concrete SAN names to fit stored evidence limits; each parsed SAN (up to 64 entries) is classified separately. Wildcards are patterns. HTTP headers are self-reported and cannot establish origin or trusted software versions.

CT can be slow or unavailable. A failed opt-in query must retain other results and show a warning. CT is historical; a listed name is not proof of current reachability or ownership. No automated active scan, internal collector, authenticated crawl, cloud integration, automatic origin detection or field overwrite is included.

No new schema is introduced in this increment. Replacing this preview with the preceding schema-25 preview preserves existing inventory and review data. New evidence kinds remain generic observations. Do not roll back to a pre-schema-25 client without restoring its matching backup.

## Dependency maintenance review

The selected versions are registry releases with source/checksum entries pinned by Cargo.lock. Existing usage/maintenance indicators and current advisory results were checked through a supplemental OSV query over the entire lockfile; the 29 newly locked package versions had no returned OSV advisory at the time of the check. This is a point-in-time observation, not a permanent security guarantee. The hickory/x509/PSL implementations replace custom protocol, ASN.1 and suffix parsing; bounded network and input contracts remain mandatory regardless of library reputation. Production release still requires the standard cargo-audit/cargo-deny and license gates. See discovery-enrichment-verification.md for existing advisory findings and unverified boundaries.

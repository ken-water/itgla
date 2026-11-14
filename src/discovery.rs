//! Safe, passive-first discovery from a user-supplied web URL.
//!
//! Discovery observations are candidates. They are never promoted to assets
//! until the user confirms them. Network work is intentionally bounded and
//! uses curl so the desktop build does not need a TLS stack.

use std::{
    collections::BTreeSet,
    net::{IpAddr, ToSocketAddrs},
    sync::atomic::{AtomicUsize, Ordering},
};

use serde::{Deserialize, Serialize};

mod authorized;
mod enrichment;
pub use authorized::{
    AuthorizedCheckOptions, AuthorizedCheckReport, AuthorizedTarget, CheckProfile, CheckTransport,
    PortCheckObservation, PortCheckStatus, TcpTransport, run_authorized_checks,
};
pub use enrichment::DiscoveryOptions;

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_HOSTS: usize = 32;
static DNS_WORKERS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryObservation {
    pub kind: String,
    pub value: String,
    pub source: String,
    pub evidence: String,
    pub confidence: u8,
    pub verification_status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub input_url: String,
    pub host: String,
    pub observations: Vec<DiscoveryObservation>,
    pub warnings: Vec<String>,
}

/// Render typed evidence as readable facts while preserving stored evidence verbatim.
pub fn evidence_summary(observation: &DiscoveryObservation) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&observation.evidence) else {
        return observation.evidence.clone();
    };
    let field = |key: &str| value.get(key).and_then(|v| v.as_str()).unwrap_or("Unknown");
    match observation.kind.as_str() {
        "dns-record" => format!(
            "{} {}: {}\nTTL: {} seconds | Resolver: {} | Transport: {}\nSystem resolver answer; DNSSEC not independently validated. DNS is not proof of a deployment.",
            field("owner"),
            field("record_type"),
            field("value"),
            value
                .get("ttl_seconds")
                .and_then(|v| v.as_u64())
                .map(|v| v.to_string())
                .unwrap_or_else(|| "Unknown".into()),
            field("resolver"),
            field("transport")
        ),
        "tls-certificate" => format!(
            "Subject: {}\nIssuer: {}\nValid from: {} UTC\nExpires: {} UTC\nDNS names: {}\n{}",
            field("subject"),
            field("issuer"),
            field("not_before_utc"),
            field("not_after_utc"),
            value
                .get("dns_names")
                .and_then(|v| v.as_array())
                .map(|names| names
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", "))
                .unwrap_or_default(),
            field("validation")
        ),
        _ => observation.evidence.clone(),
    }
}

pub(crate) fn host_from_url(input: &str) -> Option<String> {
    let value = input.trim();
    if value.len() > 2048 || value.chars().any(char::is_control) || value.contains(['?', '#']) {
        return None;
    }
    let without_scheme = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))?;
    let authority = without_scheme.split(['/', '?', '#']).next()?.trim();
    if authority.contains('@') {
        return None;
    }
    let host = if let Some((host, port)) = authority.rsplit_once(':') {
        if port.parse::<u16>().ok()?
            != if value.starts_with("http://") {
                80
            } else {
                443
            }
        {
            return None;
        }
        host
    } else {
        authority
    };
    let host = host
        .split(':')
        .next()?
        .trim()
        .trim_end_matches('.')
        .to_lowercase();
    let labels = host.split('.').collect::<Vec<_>>();
    if host.len() > 253
        || host.parse::<IpAddr>().is_ok()
        || labels.len() < 2
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        })
    {
        None
    } else {
        Some(host)
    }
}

fn origin(input: &str, host: &str) -> String {
    let scheme = if input.trim().starts_with("http://") {
        "http"
    } else {
        "https"
    };
    format!("{scheme}://{host}")
}

fn registrable_candidate(host: &str) -> String {
    // The private section prevents crossing unrelated tenants (e.g. github.io).
    psl::domain_str(host).unwrap_or(host).to_owned()
}

/// URL identity for review matching. Host/default-port differences are normalized;
/// path case and non-root trailing slashes remain significant.
pub(crate) fn page_identity(input: &str) -> Option<String> {
    let input = input.trim();
    let host = host_from_url(input)?;
    let (scheme, rest) = input.split_once("://")?;
    let path = rest.find('/').map(|index| &rest[index..]).unwrap_or("/");
    Some(format!("{scheme}://{host}{path}"))
}

fn host_like(value: &str, root: &str) -> bool {
    let value = value
        .trim()
        .trim_matches(|c: char| c == '.' || c == '/' || c == '\\');
    value == root || value.ends_with(&format!(".{root}"))
}

fn extract_hosts(text: &str, root: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut token = String::new();
    // DNS names are case-insensitive, including names embedded in HTML and XML.
    for ch in text.to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            token.push(ch);
        } else {
            if token.contains('.') && host_like(&token, root) {
                let host = token.trim_matches('_').to_lowercase();
                if host_from_url(&format!("https://{host}")).is_some() && found.len() < MAX_HOSTS {
                    found.insert(host);
                }
            }
            token.clear();
        }
    }
    if token.contains('.')
        && host_like(&token, root)
        && found.len() < MAX_HOSTS
        && host_from_url(&format!("https://{token}")).is_some()
    {
        found.insert(token.to_lowercase());
    }
    found
}

fn resolve(host: &str) -> Vec<IpAddr> {
    if DNS_WORKERS
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
            (count < 4).then_some(count + 1)
        })
        .is_err()
    {
        return Vec::new();
    }
    let host = host.to_owned();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let addresses = (host.as_str(), 443)
            .to_socket_addrs()
            .map(|items| {
                items
                    .map(|item| item.ip())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default();
        let _ = send.send(addresses);
        DNS_WORKERS.fetch_sub(1, Ordering::SeqCst);
    });
    receive
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap_or_default()
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, d] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && b == 168)
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 0 && c == 2)
                || (a == 192 && b == 88 && c == 99)
                || (a == 198 && (b == 18 || b == 19))
                || (a == 198 && b == 51 && c == 100)
                || (a == 203 && b == 0 && c == 113)
                || a >= 224
                || (a == 255 && b == 255 && c == 255 && d == 255))
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(mapped));
            }
            let segments = ip.segments();
            (segments[0] & 0xe000) == 0x2000
                && !(ip.is_loopback()
                    || ip.is_unspecified()
                    || (segments[0] & 0xfe00) == 0xfc00
                    || (segments[0] & 0xffc0) == 0xfe80
                    || segments[0] == 0x2001 && segments[1] == 0x0db8
                    || segments[0] == 0x2001 && segments[1] < 0x0200
                    || segments[0] == 0x2002
                    || segments[0] == 0x3fff && segments[1] < 0x1000
                    || segments[0] & 0xff00 == 0xff00)
        }
    }
}

/// Discover public evidence, reporting completed stages (0..=95), not elapsed time.
/// The caller owns the final persistence stage and only marks 100 after saving.
pub fn discover(
    input_url: &str,
    options: DiscoveryOptions,
    mut progress: impl FnMut(u8, &str),
) -> DiscoveryReport {
    enrichment::discover(input_url, options, &mut progress)
}

fn discover_using(
    input_url: &str,
    progress: &mut impl FnMut(u8, &str),
    resolve: impl Fn(&str) -> Vec<IpAddr>,
    fetch: impl Fn(&str, &str, &[IpAddr]) -> Result<String, String>,
) -> DiscoveryReport {
    progress(0, "Validating URL...");
    let Some(host) = host_from_url(input_url) else {
        return DiscoveryReport {
            input_url: input_url.trim().to_owned(),
            host: String::new(),
            observations: Vec::new(),
            warnings: vec!["Enter a full http:// or https:// URL with a domain name".into()],
        };
    };
    progress(5, &format!("Resolving {host}..."));
    let addresses = resolve(&host);
    if host == "localhost" || addresses.is_empty() || addresses.iter().any(|ip| !is_public_ip(*ip))
    {
        return DiscoveryReport { input_url: input_url.trim().into(), host, observations: Vec::new(), warnings: vec!["Public discovery only accepts domains that resolve exclusively to public IP addresses. Private and loopback networks will be supported by the authorized on-premise discovery phase.".into()] };
    }
    let mut observations = Vec::new();
    let mut warnings = Vec::new();
    let discovery_root = registrable_candidate(&host);
    let root = origin(input_url, &host);
    observations.push(DiscoveryObservation {
        kind: "domain".into(),
        value: host.clone(),
        source: "input".into(),
        evidence: input_url.trim().into(),
        confidence: 100,
        verification_status: "needs-review".into(),
    });
    for ip in &addresses {
        observations.push(DiscoveryObservation {
            kind: "ip-candidate".into(),
            value: ip.to_string(),
            source: "dns".into(),
            evidence: format!("A/AAAA lookup for {host}"),
            confidence: 80,
            verification_status: "needs-review".into(),
        });
    }
    let mut pages = vec![(input_url.trim().to_owned(), "page")];
    for path in [
        "/robots.txt",
        "/sitemap.xml",
        "/llms.txt",
        "/.well-known/security.txt",
    ] {
        pages.push((format!("{root}{path}"), path.trim_start_matches('/')));
    }
    for (index, (url, source)) in pages.into_iter().enumerate() {
        progress(
            10 + index as u8 * 12,
            &format!("Checking {source} ({} of 5)...", index + 1),
        );
        match fetch(&url, &host, &addresses) {
            Ok(body) => {
                observations.push(DiscoveryObservation {
                    kind: "public-endpoint".into(),
                    value: url.clone(),
                    source: source.into(),
                    evidence: "HTTP endpoint responded".into(),
                    confidence: 80,
                    verification_status: "needs-review".into(),
                });
                for subdomain in extract_hosts(&body, &discovery_root) {
                    if subdomain != host {
                        observations.push(DiscoveryObservation {
                            kind: "subdomain-candidate".into(),
                            value: subdomain,
                            source: source.into(),
                            evidence: format!("Found in {url}"),
                            confidence: 60,
                            verification_status: "needs-review".into(),
                        });
                    }
                }
            }
            Err(error) => warnings.push(format!("{url}: {error}")),
        }
        progress(
            10 + (index as u8 + 1) * 12,
            &format!("Checked {} of 5 public pages", index + 1),
        );
    }
    observations
        .sort_by(|a, b| (&a.kind, &a.value, &a.source).cmp(&(&b.kind, &b.value, &b.source)));
    observations.dedup_by(|a, b| a.kind == b.kind && a.value == b.value && a.source == b.source);
    let domains = observations
        .iter()
        .filter(|item| item.kind == "subdomain-candidate")
        .map(|item| item.value.clone())
        .collect::<BTreeSet<_>>();
    if domains.len() > MAX_HOSTS {
        warnings.push(format!(
            "DNS lookups limited to {MAX_HOSTS} names. Narrow the scope for remaining names."
        ));
    }
    let total = domains.len().min(MAX_HOSTS);
    for (index, domain) in domains.into_iter().take(MAX_HOSTS).enumerate() {
        progress(
            70 + (index * 25 / total) as u8,
            &format!("Resolving {domain} ({} of {total})...", index + 1),
        );
        for ip in resolve(&domain) {
            observations.push(DiscoveryObservation {
                kind: "ip-candidate".into(),
                value: ip.to_string(),
                source: "dns".into(),
                evidence: format!(
                    "A/AAAA lookup for {domain}; may be CDN or shared hosting, not confirmed origin"
                ),
                confidence: 70,
                verification_status: "needs-review".into(),
            });
        }
        progress(
            70 + ((index + 1) * 25 / total) as u8,
            &format!("Resolved {} of {total} candidate domains", index + 1),
        );
    }
    progress(95, "Preparing evidence for review...");
    observations
        .sort_by(|a, b| (&a.kind, &a.value, &a.source).cmp(&(&b.kind, &b.value, &b.source)));
    observations.dedup_by(|a, b| a.kind == b.kind && a.value == b.value && a.source == b.source);
    DiscoveryReport {
        input_url: input_url.trim().into(),
        host,
        observations,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_identity_normalizes_authority_but_preserves_path_semantics() {
        assert_eq!(
            page_identity("https://EXAMPLE.COM:443"),
            page_identity("https://example.com/")
        );
        assert_ne!(
            page_identity("https://example.com/Path"),
            page_identity("https://example.com/path")
        );
        assert_ne!(
            page_identity("https://example.com/path"),
            page_identity("https://example.com/path/")
        );
        assert_ne!(
            page_identity("http://example.com"),
            page_identity("https://example.com")
        );
        assert_eq!(page_identity("https://example.com/?secret=x"), None);
    }

    #[test]
    fn progress_tracks_pages_and_bounded_dns_even_when_optional_files_fail() {
        use std::cell::RefCell;
        let events = RefCell::new(Vec::new());
        let lookups = RefCell::new(Vec::new());
        let pages = RefCell::new(Vec::new());
        let report = discover_using(
            "https://example.com",
            &mut |percent, message| events.borrow_mut().push((percent, message.to_owned())),
            |host| {
                lookups.borrow_mut().push(host.to_owned());
                vec!["8.8.8.8".parse().expect("public address")]
            },
            |url, _, _| {
                pages.borrow_mut().push(url.to_owned());
                if url.ends_with(".txt") || url.ends_with(".xml") {
                    Err("Optional endpoint unavailable".into())
                } else {
                    Ok("https://api.example.com https://db.example.com".into())
                }
            },
        );
        let events = events.borrow();
        assert_eq!(pages.borrow().len(), 5);
        assert_eq!(
            &*lookups.borrow(),
            &["example.com", "api.example.com", "db.example.com"]
        );
        assert_eq!(report.warnings.len(), 4);
        assert_eq!(events.first().expect("start").0, 0);
        assert_eq!(events.last().expect("ready to save").0, 95);
        assert!(events.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        assert!(
            events
                .iter()
                .any(|(_, message)| message.contains("api.example.com (1 of 2)"))
        );
        assert!(
            events
                .iter()
                .any(|(_, message)| message.contains("security.txt (5 of 5)"))
        );
    }

    #[test]
    fn rejected_discovery_never_fetches_or_reports_completed_collection() {
        let mut events = Vec::new();
        let report = discover_using(
            "https://example.com",
            &mut |percent, _| events.push(percent),
            |_| vec!["127.0.0.1".parse().expect("loopback")],
            |_, _, _| panic!("Private target must not be fetched"),
        );
        assert!(report.observations.is_empty());
        assert_eq!(events, vec![0, 5]);
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn extracts_root_and_subdomains_only() {
        let found = extract_hosts(
            "https://WWW.Example.COM API.EXAMPLE.COM evil.example.net",
            "example.com",
        );
        assert_eq!(
            found.into_iter().collect::<Vec<_>>(),
            vec!["api.example.com", "www.example.com"]
        );
    }

    #[test]
    fn rejects_credentials_and_non_domain_input() {
        assert_eq!(
            host_from_url("https://user:secret@www.example.com/path"),
            None
        );
        assert_eq!(
            host_from_url("https://www.example.com/path"),
            Some("www.example.com".into())
        );
        assert_eq!(host_from_url("example.com"), None);
        assert_eq!(host_from_url("https://example.com:8443"), None);
        assert_eq!(host_from_url("https://example.com/?token=secret"), None);
        assert_eq!(host_from_url("https://127.0.0.1/"), None);
        assert_eq!(host_from_url("https://example.com\n/"), None);
    }

    #[test]
    fn extracted_hosts_are_valid_and_bounded() {
        let text = (0..500)
            .map(|n| format!("https://node{n}.example.com "))
            .collect::<String>();
        let found = extract_hosts(&text, "example.com");
        assert_eq!(found.len(), MAX_HOSTS);
        assert!(
            extract_hosts(
                "-bad.example.com a_b.example.com example.com.evil.net",
                "example.com"
            )
            .is_empty()
        );
    }

    #[test]
    fn rejects_private_and_reserved_ip_ranges() {
        for ip in [
            "10.0.0.1",
            "127.0.0.1",
            "172.16.0.1",
            "192.168.1.2",
            "169.254.169.254",
            "100.64.0.1",
            "fc00::1",
            "fe80::1",
            "::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!is_public_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(is_public_ip(ip.parse().unwrap()), "{ip}");
        }
    }
}

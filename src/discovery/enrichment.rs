//! Bounded public evidence collection. Every result stays pending for review.
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::{Read, Write},
    net::{IpAddr, SocketAddr, TcpStream, UdpSocket},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use base64::Engine;
use hickory_proto::{
    op::{Message, MessageType, ResponseCode},
    rr::{Name, RecordType},
};
use quick_xml::{Reader, events::Event};
use serde::{Deserialize, Serialize};
use url::Url;
use x509_parser::{extensions::GeneralName, prelude::FromDer};

use super::{
    DiscoveryObservation, DiscoveryReport, MAX_BODY_BYTES, MAX_HOSTS, extract_hosts, host_from_url,
    host_like, is_public_ip, registrable_candidate,
};

const MAX_REQUESTS: usize = 12;
const MAX_OBSERVATIONS: usize = 1000;
const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_META_BYTES: usize = 128 * 1024;
const DEADLINE: Duration = Duration::from_secs(180);
const MARKER: &str = "\nITGLA_CURL_META_V1:";

#[derive(Clone, Default)]
pub struct DiscoveryOptions {
    /// Explicit opt-in: discloses the registrable domain to crt.sh.
    pub certificate_transparency: bool,
    /// Applies only to the fixed CT provider. Target requests remain DNS-pinned/direct.
    pub ct_proxy: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct HttpEvidence {
    body: String,
    status: u16,
    headers: BTreeMap<String, String>,
    certificates: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DnsEvidence {
    version: u8,
    owner: String,
    record_type: String,
    value: String,
    ttl_seconds: u32,
    resolver: String,
    transport: String,
    dnssec_validated: bool,
}

fn pending(
    kind: &str,
    value: impl Into<String>,
    source: &str,
    evidence: impl Into<String>,
    confidence: u8,
) -> DiscoveryObservation {
    DiscoveryObservation {
        kind: kind.into(),
        value: value.into(),
        source: source.into(),
        evidence: evidence.into(),
        confidence,
        verification_status: "needs-review".into(),
    }
}

fn push(report: &mut DiscoveryReport, observation: DiscoveryObservation) {
    if report.observations.len() < MAX_OBSERVATIONS
        && observation.value.len() <= 2048
        && observation.evidence.len() <= 4096
        && !report.observations.iter().any(|item| {
            item.kind == observation.kind
                && item.value == observation.value
                && item.source == observation.source
        })
    {
        report.observations.push(observation);
    }
}

fn warn(report: &mut DiscoveryReport, warning: impl Into<String>) {
    if report.warnings.len() < 100 {
        report
            .warnings
            .push(warning.into().chars().take(4096).collect());
    }
}

fn command(name: &str) -> Command {
    let mut command = Command::new(name);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

/// Finite stdout plus a process watchdog, including a child that never closes stdout.
fn bounded_output(
    command: &mut Command,
    limit: usize,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Collector unavailable: {error}"))?;
    let child = Arc::new(std::sync::Mutex::new(child));
    let stdout = child
        .lock()
        .map_err(|_| "Collector lock failed")?
        .stdout
        .take()
        .ok_or("Missing collector output")?;
    let done = Arc::new(AtomicBool::new(false));
    let watcher_child = child.clone();
    let watcher_done = done.clone();
    let watcher = std::thread::spawn(move || {
        let deadline = Instant::now() + timeout;
        while !watcher_done.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }
        if !watcher_done.load(Ordering::Acquire)
            && let Ok(mut child) = watcher_child.lock()
        {
            let _ = child.kill();
        }
    });
    let mut bytes = Vec::new();
    let read = stdout.take(limit as u64 + 1).read_to_end(&mut bytes);
    if (read.is_err() || bytes.len() > limit)
        && let Ok(mut child) = child.lock()
    {
        let _ = child.kill();
    }
    let status = loop {
        let status = child
            .lock()
            .map_err(|_| "Collector lock failed")?
            .try_wait()
            .map_err(|_| "Collector wait failed")?;
        if let Some(status) = status {
            break status;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    done.store(true, Ordering::Release);
    let _ = watcher.join();
    if read.is_err() || bytes.len() > limit {
        return Err("Collector output exceeded its limit".into());
    }
    if !status.success() {
        return Err(format!(
            "Collector failed or timed out (status {:?})",
            status.code()
        ));
    }
    Ok(bytes)
}

fn http(url: &str, addresses: &[IpAddr]) -> Result<HttpEvidence, String> {
    let host = host_from_url(url).ok_or("Invalid public URL")?;
    if addresses.is_empty() || addresses.iter().any(|ip| !is_public_ip(*ip)) {
        return Err("HTTP target did not resolve exclusively to public addresses".into());
    }
    let mut cmd = command("curl");
    cmd.args([
        "--disable",
        "--silent",
        "--show-error",
        "--proto",
        "=http,https",
        "--connect-timeout",
        "4",
        "--max-time",
        "8",
        "--max-filesize",
        &MAX_BODY_BYTES.to_string(),
        "--user-agent",
        "ITGLA-discovery/0.2",
        "--include",
        "--write-out",
        &format!("{MARKER}%{{json}}"),
    ]);
    let port = if url.starts_with("http://") { 80 } else { 443 };
    let ip = addresses[0];
    let pin = if ip.is_ipv6() {
        format!("{host}:{port}:[{ip}]")
    } else {
        format!("{host}:{port}:{ip}")
    };
    cmd.args(["--noproxy", "*", "--resolve", &pin]);
    cmd.arg(url);
    let bytes = bounded_output(
        &mut cmd,
        MAX_BODY_BYTES + MAX_HEADER_BYTES + MAX_META_BYTES,
        Duration::from_secs(10),
    )?;
    parse_http(&bytes)
}

fn parse_http(bytes: &[u8]) -> Result<HttpEvidence, String> {
    let marker = bytes
        .windows(MARKER.len())
        .rposition(|window| window == MARKER.as_bytes())
        .ok_or("curl metadata unavailable; update system curl")?;
    if bytes.len() - marker > MAX_META_BYTES {
        return Err("HTTP metadata exceeded limit".into());
    }
    let metadata: serde_json::Value = serde_json::from_slice(&bytes[marker + MARKER.len()..])
        .map_err(|_| "Invalid curl metadata")?;
    let status = metadata
        .get("http_code")
        .and_then(|value| value.as_u64())
        .filter(|code| (100..=599).contains(code))
        .ok_or("Missing HTTP status")? as u16;
    let mut remaining = &bytes[..marker];
    let mut consumed = 0;
    let mut headers = BTreeMap::new();
    while remaining.starts_with(b"HTTP/") {
        let end = remaining
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or("Invalid HTTP headers")?;
        consumed += end + 4;
        if consumed > MAX_HEADER_BYTES {
            return Err("HTTP headers exceeded limit".into());
        }
        headers.clear();
        for line in String::from_utf8_lossy(&remaining[..end]).lines().skip(1) {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_ascii_lowercase();
                if matches!(
                    key.as_str(),
                    "server"
                        | "via"
                        | "cf-ray"
                        | "content-type"
                        | "location"
                        | "link"
                        | "cdn-cache-control"
                ) {
                    headers.insert(key, value.trim().chars().take(2048).collect());
                }
            }
        }
        remaining = &remaining[end + 4..];
    }
    if remaining.len() > MAX_BODY_BYTES {
        return Err("HTTP body exceeded limit".into());
    }
    Ok(HttpEvidence {
        body: String::from_utf8_lossy(remaining).into_owned(),
        status,
        headers,
        certificates: metadata
            .get("certs")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned(),
    })
}

/// Only public, query-free HTTP(S) URLs. Fragments have no request semantics.
fn public_link(base: &str, value: &str) -> Option<String> {
    if value.len() > 2048 || value.chars().any(char::is_control) || value.contains('\\') {
        return None;
    }
    let mut parsed = Url::parse(base).ok()?.join(value.trim()).ok()?;
    if parsed.query().is_some() || !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    parsed.set_fragment(None);
    let value = parsed.to_string();
    host_from_url(&value)?;
    Some(value)
}

fn attribute_values(body: &str) -> Vec<(String, bool)> {
    // Bounded HTML attribute tokenizer: quoted/unquoted href/src only; never executes HTML.
    let mut found = Vec::new();
    let lower = body.to_ascii_lowercase();
    let mut offset = 0;
    while offset < body.len() && found.len() < 128 {
        let Some(start) = lower[offset..].find('<') else {
            break;
        };
        let start = offset + start;
        let Some(end) = lower[start..].find('>') else {
            break;
        };
        let end = start + end;
        if end - start > 8192 {
            offset = end + 1;
            continue;
        }
        let tag = &body[start..end];
        let mut attributes = BTreeMap::new();
        let bytes = tag.as_bytes();
        let mut pos = 1;
        while pos < bytes.len() && found.len() < 128 {
            while pos < bytes.len() && !bytes[pos].is_ascii_alphabetic() {
                pos += 1;
            }
            let begin = pos;
            while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'-') {
                pos += 1;
            }
            let name = &tag[begin..pos];
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if pos >= bytes.len() || bytes[pos] != b'=' {
                continue;
            }
            pos += 1;
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if pos >= bytes.len() {
                break;
            }
            let quote = bytes[pos];
            let quoted = quote == b'\'' || quote == b'"';
            if quoted {
                pos += 1;
            }
            let value_start = pos;
            while pos < bytes.len()
                && if quoted {
                    bytes[pos] != quote
                } else {
                    !bytes[pos].is_ascii_whitespace()
                }
            {
                pos += 1;
            }
            if matches!(name.to_ascii_lowercase().as_str(), "href" | "src" | "rel")
                && let Ok(value) = quick_xml::escape::unescape(&tag[value_start..pos])
            {
                attributes.insert(name.to_ascii_lowercase(), value.into_owned());
            }
            if quoted && pos < bytes.len() {
                pos += 1;
            }
        }
        let canonical = attributes.get("rel").is_some_and(|value| {
            value
                .split_whitespace()
                .any(|part| part.eq_ignore_ascii_case("canonical"))
        });
        for key in ["href", "src"] {
            if let Some(value) = attributes.remove(key) {
                found.push((value, canonical && key == "href"));
            }
        }
        offset = start + (end - start) + 1;
        // The scan offsets are ASCII delimiters, hence UTF-8 boundaries.
    }
    found
}

fn sitemap_locations(body: &str) -> Result<(bool, Vec<String>), String> {
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);
    let mut in_location = false;
    let mut location = String::new();
    let mut index = false;
    let mut values = Vec::new();
    let mut depth = 0_usize;
    loop {
        match reader.read_event() {
            Ok(Event::Start(tag)) => {
                depth += 1;
                if depth > 32 {
                    return Err("Sitemap XML nesting exceeded limit".into());
                }
                if tag.local_name().as_ref() == "sitemapindex" {
                    index = true;
                }
                in_location = tag.local_name().as_ref() == "loc";
                if in_location {
                    location.clear();
                }
            }
            Ok(Event::Text(text)) if in_location => {
                let decoded = text.xml_content(quick_xml::XmlVersion::Implicit1_0);
                location.push_str(&decoded);
                if location.len() > 2048 {
                    return Err("Sitemap location exceeded limit".into());
                }
            }
            Ok(Event::CData(text)) if in_location => {
                location.push_str(&text.xml10_content());
            }
            Ok(Event::GeneralRef(entity)) if in_location => {
                let escaped = format!("&{};", &*entity);
                let decoded = quick_xml::escape::unescape(&escaped)
                    .map_err(|_| "Unsupported sitemap entity")?;
                location.push_str(&decoded);
            }
            Ok(Event::End(tag)) => {
                if in_location
                    && tag.local_name().as_ref() == "loc"
                    && values.len() < 128
                    && location.len() <= 2048
                {
                    values.push(location.trim().to_owned());
                }
                in_location = false;
                depth = depth.saturating_sub(1);
            }
            Ok(Event::DocType(_)) => return Err("Sitemap DTD is not supported".into()),
            Ok(Event::Eof) if depth != 0 => return Err("Incomplete sitemap XML".into()),
            Ok(Event::Eof) => break,
            Err(_) => return Err("Invalid sitemap XML".into()),
            _ => {}
        }
    }
    Ok((index, values))
}

fn reference(report: &mut DiscoveryReport, root: &str, value: &str, source: &str, origin: &str) {
    let Some(host) = host_from_url(value) else {
        return;
    };
    if host_like(&host, root) {
        if host != report.host {
            push(
                report,
                pending(
                    "subdomain-candidate",
                    &host,
                    source,
                    format!("Referenced by {source} at {origin}; relationship is not verified"),
                    60,
                ),
            );
        }
    } else {
        push(
            report,
            pending(
                "external-reference",
                &host,
                source,
                format!(
                    "Referenced at {origin}. External provider or unrelated reference. Not proof of ownership; not automatically fetched or promoted."
                ),
                40,
            ),
        );
    }
}

fn collect_response(
    report: &mut DiscoveryReport,
    root: &str,
    url: &str,
    response: &HttpEvidence,
    queue: &mut VecDeque<(String, u8)>,
    depth: u8,
) {
    let source = if depth > 0 { "sitemap" } else { "http" };
    let headers = response
        .headers
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "location" | "link"))
        .map(|(key, value)| format!("{key}: {}", value.chars().take(256).collect::<String>()))
        .collect::<Vec<_>>()
        .join("\n");
    push(
        report,
        pending(
            "http-evidence",
            url,
            source,
            format!(
                "HTTP {}\n{headers}\nServer/CDN headers are self-reported; they do not establish software versions or origins.",
                response.status
            ),
            75,
        ),
    );
    if let Some(location) = response
        .headers
        .get("location")
        .and_then(|value| public_link(url, value))
    {
        reference(report, root, &location, "http-redirect", url);
        push(
            report,
            pending(
                "link-evidence",
                location,
                "http-redirect",
                format!("Location from {url}. Redirect was not followed."),
                65,
            ),
        );
    }
    if !(200..300).contains(&response.status) {
        return;
    }
    for (value, canonical) in attribute_values(&response.body) {
        if let Some(link) = public_link(url, &value) {
            let source = if canonical {
                "html-canonical"
            } else {
                "html-link"
            };
            reference(report, root, &link, source, url);
            if canonical {
                push(
                    report,
                    pending(
                        "link-evidence",
                        link,
                        source,
                        format!(
                            "Canonical declaration from {url}; a declaration, not a verified deployment relationship"
                        ),
                        65,
                    ),
                );
            }
        }
    }
    if let Some(link_header) = response.headers.get("link") {
        for part in link_header.split(',').take(16) {
            if let Some((_, tail)) = part.split_once('<')
                && let Some((value, _)) = tail.split_once('>')
                && let Some(link) = public_link(url, value)
            {
                reference(report, root, &link, "http-link", url);
                push(
                    report,
                    pending(
                        "link-evidence",
                        link,
                        "http-link",
                        format!("Link header from {url}; not followed automatically"),
                        60,
                    ),
                );
            }
        }
    }
    let looks_xml = response.body.trim_start().starts_with('<')
        && (url.ends_with(".xml")
            || response
                .headers
                .get("content-type")
                .is_some_and(|value| value.contains("xml")));
    if looks_xml {
        match sitemap_locations(&response.body) {
            Ok((is_index, locations)) => {
                for value in locations {
                    if let Some(link) = public_link(url, &value) {
                        reference(report, root, &link, "sitemap", url);
                        if is_index
                            && depth < 2
                            && host_from_url(&link).is_some_and(|host| host_like(&host, root))
                            && queue.len() < MAX_REQUESTS
                        {
                            queue.push_back((link, depth + 1));
                        }
                    }
                }
            }
            Err(error) => warn(report, format!("{url}: {error}")),
        }
    }
    if url.ends_with("/robots.txt") {
        for line in response.body.lines().take(2048) {
            if let Some((key, value)) = line.split_once(':')
                && key.trim().eq_ignore_ascii_case("sitemap")
                && let Some(link) = public_link(url, value)
            {
                reference(report, root, &link, "robots-sitemap", url);
                if host_from_url(&link).is_some_and(|host| host_like(&host, root))
                    && queue.len() < MAX_REQUESTS
                {
                    queue.push_back((link, 1));
                }
            }
        }
    }
}

fn certificate_name(
    report: &mut DiscoveryReport,
    root: &str,
    url: &str,
    name: &str,
    names: &mut Vec<String>,
) {
    let value = name.to_ascii_lowercase();
    if let Some(suffix) = value.strip_prefix("*.") {
        if host_from_url(&format!("https://{suffix}")).is_some() {
            push(
                report,
                pending(
                    "certificate-pattern",
                    &value,
                    "tls-san",
                    format!("Wildcard SAN on {url}; pattern, not a concrete host"),
                    60,
                ),
            );
        }
    } else if host_from_url(&format!("https://{value}")).is_some() {
        names.push(value.clone());
        reference(report, root, &format!("https://{value}"), "tls-san", url);
    }
}

fn certificate(
    report: &mut DiscoveryReport,
    root: &str,
    url: &str,
    certs: &str,
) -> Result<(), String> {
    let pem = certs
        .split_once("-----BEGIN CERTIFICATE-----")
        .and_then(|(_, tail)| tail.split_once("-----END CERTIFICATE-----"))
        .map(|(pem, _)| pem)
        .ok_or(
            "System curl did not provide a leaf certificate; update curl for certificate discovery",
        )?;
    if pem.len() > 64 * 1024 {
        return Err("Leaf certificate exceeded limit".into());
    }
    let compact = pem
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>();
    let der = base64::engine::general_purpose::STANDARD
        .decode(compact)
        .map_err(|_| "Invalid certificate PEM")?;
    let (remaining, cert) = x509_parser::certificate::X509Certificate::from_der(&der)
        .map_err(|_| "Invalid X.509 certificate")?;
    if !remaining.is_empty() {
        return Err("Trailing certificate data".into());
    }
    let mut names = Vec::new();
    if let Some(san) = cert
        .subject_alternative_name()
        .map_err(|_| "Invalid certificate SAN extension")?
    {
        for name in san.value.general_names.iter().take(64) {
            if let GeneralName::DNSName(name) = name {
                certificate_name(report, root, url, name, &mut names);
            }
        }
    }
    let utc = |time: x509_parser::time::ASN1Time| {
        let value = time.to_datetime();
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            value.year(),
            u8::from(value.month()),
            value.day(),
            value.hour(),
            value.minute(),
            value.second()
        )
    };
    let evidence = serde_json::json!({"version":1,"subject":cert.subject().to_string(),"issuer":cert.issuer().to_string(),"serial":cert.raw_serial_as_string(),"not_before_utc":utc(cert.validity().not_before),"not_after_utc":utc(cert.validity().not_after),"dns_names":names.iter().take(8).collect::<Vec<_>>(),"dns_names_observed":names.len(),"dns_names_summary_limit":8,"validation":"Curl verified trust and hostname; metadata parser does not perform independent signature verification"}).to_string();
    push(report, pending("tls-certificate", url, "tls", evidence, 85));
    Ok(())
}

fn system_resolver() -> Result<SocketAddr, String> {
    #[cfg(target_os = "windows")]
    let config = {
        let bytes = bounded_output(command("powershell.exe").args(["-NoLogo","-NoProfile","-NonInteractive","-Command","Get-DnsClientServerAddress | ForEach-Object { $_.ServerAddresses } | Select-Object -Unique"]),8192,Duration::from_secs(5))?;
        String::from_utf8_lossy(&bytes).into_owned()
    };
    #[cfg(not(target_os = "windows"))]
    let config = std::fs::read_to_string("/etc/resolv.conf")
        .map_err(|_| "Cannot read system DNS resolver configuration")?
        .lines()
        .filter_map(|line| {
            line.strip_prefix("nameserver")
                .and_then(|value| value.split_whitespace().next())
        })
        .collect::<Vec<_>>()
        .join("\n");
    config
        .lines()
        .filter_map(|value| value.trim().parse::<IpAddr>().ok())
        .find(|ip| !ip.is_unspecified() && !ip.is_multicast())
        .map(|ip| SocketAddr::new(ip, 53))
        .ok_or("No usable system DNS resolver; OS address lookups remain available".into())
}

fn validate_dns(bytes: &[u8], request: &Message) -> Result<Message, String> {
    let response = Message::from_vec(bytes).map_err(|_| "Invalid DNS response")?;
    if response.metadata.id != request.metadata.id
        || response.metadata.message_type != MessageType::Response
        || response.metadata.op_code != request.metadata.op_code
        || response.queries.len() != request.queries.len()
        || !response
            .queries
            .iter()
            .zip(&request.queries)
            .all(|(answer, question)| {
                answer.query_type == question.query_type
                    && answer.query_class == question.query_class
                    && answer
                        .name
                        .to_ascii()
                        .trim_end_matches('.')
                        .eq_ignore_ascii_case(question.name.to_ascii().trim_end_matches('.'))
            })
    {
        return Err("DNS response did not match its transaction and question".into());
    }
    if !matches!(
        response.metadata.response_code,
        ResponseCode::NoError | ResponseCode::NXDomain
    ) {
        return Err(format!(
            "DNS resolver returned {}",
            response.metadata.response_code
        ));
    }
    Ok(response)
}

fn dns_query(
    host: &str,
    record_type: RecordType,
    resolver: SocketAddr,
) -> Result<Vec<DnsEvidence>, String> {
    let name = Name::from_ascii(host).map_err(|_| "Invalid DNS owner")?;
    let mut request = Message::query();
    request.metadata.recursion_desired = true;
    request.add_query(hickory_proto::op::Query::query(name, record_type));
    let bytes = request.to_vec().map_err(|_| "Cannot encode DNS query")?;
    let socket = UdpSocket::bind(if resolver.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    })
    .map_err(|_| "Cannot open DNS socket")?;
    socket
        .connect(resolver)
        .map_err(|_| "Cannot connect to system DNS resolver")?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| "Cannot set DNS timeout")?;
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| "Cannot set DNS timeout")?;
    socket.send(&bytes).map_err(|_| "Cannot send DNS query")?;
    let mut buffer = [0u8; 65535];
    let size = socket
        .recv(&mut buffer)
        .map_err(|_| "System DNS query timed out or failed")?;
    let mut response = validate_dns(&buffer[..size], &request)?;
    let mut transport = "udp";
    if response.metadata.truncation {
        let mut stream = TcpStream::connect_timeout(&resolver, Duration::from_secs(2))
            .map_err(|_| "Truncated DNS response; TCP fallback failed")?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| "Cannot set DNS timeout")?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| "Cannot set DNS timeout")?;
        stream
            .write_all(&(bytes.len() as u16).to_be_bytes())
            .and_then(|_| stream.write_all(&bytes))
            .map_err(|_| "DNS TCP write failed")?;
        let mut length = [0; 2];
        stream
            .read_exact(&mut length)
            .map_err(|_| "DNS TCP length unavailable")?;
        let mut body = vec![0; u16::from_be_bytes(length) as usize];
        stream
            .read_exact(&mut body)
            .map_err(|_| "DNS TCP read failed")?;
        response = validate_dns(&body, &request)?;
        if response.metadata.truncation {
            return Err("DNS TCP response was truncated".into());
        }
        transport = "tcp";
    }
    if response.metadata.response_code == ResponseCode::NXDomain {
        return Ok(Vec::new());
    }
    // Only retain answers reachable from the actual question through CNAMEs.
    let mut owners = BTreeSet::from([host.trim_end_matches('.').to_ascii_lowercase()]);
    for _ in 0..16 {
        let before = owners.len();
        for record in response.answers.iter().take(64) {
            if owners.contains(
                &record
                    .name
                    .to_ascii()
                    .trim_end_matches('.')
                    .to_ascii_lowercase(),
            ) && record.record_type() == RecordType::CNAME
            {
                owners.insert(
                    record
                        .data
                        .to_string()
                        .trim_end_matches('.')
                        .to_ascii_lowercase(),
                );
            }
        }
        if owners.len() == before {
            break;
        }
    }
    let mut records = Vec::new();
    for record in response.answers.iter().take(64) {
        if !owners.contains(
            &record
                .name
                .to_ascii()
                .trim_end_matches('.')
                .to_ascii_lowercase(),
        ) || (record.record_type() != record_type && record.record_type() != RecordType::CNAME)
        {
            continue;
        }
        records.push(DnsEvidence {
            version: 1,
            owner: record.name.to_ascii().trim_end_matches('.').to_owned(),
            record_type: record.record_type().to_string(),
            value: record.data.to_string().chars().take(2048).collect(),
            ttl_seconds: record.ttl,
            resolver: resolver.to_string(),
            transport: transport.into(),
            dnssec_validated: false,
        });
    }
    Ok(records)
}

fn ct_names(body: &str, root: &str) -> Result<BTreeSet<String>, String> {
    let entries: serde_json::Value =
        serde_json::from_str(body).map_err(|_| "Invalid certificate transparency response")?;
    let entries = entries
        .as_array()
        .ok_or("Certificate transparency response was not a list")?;
    let mut names = BTreeSet::new();
    for entry in entries.iter().take(1000) {
        if let Some(value) = entry.get("name_value").and_then(|value| value.as_str()) {
            for name in value.lines().take(64) {
                let name = name.trim().trim_end_matches('.').to_ascii_lowercase();
                if !name.starts_with("*.")
                    && host_like(&name, root)
                    && host_from_url(&format!("https://{name}")).is_some()
                    && names.len() < MAX_HOSTS
                {
                    names.insert(name);
                }
            }
        }
    }
    Ok(names)
}

fn follow_sitemaps(
    report: &mut DiscoveryReport,
    responses: &[(String, HttpEvidence)],
    lookup: impl Fn(&str) -> Vec<IpAddr>,
    fetch: impl Fn(&str, &[IpAddr]) -> Result<HttpEvidence, String>,
    started: Instant,
    progress: &mut impl FnMut(u8, &str),
) {
    let root = registrable_candidate(&report.host);
    let mut queue = VecDeque::new();
    let mut visited = BTreeSet::new();
    for (url, response) in responses {
        visited.insert(url.clone());
        collect_response(report, &root, url, response, &mut queue, 0);
        if url == &report.input_url
            && url.starts_with("https://")
            && let Err(error) = certificate(report, &root, url, &response.certificates)
        {
            warn(report, error);
        }
    }
    let mut requests = 5;
    while let Some((url, depth)) = queue.pop_front() {
        if requests >= MAX_REQUESTS || started.elapsed() >= DEADLINE {
            warn(
                report,
                "Sitemap request/deadline limit reached; remaining links were not fetched",
            );
            break;
        }
        if !visited.insert(url.clone()) {
            continue;
        }
        let Some(host) = host_from_url(&url) else {
            continue;
        };
        if !host_like(&host, &root) {
            continue;
        }
        progress(
            55 + ((requests - 5) * 12 / 7) as u8,
            &format!("Reading sitemap {requests} of {MAX_REQUESTS}..."),
        );
        let addresses = lookup(&host);
        requests += 1;
        if addresses.is_empty() || addresses.iter().any(|ip| !is_public_ip(*ip)) {
            warn(report, format!("{url}: public address validation failed"));
            continue;
        }
        match fetch(&url, &addresses) {
            Ok(response) => {
                if (200..300).contains(&response.status) {
                    push(
                        report,
                        pending(
                            "public-endpoint",
                            &url,
                            "sitemap",
                            "HTTP endpoint responded; review evidence before adding assets",
                            75,
                        ),
                    );
                }
                for host in extract_hosts(&response.body, &root) {
                    if host != report.host {
                        push(
                            report,
                            pending(
                                "subdomain-candidate",
                                host,
                                "sitemap",
                                format!("Found in {url}"),
                                60,
                            ),
                        );
                    }
                }
                collect_response(report, &root, &url, &response, &mut queue, depth);
            }
            Err(error) => warn(report, format!("{url}: {error}")),
        }
    }
}

pub(super) fn discover(
    input: &str,
    options: DiscoveryOptions,
    progress: &mut impl FnMut(u8, &str),
) -> DiscoveryReport {
    let started = Instant::now();
    let responses = RefCell::new(Vec::new());
    let looked_up = RefCell::new(BTreeMap::<String, Vec<IpAddr>>::new());
    let resolve = |host: &str| {
        if let Some(addresses) = looked_up.borrow().get(host) {
            return addresses.clone();
        }
        if started.elapsed() >= DEADLINE || looked_up.borrow().len() > MAX_HOSTS {
            return Vec::new();
        }
        let addresses = super::resolve(host);
        looked_up
            .borrow_mut()
            .insert(host.into(), addresses.clone());
        addresses
    };
    let mut report = super::discover_using(
        input,
        &mut |percent, message| progress((u16::from(percent) * 55 / 95) as u8, message),
        resolve,
        |url, _host, addresses| {
            if started.elapsed() >= DEADLINE {
                return Err("Discovery deadline reached".into());
            }
            let response = http(url, addresses)?;
            let body = response.body.clone();
            let status = response.status;
            responses.borrow_mut().push((url.to_owned(), response));
            if (200..300).contains(&status) {
                Ok(body)
            } else {
                Err(format!(
                    "HTTP {status}; redirects were recorded, not followed"
                ))
            }
        },
    );
    if report.host.is_empty() || report.observations.is_empty() {
        return report;
    }
    let root = registrable_candidate(&report.host);
    push(
        &mut report,
        pending(
            "discovery-scope",
            &root,
            "public-suffix-list",
            "Offline public/private suffix rules. Shared-hosting tenants stay separate. Scope does not establish ownership.",
            90,
        ),
    );
    follow_sitemaps(
        &mut report,
        &responses.into_inner(),
        resolve,
        http,
        started,
        progress,
    );
    progress(68, "Reading typed DNS records from the system resolver...");
    if started.elapsed() < DEADLINE {
        match system_resolver() {
            Ok(resolver) => {
                for (index, kind) in [
                    RecordType::A,
                    RecordType::AAAA,
                    RecordType::CNAME,
                    RecordType::MX,
                    RecordType::NS,
                    RecordType::TXT,
                    RecordType::CAA,
                    RecordType::SRV,
                ]
                .into_iter()
                .enumerate()
                {
                    if started.elapsed() >= DEADLINE {
                        break;
                    }
                    progress(68 + index as u8 * 2, &format!("Reading {kind} records..."));
                    let owner = if matches!(
                        kind,
                        RecordType::MX | RecordType::NS | RecordType::TXT | RecordType::CAA
                    ) {
                        root.as_str()
                    } else {
                        report.host.as_str()
                    };
                    match dns_query(owner, kind, resolver) {
                        Ok(records) => {
                            for record in records {
                                let value = format!(
                                    "{} {} {}",
                                    record.owner, record.record_type, record.value
                                );
                                let evidence = serde_json::to_string(&record).unwrap_or_default();
                                push(
                                    &mut report,
                                    pending("dns-record", value, "system-dns", evidence, 80),
                                );
                                if record.record_type == "CNAME" {
                                    reference(
                                        &mut report,
                                        &root,
                                        &format!("https://{}", record.value.trim_end_matches('.')),
                                        "dns-cname",
                                        &record.owner,
                                    );
                                }
                            }
                        }
                        Err(error) => warn(&mut report, format!("DNS {kind}: {error}")),
                    }
                }
            }
            Err(error) => warn(&mut report, error),
        }
    }
    progress(85, "Checking optional certificate transparency...");
    if options.certificate_transparency && started.elapsed() < DEADLINE {
        let url = format!("https://crt.sh/?q=%25.{root}&output=json");
        // Fixed provider query is constructed here; user input cannot change the host.
        // The ordinary URL guard rejects queries; provider transport has a dedicated guard below.
        match ct_fetch(&url, options.ct_proxy.as_deref()).and_then(|body| ct_names(&body, &root)) {
            Ok(names) => {
                for host in names {
                    push(
                        &mut report,
                        pending(
                            "subdomain-candidate",
                            host,
                            "certificate-transparency",
                            "Historical crt.sh certificate entry; not proof of current DNS, reachability or ownership",
                            45,
                        ),
                    );
                }
            }
            Err(error) => warn(&mut report, format!("Certificate transparency: {error}")),
        }
    }
    let hosts = report
        .observations
        .iter()
        .filter(|item| item.kind == "subdomain-candidate")
        .map(|item| item.value.clone())
        .collect::<BTreeSet<_>>();
    for (index, host) in hosts.iter().take(MAX_HOSTS).enumerate() {
        if started.elapsed() >= DEADLINE {
            break;
        }
        progress(
            86 + (index * 8 / hosts.len().max(1)) as u8,
            &format!("Resolving discovered name {host}..."),
        );
        for ip in resolve(host) {
            push(
                &mut report,
                pending(
                    "ip-candidate",
                    ip.to_string(),
                    "dns-enrichment",
                    format!(
                        "A/AAAA lookup for {host}; may be CDN/shared hosting, not a confirmed origin"
                    ),
                    70,
                ),
            );
        }
    }
    if started.elapsed() >= DEADLINE {
        warn(
            &mut report,
            "Discovery deadline reached. Partial evidence was retained; retry to collect remaining evidence.",
        );
    }
    if report.observations.len() >= MAX_OBSERVATIONS {
        warn(
            &mut report,
            "Evidence limit reached; narrow the URL scope for another run.",
        );
    }
    progress(95, "Preparing enriched evidence for review...");
    report
}

fn ct_fetch(url: &str, proxy: Option<&str>) -> Result<String, String> {
    let parsed = Url::parse(url).map_err(|_| "Invalid CT provider URL")?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("crt.sh")
        || parsed.path() != "/"
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("Invalid CT provider target".into());
    }
    let mut cmd = command("curl");
    cmd.args([
        "--disable",
        "--silent",
        "--show-error",
        "--fail",
        "--proto",
        "=https",
        "--max-time",
        "8",
        "--connect-timeout",
        "4",
        "--max-filesize",
        &MAX_BODY_BYTES.to_string(),
    ]);
    if let Some(proxy) = proxy {
        let parsed = Url::parse(proxy).map_err(|_| "Invalid CT proxy")?;
        if !matches!(parsed.scheme(), "http" | "https" | "socks5h")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err("Invalid CT proxy".into());
        }
        cmd.args(["--proxy", proxy, "--noproxy", ""]);
    } else {
        cmd.args(["--noproxy", "*"]);
    }
    cmd.arg(url);
    let bytes = bounded_output(&mut cmd, MAX_BODY_BYTES, Duration::from_secs(10))?;
    String::from_utf8(bytes).map_err(|_| "Invalid CT response encoding".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use hickory_proto::rr::{RData, Record, rdata::A};

    fn report() -> DiscoveryReport {
        DiscoveryReport {
            input_url: "http://example.com/".into(),
            host: "example.com".into(),
            observations: vec![],
            warnings: vec![],
        }
    }
    fn response(body: impl Into<String>) -> HttpEvidence {
        HttpEvidence {
            status: 200,
            body: body.into(),
            ..Default::default()
        }
    }
    fn index(urls: &[&str]) -> HttpEvidence {
        let mut result = response(format!(
            "<sitemapindex>{}</sitemapindex>",
            urls.iter()
                .map(|url| format!("<sitemap><loc>{url}</loc></sitemap>"))
                .collect::<String>()
        ));
        result
            .headers
            .insert("content-type".into(), "application/xml".into());
        result
    }

    #[test]
    fn suffix_scope_separates_shared_hosting_tenants() {
        for (host, root) in [
            ("www.shop.example.co.uk", "example.co.uk"),
            ("www.example.com.cn", "example.com.cn"),
            ("www.alice.github.io", "alice.github.io"),
            ("bob.github.io", "bob.github.io"),
            ("www.example.com", "example.com"),
        ] {
            assert_eq!(registrable_candidate(host), root);
        }
        assert!(!host_like(
            "bob.github.io",
            &registrable_candidate("alice.github.io")
        ));
    }

    #[test]
    fn links_join_paths_but_reject_secrets_protocols_and_private_targets() {
        assert_eq!(
            public_link("https://example.com/docs/start", "../guide#top").as_deref(),
            Some("https://example.com/guide")
        );
        for value in [
            "?token=secret",
            "https://user:pass@example.com/",
            "javascript:alert(1)",
            "ftp://example.com/",
            "http://127.0.0.1/",
            "https://example.com:8080/",
            "https://example.com/\\private",
            "\n/path",
        ] {
            assert!(
                public_link("https://example.com/", value).is_none(),
                "{value}"
            );
        }
        assert!(public_link("https://example.com/", "https://\u{5916}\u{90e8}.example/").is_some());
    }

    #[test]
    fn http_parser_preserves_final_status_and_drops_sensitive_headers() {
        let metadata = serde_json::json!({"http_code":302,"certs":"leaf"});
        let wire = format!(
            "HTTP/1.1 100 Continue\r\n\r\nHTTP/2 302 Found\r\nServer: nginx\r\nLocation: https://vendor.example/\r\nSet-Cookie: secret\r\nAuthorization: secret\r\n\r\nbody{MARKER}{metadata}"
        );
        let parsed = parse_http(wire.as_bytes()).unwrap();
        assert_eq!(parsed.status, 302);
        assert_eq!(parsed.body, "body");
        assert_eq!(parsed.certificates, "leaf");
        assert_eq!(parsed.headers.len(), 2);
        assert!(!parsed.headers.contains_key("set-cookie"));
        assert!(parse_http(b"HTTP/1.1 200 OK\r\n\r\nbody").is_err());
        assert!(
            parse_http(
                format!(
                    "HTTP/1.1 200 OK\r\n\r\n{}{MARKER}{{\"http_code\":200}}",
                    "x".repeat(MAX_BODY_BYTES + 1)
                )
                .as_bytes()
            )
            .is_err()
        );
    }

    #[test]
    fn html_handles_canonical_entities_unicode_and_oversized_tags() {
        let html = "<link rel='alternate canonical' href='/home&#47;\u{4e2d}\u{6587}'><script src=//cdn.vendor.example/app.js></script>";
        let links = attribute_values(html);
        assert!(links.contains(&("/home/\u{4e2d}\u{6587}".into(), true)));
        assert!(links.contains(&("//cdn.vendor.example/app.js".into(), false)));
        assert!(attribute_values(&format!("<a title='{}'>", "\u{6587}".repeat(9000))).is_empty());
    }

    #[test]
    fn sitemap_supports_namespaces_entities_and_cdata() {
        let (is_index, urls)=sitemap_locations("<s:sitemapindex xmlns:s='urn:sitemap'><s:sitemap><s:loc>https://example.com/a&#47;b</s:loc></s:sitemap><s:sitemap><s:loc><![CDATA[https://example.com/c]]></s:loc></s:sitemap></s:sitemapindex>").unwrap();
        assert!(is_index);
        assert_eq!(
            urls,
            vec!["https://example.com/a/b", "https://example.com/c"]
        );
        assert!(sitemap_locations("<!DOCTYPE a [<!ENTITY x 'secret'>]><urlset/>").is_err());
        assert!(sitemap_locations("<urlset><url><loc>unfinished").is_err());
        assert!(sitemap_locations(&format!("{}{}", "<x>".repeat(33), "</x>".repeat(33))).is_err());
        assert!(
            sitemap_locations(&format!("<urlset><loc>{}</loc></urlset>", "a".repeat(2049)))
                .is_err()
        );
        let xml = format!(
            "<urlset>{}</urlset>",
            "<url><loc>https://example.com/</loc></url>".repeat(200)
        );
        assert_eq!(sitemap_locations(&xml).unwrap().1.len(), 128);
    }

    #[test]
    fn sitemap_walk_deduplicates_cycles_limits_depth_and_keeps_partial_results() {
        let mut result = report();
        let calls = RefCell::new(Vec::new());
        let initial = vec![(
            "http://example.com/sitemap.xml".into(),
            index(&[
                "/one.xml",
                "/fail.xml",
                "https://vendor.example/external.xml",
            ]),
        )];
        follow_sitemaps(
            &mut result,
            &initial,
            |_| vec!["8.8.8.8".parse().unwrap()],
            |url, _| {
                calls.borrow_mut().push(url.to_owned());
                if url.ends_with("fail.xml") {
                    return Err("fixture failure".into());
                }
                if url.ends_with("one.xml") {
                    Ok(index(&["/sitemap.xml", "/two.xml"]))
                } else {
                    Ok(index(&["/three.xml"]))
                }
            },
            Instant::now(),
            &mut |_, _| {},
        );
        let calls = calls.into_inner();
        assert_eq!(
            calls,
            vec![
                "http://example.com/one.xml",
                "http://example.com/fail.xml",
                "http://example.com/two.xml"
            ]
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|v| v.contains("fixture failure"))
        );
        assert!(
            result
                .observations
                .iter()
                .any(|v| v.kind == "external-reference" && v.value == "vendor.example")
        );
        assert!(
            result
                .observations
                .iter()
                .all(|v| v.verification_status == "needs-review")
        );
    }

    #[test]
    fn sitemap_walk_refuses_private_and_mixed_addresses_before_fetch() {
        for addresses in [
            vec!["127.0.0.1".parse().unwrap()],
            vec!["8.8.8.8".parse().unwrap(), "192.168.1.7".parse().unwrap()],
        ] {
            let mut result = report();
            follow_sitemaps(
                &mut result,
                &[(
                    "http://example.com/sitemap.xml".into(),
                    index(&["/child.xml"]),
                )],
                |_| addresses.clone(),
                |_, _| panic!("private address reached transport"),
                Instant::now(),
                &mut |_, _| {},
            );
            assert!(
                result
                    .warnings
                    .iter()
                    .any(|v| v.contains("public address validation failed"))
            );
        }
    }

    #[test]
    fn sitemap_walk_bounds_request_count_and_deadline() {
        let initial = vec![(
            "http://example.com/sitemap.xml".into(),
            index(
                &(0..20)
                    .map(|i| format!("/{i}.xml"))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            ),
        )];
        let calls = RefCell::new(0);
        let mut result = report();
        follow_sitemaps(
            &mut result,
            &initial,
            |_| vec!["8.8.8.8".parse().unwrap()],
            |_, _| {
                *calls.borrow_mut() += 1;
                Ok(response("<urlset/>"))
            },
            Instant::now(),
            &mut |_, _| {},
        );
        assert_eq!(*calls.borrow(), 7);
        assert!(result.warnings.iter().any(|v| v.contains("limit reached")));
        follow_sitemaps(
            &mut report(),
            &initial,
            |_| panic!("deadline ignored"),
            |_, _| panic!("deadline ignored"),
            Instant::now() - DEADLINE,
            &mut |_, _| {},
        );
    }

    #[test]
    fn redirects_and_external_canonical_are_evidence_never_traversed() {
        let mut result = report();
        let mut redirect = response("");
        redirect.status = 302;
        redirect
            .headers
            .insert("location".into(), "https://vendor.example/login".into());
        let initial = vec![
            ("http://example.com/".into(), redirect),
            (
                "http://example.com/page".into(),
                response("<link rel='canonical' href='https://another.example/home'>"),
            ),
        ];
        follow_sitemaps(
            &mut result,
            &initial,
            |_| panic!("external lookup"),
            |_, _| panic!("external fetch"),
            Instant::now(),
            &mut |_, _| {},
        );
        assert_eq!(
            result
                .observations
                .iter()
                .filter(|v| v.kind == "external-reference")
                .count(),
            2
        );
        assert!(
            result
                .observations
                .iter()
                .any(|v| v.source == "html-canonical" && v.kind == "link-evidence")
        );
        assert!(
            !result
                .observations
                .iter()
                .any(|v| v.kind == "subdomain-candidate")
        );
    }

    #[test]
    fn leaf_certificate_records_dates_issuer_and_scoped_sans() {
        let mut result = report();
        result.host = "itgla.com".into();
        certificate(
            &mut result,
            "itgla.com",
            "https://itgla.com/",
            include_str!("fixtures/itgla-leaf-20261114.pem"),
        )
        .unwrap();
        let leaf = result
            .observations
            .iter()
            .find(|v| v.kind == "tls-certificate")
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&leaf.evidence).unwrap();
        assert!(json["issuer"].as_str().unwrap().contains("Let's Encrypt"));
        assert_eq!(json["not_after_utc"], "2026-12-22 14:11:33");
        assert!(
            result
                .observations
                .iter()
                .any(|v| v.kind == "subdomain-candidate" && v.value == "api.itgla.com")
        );
        let mut external = report();
        certificate(
            &mut external,
            "example.com",
            "https://example.com/",
            include_str!("fixtures/itgla-leaf-20261114.pem"),
        )
        .unwrap();
        assert!(
            !external
                .observations
                .iter()
                .any(|v| v.kind == "subdomain-candidate")
        );
        assert!(
            external
                .observations
                .iter()
                .any(|v| v.kind == "external-reference")
        );
        for pem in [
            "no cert".into(),
            "-----BEGIN CERTIFICATE-----invalid-----END CERTIFICATE-----".into(),
            format!(
                "-----BEGIN CERTIFICATE-----{}-----END CERTIFICATE-----",
                "a".repeat(65537)
            ),
        ] {
            assert!(
                certificate(&mut report(), "example.com", "https://example.com/", &pem).is_err()
            );
        }
    }

    #[test]
    fn typed_evidence_summary_is_readable_without_rewriting_source() {
        let evidence=serde_json::json!({"owner":"example.com","record_type":"A","value":"8.8.8.8","ttl_seconds":123,"resolver":"192.168.1.1:53","transport":"udp"}).to_string();
        let item = pending(
            "dns-record",
            "example.com A 8.8.8.8",
            "system-dns",
            &evidence,
            80,
        );
        let summary = super::super::evidence_summary(&item);
        assert!(summary.contains("TTL: 123 seconds"));
        assert!(summary.contains("Resolver: 192.168.1.1:53"));
        assert_eq!(item.evidence, evidence);
    }

    #[test]
    fn certificate_wildcards_remain_patterns_and_external_names_remain_references() {
        let mut result = report();
        let mut names = Vec::new();
        certificate_name(
            &mut result,
            "example.com",
            "https://example.com/",
            "*.example.com",
            &mut names,
        );
        certificate_name(
            &mut result,
            "example.com",
            "https://example.com/",
            "www.vendor.example",
            &mut names,
        );
        certificate_name(
            &mut result,
            "example.com",
            "https://example.com/",
            "*.*.example.com",
            &mut names,
        );
        assert_eq!(result.observations.len(), 2);
        assert_eq!(result.observations[0].kind, "certificate-pattern");
        assert_eq!(result.observations[1].kind, "external-reference");
        assert!(
            !result
                .observations
                .iter()
                .any(|v| v.kind == "subdomain-candidate")
        );
    }

    #[test]
    fn ct_is_opt_in_historical_bounded_and_scoped() {
        assert!(!DiscoveryOptions::default().certificate_transparency);
        let body=serde_json::json!([{"name_value":"api.example.com\n*.example.com\napi.example.com\nevil-example.com\nvendor.example\nEXAMPLE.COM."}]).to_string();
        assert_eq!(
            ct_names(&body, "example.com").unwrap(),
            BTreeSet::from(["api.example.com".into(), "example.com".into()])
        );
        assert!(ct_names("{}", "example.com").is_err());
        let body=serde_json::json!([{"name_value":(0..100).map(|i|format!("n{i}.example.com")).collect::<Vec<_>>().join("\n")}]).to_string();
        assert_eq!(ct_names(&body, "example.com").unwrap().len(), MAX_HOSTS);
    }

    fn answer(request: &Message) -> Message {
        let mut response = request.clone();
        response.metadata.message_type = MessageType::Response;
        response.answers.push(Record::from_rdata(
            Name::from_ascii("example.com").unwrap(),
            321,
            RData::A(A::new(8, 8, 8, 8)),
        ));
        response.answers.push(Record::from_rdata(
            Name::from_ascii("unrelated.example").unwrap(),
            111,
            RData::A(A::new(9, 9, 9, 9)),
        ));
        response
    }

    #[test]
    fn dns_rejects_mismatched_transactions_and_questions() {
        let mut request = Message::query();
        request.add_query(hickory_proto::op::Query::query(
            Name::from_ascii("example.com").unwrap(),
            RecordType::A,
        ));
        let valid = answer(&request);
        assert!(validate_dns(&valid.to_vec().unwrap(), &request).is_ok());
        let mut invalid = valid.clone();
        invalid.metadata.id = invalid.metadata.id.wrapping_add(1);
        assert!(validate_dns(&invalid.to_vec().unwrap(), &request).is_err());
        let mut invalid = valid.clone();
        invalid.queries.clear();
        assert!(validate_dns(&invalid.to_vec().unwrap(), &request).is_err());
        let mut invalid = valid;
        invalid.metadata.response_code = ResponseCode::ServFail;
        assert!(validate_dns(&invalid.to_vec().unwrap(), &request).is_err());
        assert!(validate_dns(b"broken", &request).is_err());
    }

    #[test]
    fn dns_udp_retains_ttl_resolver_and_omits_unrelated_answers() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let address = server.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let mut buffer = [0; 4096];
            let (size, peer) = server.recv_from(&mut buffer).unwrap();
            let request = Message::from_vec(&buffer[..size]).unwrap();
            server
                .send_to(&answer(&request).to_vec().unwrap(), peer)
                .unwrap();
        });
        let records = dns_query("example.com", RecordType::A, address).unwrap();
        worker.join().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].ttl_seconds, 321);
        assert_eq!(records[0].resolver, address.to_string());
        assert_eq!(records[0].transport, "udp");
        assert!(!records[0].dnssec_validated);
    }

    #[test]
    fn dns_truncated_udp_falls_back_to_tcp() {
        let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        tcp.set_nonblocking(true).unwrap();
        let address = tcp.local_addr().unwrap();
        let udp = UdpSocket::bind(address).unwrap();
        udp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let worker = std::thread::spawn(move || {
            let mut buffer = [0; 4096];
            let (size, peer) = udp.recv_from(&mut buffer).unwrap();
            let request = Message::from_vec(&buffer[..size]).unwrap();
            let mut truncated = answer(&request);
            truncated.metadata.truncation = true;
            udp.send_to(&truncated.to_vec().unwrap(), peer).unwrap();
            let started = Instant::now();
            let mut stream = loop {
                match tcp.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && started.elapsed() < Duration::from_secs(5) =>
                    {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut length = [0; 2];
            stream.read_exact(&mut length).unwrap();
            let mut body = vec![0; u16::from_be_bytes(length) as usize];
            stream.read_exact(&mut body).unwrap();
            let request = Message::from_vec(&body).unwrap();
            let bytes = answer(&request).to_vec().unwrap();
            stream
                .write_all(&(bytes.len() as u16).to_be_bytes())
                .unwrap();
            stream.write_all(&bytes).unwrap();
        });
        let records = dns_query("example.com", RecordType::A, address).unwrap();
        worker.join().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].transport, "tcp");
    }

    #[test]
    fn process_watchdog_terminates_stalled_collector() {
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut cmd = command("powershell.exe");
            cmd.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 10",
            ]);
            cmd
        };
        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut cmd = command("sleep");
            cmd.arg("10");
            cmd
        };
        let started = Instant::now();
        assert!(bounded_output(&mut cmd, 1024, Duration::from_millis(150)).is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    #[ignore = "Requires internet access to the owned itgla.com site; explicit live acceptance only"]
    fn live_owned_public_discovery() {
        let mut progress = Vec::new();
        let options = DiscoveryOptions {
            certificate_transparency: std::env::var_os("ITGLA_TEST_CT").is_some(),
            ct_proxy: std::env::var("ITGLA_TEST_CT_PROXY").ok(),
        };
        let result = discover("https://itgla.com/", options, &mut |percent, message| {
            eprintln!("{percent}% {message}");
            progress.push(percent);
        });
        assert_eq!(result.host, "itgla.com");
        assert!(progress.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(
            result
                .observations
                .iter()
                .any(|item| item.kind == "http-evidence")
        );
        assert!(
            result
                .observations
                .iter()
                .any(|item| item.kind == "tls-certificate")
        );
        assert!(
            result
                .observations
                .iter()
                .any(|item| item.kind == "dns-record")
        );
        assert!(
            result
                .observations
                .iter()
                .all(|item| item.verification_status == "needs-review")
        );
        eprintln!("{}", serde_json::to_string_pretty(&result).unwrap());
    }
}

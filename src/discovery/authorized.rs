//! Explicitly authorised, bounded active checks.
//!
//! This module only checks targets and ports selected by the user. It never
//! guesses credentials, follows redirects, exploits services, or promotes
//! results to inventory records.

use std::{
    net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const MAX_TARGETS: usize = 64;
const MAX_PORTS: usize = 16;
const MAX_CHECKS: usize = 256;
const MAX_CIDR_HOSTS: usize = 64;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedTarget {
    pub value: String,
    pub label: String,
}

impl AuthorizedTarget {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckProfile {
    Tcp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedCheckOptions {
    pub targets: Vec<AuthorizedTarget>,
    pub ports: Vec<u16>,
    pub profile: CheckProfile,
    pub timeout: Duration,
}

impl Default for AuthorizedCheckOptions {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            ports: vec![22, 80, 443],
            profile: CheckProfile::Tcp,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortCheckStatus {
    Open,
    Closed,
    TimedOut,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortCheckObservation {
    pub target: String,
    pub label: String,
    pub port: u16,
    pub profile: CheckProfile,
    pub status: PortCheckStatus,
    pub detail: String,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedCheckReport {
    pub started_at_utc: String,
    pub finished_at_utc: String,
    pub requested_targets: Vec<String>,
    pub requested_ports: Vec<u16>,
    pub observations: Vec<PortCheckObservation>,
    pub stopped: bool,
    pub warnings: Vec<String>,
}

/// Transport seam for deterministic tests and future TLS/HTTP transports.
pub trait CheckTransport: Send + Sync {
    fn tcp_check(&self, target: &str, port: u16, timeout: Duration) -> PortCheckStatus;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TcpTransport;

impl CheckTransport for TcpTransport {
    fn tcp_check(&self, target: &str, port: u16, timeout: Duration) -> PortCheckStatus {
        let address = match target.parse::<IpAddr>() {
            Ok(ip) => SocketAddr::new(ip, port),
            Err(_) => match (target, port)
                .to_socket_addrs()
                .ok()
                .and_then(|mut a| a.next())
            {
                Some(address) => address,
                None => return PortCheckStatus::Failed,
            },
        };
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                drop(stream);
                PortCheckStatus::Open
            }
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => PortCheckStatus::TimedOut,
            Err(_) => PortCheckStatus::Closed,
        }
    }
}

fn valid_hostname(value: &str) -> bool {
    let value = value.trim().trim_end_matches('.');
    if value.is_empty() || value.len() > 253 || value.contains(['/', ':', '@']) {
        return false;
    }
    let labels = value.split('.').collect::<Vec<_>>();
    labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

fn expand_cidr(value: &str) -> Result<Vec<String>, String> {
    let (network, prefix) = value.split_once('/').ok_or("CIDR must include prefix")?;
    let ip: IpAddr = network.parse().map_err(|_| "Invalid CIDR address")?;
    let prefix: u8 = prefix.parse().map_err(|_| "Invalid CIDR prefix")?;
    let bits = if ip.is_ipv4() { 32 } else { 128 };
    if prefix > bits || bits == 128 && prefix < 120 {
        return Err("CIDR range exceeds 64 hosts".into());
    }
    let host_bits = bits - prefix;
    let count = 1usize
        .checked_shl(host_bits as u32)
        .ok_or("CIDR range too large")?;
    if count > MAX_CIDR_HOSTS {
        return Err(format!("CIDR range limited to {MAX_CIDR_HOSTS} hosts"));
    }
    let base = match ip {
        IpAddr::V4(v) => u32::from(v) as u128,
        IpAddr::V6(v) => u128::from(v),
    };
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (bits - prefix)
    };
    (0..count)
        .map(|offset| {
            let value = (base & mask) + offset as u128;
            if bits == 32 {
                IpAddr::V4(std::net::Ipv4Addr::from(value as u32)).to_string()
            } else {
                IpAddr::V6(std::net::Ipv6Addr::from(value)).to_string()
            }
        })
        .collect::<Vec<_>>()
        .pipe(Ok)
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}
impl<T> Pipe for T {}

fn normalize_targets(targets: &[AuthorizedTarget]) -> Result<Vec<AuthorizedTarget>, String> {
    if targets.is_empty() {
        return Err("Select at least one authorized target".into());
    }
    let mut output = Vec::new();
    for target in targets {
        let value = target
            .value
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if value.contains('/') {
            for host in expand_cidr(&value)? {
                output.push(AuthorizedTarget::new(host, &target.label));
            }
        } else if value.parse::<IpAddr>().is_ok() || valid_hostname(&value) {
            output.push(AuthorizedTarget::new(value, &target.label));
        } else {
            return Err(format!("Invalid authorized target: {}", target.value));
        }
    }
    if output.len() > MAX_TARGETS {
        return Err(format!("Target limit is {MAX_TARGETS} hosts"));
    }
    output.sort_by(|a, b| a.value.cmp(&b.value));
    output.dedup_by(|a, b| a.value == b.value);
    Ok(output)
}

fn normalize_ports(ports: &[u16]) -> Result<Vec<u16>, String> {
    if ports.is_empty() {
        return Err("Select at least one port".into());
    }
    if ports.len() > MAX_PORTS {
        return Err(format!("Port limit is {MAX_PORTS}"));
    }
    let mut ports = ports.to_vec();
    ports.sort_unstable();
    ports.dedup();
    if ports.contains(&0) {
        return Err("Port must be between 1 and 65535".into());
    }
    Ok(ports)
}

fn utc_now() -> String {
    // Stable audit representation without adding a time dependency.
    format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    )
}

pub fn run_authorized_checks<T: CheckTransport>(
    options: AuthorizedCheckOptions,
    transport: &T,
    cancel: Arc<AtomicBool>,
    mut progress: impl FnMut(usize, usize, &str),
) -> Result<AuthorizedCheckReport, String> {
    let targets = normalize_targets(&options.targets)?;
    let ports = normalize_ports(&options.ports)?;
    if targets.len().saturating_mul(ports.len()) > MAX_CHECKS {
        return Err(format!("Check limit is {MAX_CHECKS} target-port pairs"));
    }
    if options.timeout.is_zero() || options.timeout > Duration::from_secs(30) {
        return Err("Timeout must be between 1 and 30 seconds".into());
    }
    let started_at_utc = utc_now();
    let total = targets.len() * ports.len();
    let mut observations = Vec::with_capacity(total);
    let mut stopped = false;
    for target in &targets {
        for port in &ports {
            if cancel.load(Ordering::Acquire) {
                stopped = true;
                break;
            }
            let index = observations.len() + 1;
            progress(
                index - 1,
                total,
                &format!("Checking {}:{}", target.value, port),
            );
            let start = Instant::now();
            let status = match options.profile {
                CheckProfile::Tcp => transport.tcp_check(&target.value, *port, options.timeout),
            };
            observations.push(PortCheckObservation {
                target: target.value.clone(),
                label: target.label.clone(),
                port: *port,
                profile: options.profile,
                detail: format!(
                    "Authorized {} check",
                    match options.profile {
                        CheckProfile::Tcp => "TCP",
                    }
                ),
                status,
                elapsed_ms: start.elapsed().as_millis(),
            });
            progress(index, total, &format!("Checked {}:{}", target.value, port));
        }
        if stopped {
            break;
        }
    }
    Ok(AuthorizedCheckReport {
        started_at_utc,
        finished_at_utc: utc_now(),
        requested_targets: targets.into_iter().map(|t| t.value).collect(),
        requested_ports: ports,
        observations,
        stopped,
        warnings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        stop_after: Option<usize>,
        calls: std::sync::Mutex<usize>,
    }
    impl CheckTransport for Fake {
        fn tcp_check(&self, _: &str, _: u16, _: Duration) -> PortCheckStatus {
            let mut calls = self.calls.lock().unwrap();
            *calls += 1;
            if self.stop_after.is_some_and(|limit| *calls > limit) {
                PortCheckStatus::TimedOut
            } else {
                PortCheckStatus::Open
            }
        }
    }
    fn options() -> AuthorizedCheckOptions {
        AuthorizedCheckOptions {
            targets: vec![AuthorizedTarget::new("192.168.1.7", "server")],
            ports: vec![22, 5432],
            ..Default::default()
        }
    }
    #[test]
    fn runs_only_selected_pairs_and_reports_progress() {
        let fake = Fake {
            stop_after: None,
            calls: std::sync::Mutex::new(0),
        };
        let events = std::sync::Mutex::new(Vec::new());
        let report = run_authorized_checks(
            options(),
            &fake,
            Arc::new(AtomicBool::new(false)),
            |done, total, _| events.lock().unwrap().push((done, total)),
        )
        .unwrap();
        assert_eq!(report.observations.len(), 2);
        assert_eq!(report.requested_ports, vec![22, 5432]);
        assert_eq!(report.observations[0].status, PortCheckStatus::Open);
        assert_eq!(events.lock().unwrap().last(), Some(&(2, 2)));
    }
    #[test]
    fn cancellation_stops_before_next_pair() {
        let fake = Fake {
            stop_after: None,
            calls: std::sync::Mutex::new(0),
        };
        let cancel = Arc::new(AtomicBool::new(true));
        let report = run_authorized_checks(options(), &fake, cancel, |_, _, _| {}).unwrap();
        assert!(report.stopped);
        assert!(report.observations.is_empty());
    }
    #[test]
    fn rejects_unbounded_or_ambiguous_scope() {
        let fake = Fake {
            stop_after: None,
            calls: std::sync::Mutex::new(0),
        };
        let mut o = options();
        o.targets = vec![AuthorizedTarget::new("10.0.0.0/24", "lan")];
        assert!(
            run_authorized_checks(o, &fake, Arc::new(AtomicBool::new(false)), |_, _, _| {})
                .is_err()
        );
        let mut o = options();
        o.targets = vec![AuthorizedTarget::new("https://example.com", "bad")];
        assert!(
            run_authorized_checks(o, &fake, Arc::new(AtomicBool::new(false)), |_, _, _| {})
                .is_err()
        );
    }
    #[test]
    fn cidr_is_explicit_and_bounded() {
        assert_eq!(expand_cidr("192.168.1.0/30").unwrap().len(), 4);
        assert!(expand_cidr("192.168.0.0/16").is_err());
    }
}

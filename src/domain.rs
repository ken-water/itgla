use std::net::IpAddr;

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerRecord {
    pub id: i64,
    pub tags: Vec<String>,
    pub ip_address: String,
    pub ports: String,
    pub custom_values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerColumn {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerDraft {
    pub tags: Vec<String>,
    pub ip_address: String,
    pub ports: String,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ServerValidationError {
    #[error("Enter at least one tag")]
    EmptyTags,
    #[error("Tags must be 48 characters or fewer")]
    TagTooLong,
    #[error("A server can have at most 20 tags")]
    TooManyTags,
    #[error("Enter a valid IPv4 or IPv6 address")]
    InvalidIpAddress,
    #[error("Enter at least one port")]
    EmptyPorts,
    #[error("Ports must be numbers from 1 to 65535, separated by commas")]
    InvalidPort,
    #[error("A server can have at most 32 ports")]
    TooManyPorts,
}

pub fn validate_server(draft: &ServerDraft) -> Result<ServerDraft, ServerValidationError> {
    let mut tags = Vec::new();
    for candidate in &draft.tags {
        let tag = candidate.trim();
        if tag.is_empty()
            || tags
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(tag))
        {
            continue;
        }
        if tag.chars().count() > 48 {
            return Err(ServerValidationError::TagTooLong);
        }
        tags.push(tag.to_owned());
    }
    if tags.is_empty() {
        return Err(ServerValidationError::EmptyTags);
    }
    if tags.len() > 20 {
        return Err(ServerValidationError::TooManyTags);
    }
    let ip_address = draft.ip_address.trim();
    if ip_address.parse::<IpAddr>().is_err() {
        return Err(ServerValidationError::InvalidIpAddress);
    }
    let ports = normalize_ports(&draft.ports)?;
    Ok(ServerDraft {
        tags,
        ip_address: ip_address.to_owned(),
        ports,
    })
}

pub fn server_tags_from_input(value: &str) -> Vec<String> {
    value.split(',').map(str::to_owned).collect()
}

fn normalize_ports(value: &str) -> Result<String, ServerValidationError> {
    let mut ports = Vec::new();
    for candidate in value.split(',') {
        let candidate = candidate.trim();
        if candidate.is_empty() {
            continue;
        }
        let port = candidate
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or(ServerValidationError::InvalidPort)?;
        if !ports.contains(&port) {
            ports.push(port);
        }
    }
    if ports.is_empty() {
        return Err(ServerValidationError::EmptyPorts);
    }
    if ports.len() > 32 {
        return Err(ServerValidationError::TooManyPorts);
    }
    Ok(ports
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(", "))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Website,
    Domain,
    Certificate,
    Server,
    Service,
}

impl ResourceKind {
    pub const ALL: [Self; 5] = [
        Self::Website,
        Self::Domain,
        Self::Certificate,
        Self::Server,
        Self::Service,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Website => "Website",
            Self::Domain => "Domain",
            Self::Certificate => "Certificate",
            Self::Server => "Server",
            Self::Service => "Service",
        }
    }

    pub fn mark(self) -> &'static str {
        match self {
            Self::Website => "W",
            Self::Domain => "D",
            Self::Certificate => "C",
            Self::Server => "S",
            Self::Service => "A",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Website => "website",
            Self::Domain => "domain",
            Self::Certificate => "certificate",
            Self::Server => "server",
            Self::Service => "service",
        }
    }

    pub fn from_index(index: i32) -> Self {
        Self::ALL
            .get(index.max(0) as usize)
            .copied()
            .unwrap_or(Self::Website)
    }

    pub fn index(self) -> i32 {
        Self::ALL.iter().position(|kind| *kind == self).unwrap_or(0) as i32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Warning,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipKind {
    DeploysTo,
    UsesDomain,
    ProtectedBy,
    DependsOn,
    Serves,
}

impl RelationshipKind {
    pub const ALL: [Self; 5] = [
        Self::DeploysTo,
        Self::UsesDomain,
        Self::ProtectedBy,
        Self::DependsOn,
        Self::Serves,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::DeploysTo => "deploys_to",
            Self::UsesDomain => "uses_domain",
            Self::ProtectedBy => "protected_by",
            Self::DependsOn => "depends_on",
            Self::Serves => "serves",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DeploysTo => "Deploys to",
            Self::UsesDomain => "Uses domain",
            Self::ProtectedBy => "Protected by",
            Self::DependsOn => "Depends on",
            Self::Serves => "Serves",
        }
    }

    pub fn from_index(index: i32) -> Self {
        Self::ALL
            .get(index.max(0) as usize)
            .copied()
            .map_or(Self::DependsOn, |kind| kind)
    }
}

impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Warning => "Attention",
            Self::Critical => "Critical",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Warning,
            2 => Self::Critical,
            _ => Self::Healthy,
        }
    }

    pub fn index(self) -> i32 {
        match self {
            Self::Healthy => 0,
            Self::Warning => 1,
            Self::Critical => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub asset_count: i64,
    pub attention_count: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub id: i64,
    pub project_id: i64,
    pub kind: ResourceKind,
    pub name: String,
    pub detail: String,
    pub status_detail: String,
    pub environment: String,
    pub health: Health,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetDraft {
    pub project_id: i64,
    pub kind: ResourceKind,
    pub name: String,
    pub detail: String,
    pub status_detail: String,
    pub environment: String,
    pub health: Health,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relationship {
    pub id: i64,
    pub source_asset_id: i64,
    pub source_name: String,
    pub target_asset_id: i64,
    pub target_name: String,
    pub kind: RelationshipKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalAsset {
    pub project_name: String,
    pub asset: Asset,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Name is required")]
    EmptyName,
    #[error("Name must be {0} characters or fewer")]
    NameTooLong(usize),
    #[error("Tags must be 48 characters or fewer")]
    TagTooLong,
    #[error("A record can have at most 20 tags")]
    TooManyTags,
    #[error(transparent)]
    Server(#[from] ServerValidationError),
}

pub fn validate_name(name: &str, maximum: usize) -> Result<String, ValidationError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ValidationError::EmptyName);
    }
    if trimmed.chars().count() > maximum {
        return Err(ValidationError::NameTooLong(maximum));
    }
    Ok(trimmed.to_owned())
}

pub fn parse_tags(value: &str) -> Result<Vec<String>, ValidationError> {
    let mut tags = Vec::new();
    for candidate in value.split([',', '\u{ff0c}']) {
        let tag = candidate.trim();
        if tag.is_empty() || tags.iter().any(|existing| existing == tag) {
            continue;
        }
        if tag.chars().count() > 48 {
            return Err(ValidationError::TagTooLong);
        }
        tags.push(tag.to_owned());
    }
    if tags.len() > 20 {
        return Err(ValidationError::TooManyTags);
    }
    Ok(tags)
}

pub fn filter_assets<'a>(
    assets: &'a [Asset],
    kind: Option<ResourceKind>,
    query: &str,
) -> Vec<&'a Asset> {
    let normalized = query.trim().to_lowercase();
    assets
        .iter()
        .filter(|asset| kind.is_none_or(|wanted| asset.kind == wanted))
        .filter(|asset| {
            normalized.is_empty()
                || asset.name.to_lowercase().contains(&normalized)
                || asset.detail.to_lowercase().contains(&normalized)
                || asset.kind.label().contains(&normalized)
                || asset
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&normalized))
        })
        .collect()
}

pub fn kind_from_label(label: &str) -> Option<ResourceKind> {
    ResourceKind::ALL
        .into_iter()
        .find(|kind| kind.label() == label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_normalizes_server_records() {
        let draft = ServerDraft {
            tags: vec!["  production  ".into(), "api".into(), "PRODUCTION".into()],
            ip_address: " 203.0.113.10 ".into(),
            ports: "443, 22, 443".into(),
        };
        assert_eq!(
            validate_server(&draft),
            Ok(ServerDraft {
                tags: vec!["production".into(), "api".into()],
                ip_address: "203.0.113.10".into(),
                ports: "443, 22".into(),
            })
        );
    }

    #[test]
    fn rejects_invalid_server_addresses_and_ports() {
        let mut draft = ServerDraft {
            tags: vec!["database".into()],
            ip_address: "not-an-ip".into(),
            ports: "5432".into(),
        };
        assert_eq!(
            validate_server(&draft),
            Err(ServerValidationError::InvalidIpAddress)
        );
        draft.ip_address = "2001:db8::10".into();
        draft.ports = "0, 70000".into();
        assert_eq!(
            validate_server(&draft),
            Err(ServerValidationError::InvalidPort)
        );
    }

    fn asset(kind: ResourceKind, name: &str) -> Asset {
        Asset {
            id: 1,
            project_id: 1,
            kind,
            name: name.into(),
            detail: "Docker · production".into(),
            status_detail: "healthy".into(),
            environment: "Production".into(),
            health: Health::Healthy,
            tags: vec!["Docker".into()],
        }
    }

    #[test]
    fn filters_by_project_kind_and_query() {
        let assets = [
            asset(ResourceKind::Service, "sync-worker"),
            asset(ResourceKind::Service, "notes-api"),
            asset(ResourceKind::Domain, "cloudnote.io"),
        ];
        let services = filter_assets(&assets, Some(ResourceKind::Service), "");
        assert_eq!(services.len(), 2);
        assert!(
            services
                .iter()
                .all(|asset| asset.kind == ResourceKind::Service)
        );

        let worker = filter_assets(&assets, None, "WORKER");
        assert_eq!(worker.len(), 1);
        assert_eq!(worker[0].name, "sync-worker");
    }

    #[test]
    fn empty_results_are_explicit() {
        let assets = [asset(ResourceKind::Website, "cloudnote.io")];
        assert!(filter_assets(&assets, None, "not-a-real-resource").is_empty());
    }

    #[test]
    fn tags_are_trimmed_deduplicated_and_searchable() {
        assert_eq!(
            parse_tags(" production, Docker\u{ff0c}production "),
            Ok(vec!["production".into(), "Docker".into()])
        );
        let assets = [asset(ResourceKind::Service, "notes-api")];
        assert_eq!(filter_assets(&assets, None, "docker").len(), 1);
    }
}

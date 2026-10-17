use std::net::IpAddr;

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerRecord {
    pub id: i64,
    pub name: String,
    pub tags: Vec<String>,
    pub ip_address: String,
    pub ports: String,
    pub environment: String,
    pub status: String,
    pub purpose: String,
    pub custom_values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerColumn {
    pub id: i64,
    pub name: String,
    pub value_type: String,
    pub default_value: String,
    pub hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedOption {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub semantic: String,
    pub color: String,
    pub is_default: bool,
    pub hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedTag {
    pub id: i64,
    pub label: String,
    pub color: String,
    pub hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerDraft {
    pub name: String,
    pub tags: Vec<String>,
    pub ip_address: String,
    pub ports: String,
    pub environment: String,
    pub status: String,
    pub purpose: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteRecord {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub environment: String,
    pub status: String,
    pub purpose: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteDraft {
    pub name: String,
    pub url: String,
    pub environment: String,
    pub status: String,
    pub purpose: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkKind {
    Change,
    Task,
    Renewal,
    Incident,
    Migration,
    Launch,
    Maintenance,
}

impl WorkKind {
    pub const ALL: [Self; 7] = [
        Self::Change,
        Self::Task,
        Self::Renewal,
        Self::Incident,
        Self::Migration,
        Self::Launch,
        Self::Maintenance,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Change => "change",
            Self::Task => "task",
            Self::Renewal => "renewal",
            Self::Incident => "incident",
            Self::Migration => "migration",
            Self::Launch => "launch",
            Self::Maintenance => "maintenance",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Change => "Change",
            Self::Task => "Task",
            Self::Renewal => "Renewal",
            Self::Incident => "Incident",
            Self::Migration => "Migration",
            Self::Launch => "Launch",
            Self::Maintenance => "Maintenance",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == value)
    }

    pub fn index(self) -> i32 {
        Self::ALL.iter().position(|kind| *kind == self).unwrap_or(0) as i32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkStatus {
    Inbox,
    Planned,
    Ready,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueState {
    Overdue,
    Today,
    Soon,
    Scheduled,
    Flexible,
    None,
}

impl DueState {
    pub fn parse(value: &str) -> Self {
        match value {
            "overdue" => Self::Overdue,
            "today" => Self::Today,
            "soon" => Self::Soon,
            "scheduled" => Self::Scheduled,
            "flexible" => Self::Flexible,
            _ => Self::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Overdue => "Overdue",
            Self::Today => "Due today",
            Self::Soon => "Due soon",
            Self::Scheduled => "Scheduled",
            Self::Flexible => "Flexible",
            Self::None => "No due date",
        }
    }
}

impl WorkStatus {
    pub const ALL: [Self; 7] = [
        Self::Inbox,
        Self::Planned,
        Self::Ready,
        Self::InProgress,
        Self::Blocked,
        Self::Done,
        Self::Cancelled,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Planned => "planned",
            Self::Ready => "ready",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Planned => "Planned",
            Self::Ready => "Ready",
            Self::InProgress => "In progress",
            Self::Blocked => "Blocked",
            Self::Done => "Done",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|status| status.key() == value)
    }

    pub fn index(self) -> i32 {
        Self::ALL
            .iter()
            .position(|status| *status == self)
            .unwrap_or(0) as i32
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkItem {
    pub id: i64,
    pub title: String,
    pub kind: WorkKind,
    pub status: WorkStatus,
    pub priority: String,
    pub owner: String,
    pub start_at: String,
    pub due_at: String,
    pub notes: String,
    pub checklist_total: i64,
    pub checklist_completed: i64,
    pub project_count: i64,
    pub server_count: i64,
    pub asset_count: i64,
    pub due_state: DueState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkProjectChoice {
    pub id: i64,
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkServerChoice {
    pub id: i64,
    pub label: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkAssetChoice {
    pub id: i64,
    pub label: String,
    pub selected: bool,
    pub origin: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkImpactChoice {
    pub id: i64,
    pub label: String,
    pub category: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkHealth {
    pub warnings: Vec<String>,
    pub action: Option<WorkHealthAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkHealthAction {
    ReviewScope,
    ReviewImpact,
    AddOwner,
    AddDueDate,
}

impl WorkHealthAction {
    pub fn code(self) -> i32 {
        match self {
            Self::ReviewScope => 1,
            Self::ReviewImpact => 2,
            Self::AddOwner => 3,
            Self::AddDueDate => 4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ReviewScope => "Review scope",
            Self::ReviewImpact => "Review impact",
            Self::AddOwner => "Add owner",
            Self::AddDueDate => "Add due date",
        }
    }
}

impl WorkHealth {
    pub fn message(&self) -> String {
        if self.warnings.is_empty() {
            "Scope looks ready for review".into()
        } else {
            self.warnings.join(" · ")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkDraft {
    pub title: String,
    pub kind: WorkKind,
    pub status: WorkStatus,
    pub priority: String,
    pub owner: String,
    pub start_at: String,
    pub due_at: String,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChecklistItem {
    pub id: i64,
    pub title: String,
    pub completed: bool,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ServerValidationError {
    #[error("Server name is required")]
    EmptyName,
    #[error("Server name must be 160 characters or fewer")]
    NameTooLong,
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
    #[error("Environment is required and must be 64 characters or fewer")]
    InvalidEnvironment,
    #[error("Status is required and must be 64 characters or fewer")]
    InvalidStatus,
    #[error("Purpose must be 1,024 characters or fewer")]
    PurposeTooLong,
}

pub fn validate_server(draft: &ServerDraft) -> Result<ServerDraft, ServerValidationError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(ServerValidationError::EmptyName);
    }
    if name.chars().count() > 160 {
        return Err(ServerValidationError::NameTooLong);
    }
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
    let environment = draft.environment.trim();
    if environment.is_empty() || environment.chars().count() > 64 {
        return Err(ServerValidationError::InvalidEnvironment);
    }
    let status = draft.status.trim();
    if status.is_empty() || status.chars().count() > 64 {
        return Err(ServerValidationError::InvalidStatus);
    }
    let purpose = draft.purpose.trim();
    if purpose.chars().count() > 1_024 {
        return Err(ServerValidationError::PurposeTooLong);
    }
    let purpose = if purpose.is_empty() { name } else { purpose };
    Ok(ServerDraft {
        name: name.to_owned(),
        tags,
        ip_address: ip_address.to_owned(),
        ports,
        environment: environment.to_owned(),
        status: status.to_owned(),
        purpose: purpose.to_owned(),
    })
}

pub fn server_tags_from_input(value: &str) -> Vec<String> {
    value.split(',').map(str::to_owned).collect()
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SiteValidationError {
    #[error("Site name is required")]
    EmptyName,
    #[error("Site name must be 160 characters or fewer")]
    NameTooLong,
    #[error("Enter a URL beginning with http:// or https://")]
    InvalidUrl,
    #[error("Environment is required and must be 64 characters or fewer")]
    InvalidEnvironment,
    #[error("Status is required and must be 64 characters or fewer")]
    InvalidStatus,
    #[error("Purpose must be 1,024 characters or fewer")]
    PurposeTooLong,
    #[error("Tags must be 48 characters or fewer")]
    TagTooLong,
    #[error("A site can have at most 20 tags")]
    TooManyTags,
}

pub fn validate_site(draft: &SiteDraft) -> Result<SiteDraft, SiteValidationError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(SiteValidationError::EmptyName);
    }
    if name.chars().count() > 160 {
        return Err(SiteValidationError::NameTooLong);
    }
    let url = draft.url.trim();
    let url_rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    if url_rest.is_none_or(|rest| rest.trim().is_empty() || rest.contains(char::is_whitespace))
        || url.chars().count() > 2_048
    {
        return Err(SiteValidationError::InvalidUrl);
    }
    let environment = draft.environment.trim();
    if environment.is_empty() || environment.chars().count() > 64 {
        return Err(SiteValidationError::InvalidEnvironment);
    }
    let status = draft.status.trim();
    if status.is_empty() || status.chars().count() > 64 {
        return Err(SiteValidationError::InvalidStatus);
    }
    let purpose = draft.purpose.trim();
    if purpose.chars().count() > 1_024 {
        return Err(SiteValidationError::PurposeTooLong);
    }
    let mut tags = Vec::new();
    for candidate in &draft.tags {
        let tag = candidate.trim();
        if tag.is_empty()
            || tags
                .iter()
                .any(|item: &String| item.eq_ignore_ascii_case(tag))
        {
            continue;
        }
        if tag.chars().count() > 48 {
            return Err(SiteValidationError::TagTooLong);
        }
        tags.push(tag.to_owned());
    }
    if tags.len() > 20 {
        return Err(SiteValidationError::TooManyTags);
    }
    Ok(SiteDraft {
        name: name.to_owned(),
        url: url.to_owned(),
        environment: environment.to_owned(),
        status: status.to_owned(),
        purpose: purpose.to_owned(),
        tags,
    })
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
    #[error(transparent)]
    Site(#[from] SiteValidationError),
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
            name: "api-01".into(),
            tags: vec!["  production  ".into(), "api".into(), "PRODUCTION".into()],
            ip_address: " 203.0.113.10 ".into(),
            ports: "443, 22, 443".into(),
            environment: " production ".into(),
            status: " active ".into(),
            purpose: " API gateway ".into(),
        };
        assert_eq!(
            validate_server(&draft),
            Ok(ServerDraft {
                name: "api-01".into(),
                tags: vec!["production".into(), "api".into()],
                ip_address: "203.0.113.10".into(),
                ports: "443, 22".into(),
                environment: "production".into(),
                status: "active".into(),
                purpose: "API gateway".into(),
            })
        );
    }

    #[test]
    fn rejects_invalid_server_addresses_and_ports() {
        let mut draft = ServerDraft {
            name: "database-01".into(),
            tags: vec!["database".into()],
            ip_address: "not-an-ip".into(),
            ports: "5432".into(),
            environment: "production".into(),
            status: "active".into(),
            purpose: String::new(),
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

    #[test]
    fn validates_and_normalizes_sites() {
        let draft = SiteDraft {
            name: "  Customer portal  ".into(),
            url: " https://app.example.com ".into(),
            environment: " production ".into(),
            status: " active ".into(),
            purpose: " Customer application ".into(),
            tags: vec![" customer-facing ".into(), "Customer-Facing".into()],
        };
        assert_eq!(
            validate_site(&draft),
            Ok(SiteDraft {
                name: "Customer portal".into(),
                url: "https://app.example.com".into(),
                environment: "production".into(),
                status: "active".into(),
                purpose: "Customer application".into(),
                tags: vec!["customer-facing".into()],
            })
        );
        let mut invalid = draft;
        invalid.url = "example.com".into();
        assert_eq!(
            validate_site(&invalid),
            Err(SiteValidationError::InvalidUrl)
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
    fn work_kind_and_status_indices_follow_stable_option_order() {
        assert_eq!(WorkKind::Change.index(), 0);
        assert_eq!(WorkKind::Maintenance.index(), 6);
        assert_eq!(WorkStatus::Inbox.index(), 0);
        assert_eq!(WorkStatus::InProgress.index(), 3);
        assert_eq!(WorkStatus::Cancelled.index(), 6);
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

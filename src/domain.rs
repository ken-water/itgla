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
            Self::Website => "网站",
            Self::Domain => "域名",
            Self::Certificate => "证书",
            Self::Server => "服务器",
            Self::Service => "服务",
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Warning,
    Critical,
}

impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "正常",
            Self::Warning => "需关注",
            Self::Critical => "高风险",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub id: i64,
    pub name: String,
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

    fn asset(kind: ResourceKind, name: &str) -> Asset {
        Asset {
            id: 1,
            project_id: 1,
            kind,
            name: name.into(),
            detail: "Docker · production".into(),
            status_detail: "healthy".into(),
            environment: "生产".into(),
            health: Health::Healthy,
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
}

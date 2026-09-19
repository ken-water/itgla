use thiserror::Error;

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

impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "正常",
            Self::Warning => "需关注",
            Self::Critical => "高风险",
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

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("名称不能为空")]
    EmptyName,
    #[error("名称不能超过 {0} 个字符")]
    NameTooLong(usize),
    #[error("标签不能超过 48 个字符")]
    TagTooLong,
    #[error("最多允许 20 个标签")]
    TooManyTags,
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
    for candidate in value.split([',', '，']) {
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
            parse_tags(" production, Docker，production "),
            Ok(vec!["production".into(), "Docker".into()])
        );
        let assets = [asset(ResourceKind::Service, "notes-api")];
        assert_eq!(filter_assets(&assets, None, "docker").len(), 1);
    }
}

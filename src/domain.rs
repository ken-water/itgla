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
pub struct Asset {
    pub project: &'static str,
    pub kind: ResourceKind,
    pub name: &'static str,
    pub detail: &'static str,
    pub status_detail: &'static str,
    pub environment: &'static str,
    pub health: Health,
}

pub const PROJECTS: [&str; 3] = ["Cloudnote", "Shipfast", "Northstar API"];

pub const ASSETS: [Asset; 14] = [
    Asset {
        project: "Cloudnote",
        kind: ResourceKind::Website,
        name: "app.cloudnote.io",
        detail: "生产站点 · Vercel",
        status_detail: "刚刚检查",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Cloudnote",
        kind: ResourceKind::Domain,
        name: "cloudnote.io",
        detail: "Cloudflare · 自动续费",
        status_detail: "2027-08-16 到期",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Cloudnote",
        kind: ResourceKind::Certificate,
        name: "*.cloudnote.io",
        detail: "Let's Encrypt · 自动续期",
        status_detail: "12 天后续期",
        environment: "生产",
        health: Health::Warning,
    },
    Asset {
        project: "Cloudnote",
        kind: ResourceKind::Server,
        name: "cn-prod-01",
        detail: "东京 · 2 vCPU / 4 GB",
        status_detail: "CPU 34% · 运行 128 天",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Cloudnote",
        kind: ResourceKind::Service,
        name: "notes-api",
        detail: "Docker · :8080",
        status_detail: "v2.8.1 · 3 个实例",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Cloudnote",
        kind: ResourceKind::Service,
        name: "sync-worker",
        detail: "Docker · 队列任务",
        status_detail: "积压 1,248 项",
        environment: "生产",
        health: Health::Critical,
    },
    Asset {
        project: "Shipfast",
        kind: ResourceKind::Website,
        name: "shipfa.st",
        detail: "营销站点 · Netlify",
        status_detail: "5 分钟前检查",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Shipfast",
        kind: ResourceKind::Domain,
        name: "shipfa.st",
        detail: "Namecheap · 手动续费",
        status_detail: "31 天后到期",
        environment: "生产",
        health: Health::Warning,
    },
    Asset {
        project: "Shipfast",
        kind: ResourceKind::Server,
        name: "sf-prod-eu",
        detail: "法兰克福 · 4 vCPU / 8 GB",
        status_detail: "CPU 51% · 运行 46 天",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Shipfast",
        kind: ResourceKind::Service,
        name: "checkout-api",
        detail: "systemd · :9000",
        status_detail: "v1.14.0 · 正常",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Northstar API",
        kind: ResourceKind::Domain,
        name: "northstar.dev",
        detail: "Cloudflare · 自动续费",
        status_detail: "2027-11-04 到期",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Northstar API",
        kind: ResourceKind::Certificate,
        name: "api.northstar.dev",
        detail: "Google Trust Services",
        status_detail: "67 天后续期",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Northstar API",
        kind: ResourceKind::Server,
        name: "ns-edge-01",
        detail: "新加坡 · 2 vCPU / 2 GB",
        status_detail: "CPU 18% · 运行 19 天",
        environment: "生产",
        health: Health::Healthy,
    },
    Asset {
        project: "Northstar API",
        kind: ResourceKind::Service,
        name: "gateway",
        detail: "Docker · :443",
        status_detail: "v4.2.0 · 正常",
        environment: "生产",
        health: Health::Healthy,
    },
];

pub fn filter_assets(
    project: &str,
    kind: Option<ResourceKind>,
    query: &str,
) -> Vec<&'static Asset> {
    let normalized = query.trim().to_lowercase();
    ASSETS
        .iter()
        .filter(|asset| asset.project == project)
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

    #[test]
    fn filters_by_project_kind_and_query() {
        let services = filter_assets("Cloudnote", Some(ResourceKind::Service), "");
        assert_eq!(services.len(), 2);
        assert!(
            services
                .iter()
                .all(|asset| asset.kind == ResourceKind::Service)
        );

        let worker = filter_assets("Cloudnote", None, "WORKER");
        assert_eq!(worker.len(), 1);
        assert_eq!(worker[0].name, "sync-worker");
    }

    #[test]
    fn empty_results_are_explicit() {
        assert!(filter_assets("Cloudnote", None, "not-a-real-resource").is_empty());
        assert!(filter_assets("Unknown", None, "").is_empty());
    }
}

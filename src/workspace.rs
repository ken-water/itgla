//! Read-only projections of live inventory. Secrets never enter workspace models.
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{
    AppWindow, WorkspaceItem,
    asset_context::ContextRecord,
    storage::{Repository, StorageError},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InventoryResourceRef {
    pub kind: String,
    pub id: i64,
}

#[derive(Clone, Debug)]
pub struct AssetSummary {
    pub resource: InventoryResourceRef,
    pub name: String,
    pub address: String,
    pub purpose: String,
    pub tags: String,
    pub owner: String,
    pub environment: String,
    pub checked: String,
    pub expires: String,
}

impl AssetSummary {
    pub fn key(&self) -> String {
        format!("{}:{}", self.resource.kind, self.resource.id)
    }
    fn searchable(&self) -> String {
        format!(
            "{} {} {} {} {} {} {}",
            self.name,
            self.address,
            self.purpose,
            self.tags,
            self.owner,
            self.environment,
            self.resource.kind
        )
        .to_lowercase()
    }
}

fn context<'a>(
    all: &'a crate::asset_context::ContextIndex,
    kind: &str,
    id: i64,
) -> Option<&'a ContextRecord> {
    all.get(&(kind.into(), id))
}

pub fn assets(repo: &Repository) -> Result<Vec<AssetSummary>, StorageError> {
    let contexts = repo.asset_contexts()?;
    let servers = repo.display_server_records(false)?;
    let entries = repo.inventory_entries()?;
    let server_names = servers
        .iter()
        .map(|s| (s.id, s.name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut out = Vec::new();
    for s in &servers {
        let c = context(&contexts, "server", s.id);
        out.push(AssetSummary {
            resource: InventoryResourceRef {
                kind: "server".into(),
                id: s.id,
            },
            name: s.name.clone(),
            address: s.ip_address.clone(),
            purpose: s.purpose.clone(),
            tags: s.tags.join(", "),
            owner: c.map(|c| c.data.owner.clone()).unwrap_or_default(),
            environment: c
                .filter(|c| !c.data.environment.is_empty())
                .map(|c| c.data.environment.clone())
                .unwrap_or_else(|| s.environment.clone()),
            checked: c.and_then(|c| c.verified_at.clone()).unwrap_or_default(),
            expires: String::new(),
        });
    }
    for e in entries.iter().filter(|e| !e.hidden) {
        let kind = if e.kind == 0 { "service" } else { "domain" };
        let c = context(&contexts, kind, e.id);
        let host_names = e
            .hosts
            .iter()
            .filter_map(|id| server_names.get(id).copied())
            .collect::<Vec<_>>()
            .join(", ");
        let address = if e.kind == 0 {
            format!(
                "{} {}{}",
                e.details.protocol,
                e.details.ports,
                if host_names.is_empty() {
                    String::new()
                } else {
                    format!(" · {host_names}")
                }
            )
        } else {
            e.details.name.clone()
        };
        out.push(AssetSummary {
            resource: InventoryResourceRef {
                kind: kind.into(),
                id: e.id,
            },
            name: if e.kind == 0 {
                crate::operational::display_service_name(e, &servers)
            } else {
                e.details.name.clone()
            },
            address,
            purpose: e.details.notes.clone(),
            tags: e.details.tags.clone(),
            owner: c.map(|c| c.data.owner.clone()).unwrap_or_default(),
            environment: c.map(|c| c.data.environment.clone()).unwrap_or_default(),
            checked: c.and_then(|c| c.verified_at.clone()).unwrap_or_default(),
            expires: e.details.expires.clone(),
        });
    }
    for b in repo.businesses()? {
        let c = context(&contexts, "business", b.id);
        out.push(AssetSummary {
            resource: InventoryResourceRef {
                kind: "business".into(),
                id: b.id,
            },
            name: b.name,
            address: String::new(),
            purpose: b.notes,
            tags: b.tags,
            owner: c.map(|c| c.data.owner.clone()).unwrap_or_default(),
            environment: c.map(|c| c.data.environment.clone()).unwrap_or_default(),
            checked: c.and_then(|c| c.verified_at.clone()).unwrap_or_default(),
            expires: String::new(),
        });
    }
    Ok(out)
}

pub fn search<'a>(all: &'a [AssetSummary], query: &str, kind: &str) -> Vec<&'a AssetSummary> {
    let q = query.trim().to_lowercase();
    let mut result = all
        .iter()
        .filter(|a| (kind.is_empty() || a.resource.kind == kind) && a.searchable().contains(&q))
        .collect::<Vec<_>>();
    result.sort_by_cached_key(|a| {
        let n = a.name.to_lowercase();
        (
            if n == q {
                0
            } else if n.starts_with(&q) {
                1
            } else {
                2
            },
            n,
            a.resource.kind.clone(),
            a.resource.id,
        )
    });
    result
}

fn item(a: &AssetSummary) -> WorkspaceItem {
    WorkspaceItem {
        key: a.key().into(),
        title: a.name.clone().into(),
        subtitle: format!(
            "{} · {}",
            a.resource.kind,
            if a.address.is_empty() {
                &a.purpose
            } else {
                &a.address
            }
        )
        .into(),
    }
}

pub fn refresh(w: &AppWindow, repo: &Repository) -> Result<(), StorageError> {
    let all = assets(repo)?;
    refresh_projection(w, repo, &all)
}

fn refresh_projection(
    w: &AppWindow,
    repo: &Repository,
    all: &[AssetSummary],
) -> Result<(), StorageError> {
    crate::refresh_discovery(w, repo)?;
    let types = ["", "server", "service", "domain", "business"];
    w.set_workspace_has_assets(!all.is_empty());
    let kind = types.get(w.get_workspace_type() as usize).unwrap_or(&"");
    let mut found = search(all, w.get_workspace_query().as_str(), kind);
    let mode = w.get_workspace_sort();
    if mode >= 0 {
        let value = |a: &AssetSummary| {
            match mode / 2 {
                0 => &a.name,
                1 => &a.address,
                2 => &a.resource.kind,
                3 => &a.purpose,
                4 => &a.environment,
                5 => &a.owner,
                _ => &a.checked,
            }
            .to_lowercase()
        };
        found.sort_by_cached_key(|a| (value(a), a.key()));
        if mode % 2 == 1 {
            found.reverse();
        }
    }
    w.set_workspace_businesses(ModelRc::new(VecModel::from(
        all.iter()
            .filter(|a| a.resource.kind == "business")
            .take(5)
            .map(item)
            .collect::<Vec<_>>(),
    )));
    let mut recent = all
        .iter()
        .filter(|a| !a.checked.is_empty())
        .collect::<Vec<_>>();
    recent.sort_by(|a, b| {
        b.checked
            .cmp(&a.checked)
            .then_with(|| a.key().cmp(&b.key()))
    });
    w.set_workspace_recent(ModelRc::new(VecModel::from(
        recent
            .into_iter()
            .take(5)
            .map(|a| WorkspaceItem {
                subtitle: format!("Checked {} UTC", a.checked).into(),
                ..item(a)
            })
            .collect::<Vec<_>>(),
    )));
    let rows = found
        .iter()
        .take(1000)
        .enumerate()
        .map(|(i, a)| {
            crate::tables::row(
                i as i32,
                vec![
                    a.name.clone(),
                    a.address.clone(),
                    a.resource.kind.clone(),
                    a.purpose.clone(),
                    a.environment.clone(),
                    a.owner.clone(),
                    if a.checked.is_empty() {
                        "Not checked".into()
                    } else {
                        a.checked.chars().take(10).collect()
                    },
                ],
            )
        })
        .collect::<Vec<_>>();
    w.set_workspace_rows(ModelRc::new(VecModel::from(rows)));
    w.set_workspace_keys(ModelRc::new(VecModel::from(
        found
            .iter()
            .take(1000)
            .map(|a| slint::SharedString::from(a.key()))
            .collect::<Vec<_>>(),
    )));
    let technical = all.iter().filter(|a| a.resource.kind != "business").count();
    let businesses = all.iter().filter(|a| a.resource.kind == "business").count();
    let managed = technical + businesses;
    w.set_workspace_count(
        format!(
            "Free plan: {} assets | No current limit | Technical: {} | Businesses: {}",
            managed, technical, businesses
        )
        .into(),
    );
    w.set_workspace_result_status(
        format!(
            "{} matching assets{}",
            found.len(),
            if found.len() > 1000 {
                " · showing first 1000; refine your search"
            } else {
                ""
            }
        )
        .into(),
    );
    let global = search(all, w.get_workspace_search_query().as_str(), "");
    w.set_workspace_search_results(ModelRc::new(VecModel::from(
        global.iter().take(50).map(|a| item(a)).collect::<Vec<_>>(),
    )));
    w.set_workspace_search_status(
        format!(
            "{} results{}",
            global.len(),
            if global.len() > 50 {
                " · showing first 50; refine your search"
            } else {
                ""
            }
        )
        .into(),
    );
    crate::reminders::refresh(w, repo, all)?;
    Ok(())
}

pub fn details(w: &AppWindow, repo: &Repository, key: &str) -> Result<(), StorageError> {
    let all = assets(repo)?;
    let a = all.iter().find(|a| a.key() == key).ok_or_else(|| {
        StorageError::InvalidImport(
            "This asset is hidden or no longer available. Refresh the workspace.".into(),
        )
    })?;
    let c = repo.asset_context(&a.resource.kind, a.resource.id)?;
    let mut text = format!(
        "{}\n{}\n\nAddress / service: {}\nPurpose: {}\nEnvironment: {}\nContact: {}\nLast checked: {}\nSource: {}",
        a.name,
        a.resource.kind,
        recorded(&a.address),
        recorded(&a.purpose),
        recorded(&a.environment),
        recorded(&a.owner),
        recorded(&a.checked),
        recorded(&c.data.source)
    );
    if !a.expires.is_empty() {
        text.push_str(&format!("\nExpiry date: {}", a.expires));
    }
    if (a.resource.kind == "service" || a.resource.kind == "domain")
        && let Some(entry) = repo
            .inventory_entries()?
            .into_iter()
            .find(|e| e.id == a.resource.id)
    {
        let d = entry.details;
        if a.resource.kind == "service" {
            text.push_str(&format!(
                "\nVersion: {}\nConnection: {}\nStatus: {}",
                recorded(&d.version),
                recorded(&d.address),
                if d.pending_confirmation {
                    "Pending confirmation"
                } else {
                    "Recorded; not live monitored"
                }
            ));
        } else {
            text.push_str(&format!(
                "\nParent domain: {}\nRegistrar: {}\nRegistration date: {}\nCDN: {}",
                recorded(&d.parent),
                recorded(&d.registrar),
                recorded(&d.registered),
                recorded(&d.cdn)
            ));
        }
    }
    text.push_str(&format!(
        "\n\nDocumentation: {}\n\nInformation is manually recorded, not a live health check.",
        if c.data.documentation.is_empty() {
            "Not recorded"
        } else {
            &c.data.documentation
        }
    ));
    let index = all.iter().map(|a| (a.key(), a)).collect::<HashMap<_, _>>();
    let mut related = Vec::new();
    for link in repo.resource_links()? {
        let source = format!("{}:{}", link.source_type, link.source_id);
        let target = format!("{}:{}", link.target_type, link.target_id);
        let other = if source == key {
            Some((target, link.relation_type.replace('_', " ")))
        } else if target == key {
            Some((
                source,
                format!("{} this asset", link.relation_type.replace('_', " ")),
            ))
        } else {
            None
        };
        if let Some((other, label)) = other
            && let Some(b) = index.get(&other)
        {
            related.push(WorkspaceItem {
                key: other.into(),
                title: b.name.clone().into(),
                subtitle: format!("{} · {}", label, b.resource.kind).into(),
            });
        }
    }
    related.sort_by(|a, b| a.title.cmp(&b.title));
    related.dedup_by(|a, b| a.key == b.key);
    let mut activity = String::new();
    let history = repo.resource_history(&a.resource.kind, a.resource.id)?;
    activity.push_str("Recent activity (up to 100 entries)\n");
    if history.is_empty() {
        activity.push_str("No recorded activity. Earlier history may not be available.");
    }
    for h in history {
        activity.push_str(&format!(
            "\n{} UTC · {} · {}",
            h.occurred_at, h.action, h.summary
        ));
    }
    if a.resource.kind == "domain" {
        activity.push_str("\n\nManual expiry review history (up to 100 entries)");
        let mut stmt = repo.connection.prepare("SELECT occurred_at,action,source_date,actor FROM reminder_activity WHERE resource_id=?1 ORDER BY id DESC LIMIT 100")?;
        let events = stmt.query_map([a.resource.id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        for event in events {
            let (time, action, date, actor) = event?;
            activity.push_str(&format!(
                "\n{time} UTC | {actor}: {action} recorded expiry {date}"
            ));
        }
    }
    w.set_workspace_detail_history(activity.into());
    w.set_workspace_detail_text(text.into());
    w.set_workspace_detail_key(key.into());
    w.set_workspace_related(ModelRc::new(VecModel::from(related)));
    w.set_workspace_detail_open(true);
    Ok(())
}

/// Stable native page IDs; unknown legacy/invalid settings fall back to Overview.
pub fn startup_page(repo: &Repository) -> i32 {
    repo.connection
        .query_row(
            "SELECT value FROM app_settings WHERE key='workspace_page'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .filter(|v| valid_page(*v))
        .unwrap_or(8)
}
fn valid_page(page: i32) -> bool {
    matches!(page, 0 | 1 | 2 | 4..=13)
}
pub fn remember_page(repo: &Repository, page: i32) -> Result<(), StorageError> {
    if valid_page(page) {
        repo.connection.execute("INSERT INTO app_settings(key,value) VALUES('workspace_page',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=CURRENT_TIMESTAMP WHERE value<>excluded.value",[page.to_string()])?;
    }
    Ok(())
}

fn recorded(s: &str) -> &str {
    if s.is_empty() { "Not recorded" } else { s }
}
fn report(w: &AppWindow, result: Result<(), StorageError>) {
    if let Err(e) = result {
        w.set_workspace_error(e.to_string().into());
    } else {
        w.set_workspace_error("".into());
    }
}

/// Poll only SQLite revision counters; rebuild visible projections after a write.
/// This also catches existing editors/importers without duplicating their transactions.
pub fn setup(window: &AppWindow, repo: Rc<RefCell<Repository>>) {
    if let Ok(saved) = repo.borrow().connection.query_row(
        "SELECT value FROM app_settings WHERE key='workspace_directory_layout'",
        [],
        |r| r.get::<_, String>(0),
    ) && let Ok((widths, fitted)) = serde_json::from_str::<(Vec<f32>, bool)>(&saved)
        && widths.len() == 9
        && widths
            .iter()
            .all(|v| v.is_finite() && *v >= 72.0 && *v <= 3000.0)
    {
        window.set_workspace_column_widths(ModelRc::new(VecModel::from(widths)));
        window.set_workspace_fitted(fitted);
    }
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_workspace_save_layout(move || {
        let Some(w)=weak.upgrade() else { return };
        let widths=w.get_workspace_column_widths().iter().collect::<Vec<_>>();
        let result=(|| {
            let value=serde_json::to_string(&(widths,w.get_workspace_fitted()))?;
            r.borrow().connection.execute("INSERT INTO app_settings(key,value) VALUES('workspace_directory_layout',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value WHERE value<>excluded.value",[value])?;
            Ok(())
        })();
        report(&w,result);
    });
    let weak = window.as_weak();
    let r = repo.clone();
    let previous = std::cell::Cell::new(None);
    window.on_workspace_sync(move || {
        let Some(w) = weak.upgrade() else { return };
        let r = r.borrow();
        let revision = r.connection.query_row(
            "SELECT total_changes(), (SELECT data_version FROM pragma_data_version)",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        );
        let Ok(revision) = revision else { return };
        if previous.get() == Some(revision) {
            return;
        }
        previous.set(Some(revision));
        if w.get_workspace_search_open()
            || w.get_workspace_detail_open()
            || matches!(w.get_active_tab(), 8 | 9 | 13)
        {
            report(&w, refresh(&w, &r));
            if w.get_workspace_detail_open() {
                let result = details(&w, &r, w.get_workspace_detail_key().as_str());
                if result.is_err() {
                    w.set_workspace_detail_open(false);
                }
                report(&w, result);
            }
        }
    });
    let weak = window.as_weak();
    let r = repo.clone();
    let cache = RefCell::new(None::<((i64, i64, bool), Vec<AssetSummary>)>);
    window.on_workspace_refresh(move || {
        if let Some(w) = weak.upgrade() {
            let r = r.borrow();
            let result = (|| {
                let revision = r.connection.query_row(
                    "SELECT total_changes(), (SELECT data_version FROM pragma_data_version)",
                    [],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )?;
                let revision = (revision.0, revision.1, w.get_demo_mode());
                let mut cache = cache.borrow_mut();
                if cache
                    .as_ref()
                    .is_none_or(|(version, _)| *version != revision)
                {
                    *cache = Some((revision, assets(&r)?));
                }
                if let Some((_, all)) = cache.as_ref() {
                    refresh_projection(&w, &r, all)?;
                }
                Ok(())
            })();
            report(&w, result);
        }
    });
    let history = Rc::new(RefCell::new(Vec::<String>::new()));
    let weak = window.as_weak();
    let r = repo.clone();
    let h = history.clone();
    window.on_workspace_open(move |key| {
        if let Some(w) = weak.upgrade() {
            if w.get_workspace_detail_open() {
                let previous = w.get_workspace_detail_key().to_string();
                if previous != key.as_str() {
                    h.borrow_mut().push(previous);
                }
            } else {
                h.borrow_mut().clear();
            }
            report(&w, details(&w, &r.borrow(), key.as_str()));
            w.set_workspace_can_back(!h.borrow().is_empty());
        }
    });
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_workspace_back(move || {
        if let Some(w) = weak.upgrade() {
            if let Some(key) = history.borrow_mut().pop() {
                report(&w, details(&w, &r.borrow(), &key));
            }
            w.set_workspace_can_back(!history.borrow().is_empty());
        }
    });
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_workspace_reminder_action(move |key, action| {
        if let Some(w) = weak.upgrade() {
            let changed = { crate::reminders::act(&mut r.borrow_mut(), key.as_str(), action) };
            let result = changed.and_then(|()| refresh(&w, &r.borrow()));
            report(&w, result);
        }
    });
    let weak = window.as_weak();
    window.on_workspace_map(move |key| {
        if let Some(w) = weak.upgrade() {
            w.set_workspace_detail_open(false);
            w.invoke_tab_selected(7);
            w.set_topology_view_expanded("".into());
            w.set_topology_view_focus(key);
            w.invoke_topology_refresh();
        }
    });
    let weak = window.as_weak();
    window.on_workspace_edit(move |key| {
        if let Some(w) = weak.upgrade() {
            let Some((kind, id)) = key.split_once(':') else {
                return;
            };
            let Ok(id) = id.parse::<i32>() else { return };
            w.set_workspace_detail_open(false);
            w.invoke_topology_open(kind.into(), id);
        }
    });
    let weak = window.as_weak();
    window.on_workspace_review(move |key| {
        if let Some(w) = weak.upgrade() {
            if let Some(id) = key
                .strip_prefix("work:")
                .and_then(|s| s.parse::<i32>().ok())
            {
                w.invoke_tab_selected(12);
                w.invoke_work_selected(id);
            } else if key.starts_with("domain:") {
                let parts = key.split(':').collect::<Vec<_>>();
                if parts.len() == 3 {
                    w.invoke_workspace_open(format!("domain:{}", parts[1]).into());
                    w.set_workspace_reminder_key(key);
                }
            }
        }
    });
    let weak = window.as_weak();
    let r = repo;
    window.on_workspace_new(move |kind| {
        if let Some(w) = weak.upgrade() {
            match kind {
                0 => {
                    crate::clear_editor(&w);
                    w.invoke_tab_selected(0);
                    w.set_server_editor_open(true);
                }
                1 | 2 => {
                    w.invoke_tab_selected(if kind == 1 { 4 } else { 5 });
                    w.invoke_inventory_new(if kind == 1 { 0 } else { 1 });
                    w.set_inventory_editor_requested(true);
                }
                3 => {
                    w.invoke_tab_selected(6);
                    w.invoke_business_select(0);
                    w.set_business_editor_open(true);
                }
                4 => {
                    w.invoke_tab_selected(1);
                }
                5 => {
                    w.invoke_tab_selected(12);
                    let _ = crate::set_work_rows(&w, &r.borrow(), &crate::WorkListState::default());
                }
                6 => w.invoke_tab_selected(10),
                7 => w.invoke_tab_selected(11),
                8 => {
                    w.invoke_tab_selected(0);
                    w.invoke_open_server_import();
                }
                _ => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_is_literal_and_resource_ids_are_typed() {
        let a = AssetSummary {
            resource: InventoryResourceRef {
                kind: "server".into(),
                id: 1,
            },
            name: "100%_db'".into(),
            address: "2001:db8::1".into(),
            purpose: "Production".into(),
            tags: "db".into(),
            owner: "Alex".into(),
            environment: "Production".into(),
            checked: String::new(),
            expires: String::new(),
        };
        let mut b = a.clone();
        b.resource.kind = "service".into();
        b.name = "PostgreSQL".into();
        let all = vec![a, b];
        assert_ne!(all[0].key(), all[1].key());
        assert_eq!(search(&all, "%_", "").len(), 1);
        assert_eq!(search(&all, "Alex", "").len(), 2);
        assert_eq!(search(&all, "2001:db8", "service").len(), 1);
        assert!(search(&all, "secret-sentinel", "").is_empty());
    }
    #[test]
    fn projection_uses_live_inventory_and_excludes_custom_secret_values() -> Result<(), StorageError>
    {
        let mut r = Repository::in_memory()?;
        r.save_server(
            None,
            &crate::domain::ServerDraft {
                name: "Primary".into(),
                ip_address: "203.0.113.10".into(),
                ports: "22".into(),
                tags: vec!["test".into()],
                purpose: "Customer API".into(),
                environment: "Production".into(),
                status: "Active".into(),
            },
        )?;
        let column = r.save_server_column(None, "Private note", "text", "")?;
        let server = r.server_records()?[0].id;
        r.connection.execute("INSERT INTO server_custom_values(server_id,column_id,value) VALUES(?1,?2,'secret-sentinel-unique')",rusqlite::params![server,column])?;
        let all = assets(&r)?;
        assert!(search(&all, "secret-sentinel-unique", "").is_empty());
        assert!(all.iter().any(|a| a.name == "Primary"));
        assert!(all.iter().any(|a| a.resource.kind == "service"));
        assert_eq!(search(&all, "203.0.113.10", "server").len(), 1);
        Ok(())
    }
    #[test]
    fn details_traversal_is_read_only_and_live_edits_refresh() -> Result<(), StorageError> {
        use slint::Model;
        i_slint_backend_testing::init_no_event_loop();
        let mut repository = Repository::in_memory()?;
        crate::topology::reset_demo(&mut repository)?;
        let repo = Rc::new(RefCell::new(repository));
        let w = AppWindow::new().expect("test window");
        crate::topology::setup(&w, repo.clone());
        crate::operational::setup(&w, repo.clone());
        setup(&w, repo.clone());
        w.set_onboarding_open(false);
        w.invoke_workspace_refresh();
        let service = assets(&repo.borrow())?
            .into_iter()
            .find(|a| a.name == "PostgreSQL primary")
            .expect("demo service");
        let before: i64 =
            repo.borrow()
                .connection
                .query_row("SELECT total_changes()", [], |r| r.get(0))?;
        w.invoke_workspace_open(service.key().into());
        assert!(w.get_workspace_detail_text().contains("5432"));
        assert!(w.get_workspace_detail_text().contains("Alex"));
        let host = w
            .get_workspace_related()
            .iter()
            .find(|i| i.key.starts_with("server:"))
            .expect("hosting server");
        w.invoke_workspace_open(host.key.clone());
        assert!(w.get_workspace_can_back());
        w.invoke_workspace_back();
        assert_eq!(w.get_workspace_detail_key(), service.key());
        let after: i64 = repo
            .borrow()
            .connection
            .query_row("SELECT total_changes()", [], |r| r.get(0))?;
        assert_eq!(before, after, "viewing must never write");
        w.invoke_workspace_sync();
        repo.borrow_mut()
            .set_inventory_hidden(service.resource.id, true)?;
        w.invoke_workspace_sync();
        assert!(!w.get_workspace_detail_open());
        assert!(w.get_workspace_error().contains("no longer available"));
        assert!(
            !w.get_workspace_keys()
                .iter()
                .any(|key| key == service.key())
        );
        w.invoke_workspace_new(3);
        assert!(w.get_business_editor_open());
        Ok(())
    }

    #[test]
    fn search_large_inventory_measurement() {
        let all = (0..10_000)
            .map(|id| AssetSummary {
                resource: InventoryResourceRef {
                    kind: "server".into(),
                    id,
                },
                name: format!("Server-{id:05}"),
                address: format!("10.0.{}.{}", id / 256, id % 256),
                purpose: "API production".into(),
                tags: "customer".into(),
                owner: "Alex".into(),
                environment: "Production".into(),
                checked: String::new(),
                expires: String::new(),
            })
            .collect::<Vec<_>>();
        let mut durations = Vec::new();
        for _ in 0..20 {
            let start = std::time::Instant::now();
            assert_eq!(search(&all, "Alex", "").len(), 10_000);
            durations.push(start.elapsed());
        }
        durations.sort();
        eprintln!(
            "10,000 public summaries, debug search p95: {:?}; excludes SQLite/model/render and debounce",
            durations[18]
        );
    }
    #[test]
    fn page_preference_keeps_stable_ids_and_rejects_unknown_values() -> Result<(), StorageError> {
        let r = Repository::in_memory()?;
        assert_eq!(startup_page(&r), 8);
        remember_page(&r, 1)?;
        assert_eq!(startup_page(&r), 1);
        remember_page(&r, 99)?;
        assert_eq!(startup_page(&r), 1);
        r.connection.execute(
            "UPDATE app_settings SET value='3' WHERE key='workspace_page'",
            [],
        )?;
        assert_eq!(startup_page(&r), 8);
        Ok(())
    }

    #[test]
    fn search_modal_keyboard_selection_and_editor_guard() -> Result<(), StorageError> {
        use slint::{Model, platform::Key, platform::WindowEvent};
        i_slint_backend_testing::init_no_event_loop();
        let mut repository = Repository::in_memory()?;
        crate::topology::reset_demo(&mut repository)?;
        let repo = Rc::new(RefCell::new(repository));
        let w = AppWindow::new().expect("window");
        setup(&w, repo);
        w.set_onboarding_open(false);
        w.show().expect("show window");
        w.invoke_open_workspace_search();
        assert!(w.get_workspace_search_open());
        assert!(w.get_workspace_search_results().row_count() > 1);
        let key = w.get_workspace_search_results().row_data(1).unwrap().key;
        for key in [Key::DownArrow, Key::Return] {
            w.window()
                .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
            w.window()
                .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
        }
        assert!(!w.get_workspace_search_open());
        assert_eq!(w.get_workspace_detail_key(), key);
        w.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::Escape.into(),
        });
        w.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::Escape.into(),
        });
        assert!(!w.get_workspace_detail_open());
        w.set_server_editor_open(true);
        w.invoke_open_workspace_search();
        assert!(!w.get_workspace_search_open());
        w.set_server_editor_open(false);
        w.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::Control.into(),
        });
        w.window()
            .dispatch_event(WindowEvent::KeyPressed { text: "k".into() });
        w.window()
            .dispatch_event(WindowEvent::KeyReleased { text: "k".into() });
        w.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::Control.into(),
        });
        assert!(w.get_workspace_search_open());
        Ok(())
    }
    #[test]
    fn directory_layout_and_workspace_actions_survive_navigation() -> Result<(), StorageError> {
        use i_slint_backend_testing::ElementHandle;
        i_slint_backend_testing::init_no_event_loop();
        let mut repository = Repository::in_memory()?;
        crate::topology::reset_demo(&mut repository)?;
        let repo = Rc::new(RefCell::new(repository));
        let w = AppWindow::new().expect("window");
        setup(&w, repo.clone());
        w.set_workspace_column_widths(ModelRc::new(VecModel::from(vec![200.0; 9])));
        w.set_workspace_fitted(false);
        w.invoke_workspace_save_layout();
        let next = AppWindow::new().expect("second window");
        setup(&next, repo.clone());
        assert!(!next.get_workspace_fitted());
        assert_eq!(next.get_workspace_column_widths().row_data(0), Some(200.0));
        let weak = w.as_weak();
        w.on_tab_selected(move |tab| weak.upgrade().unwrap().set_active_tab(tab));
        w.set_onboarding_open(false);
        w.show().expect("show window");
        for (width, height) in [(1100, 720), (1440, 900)] {
            w.window().set_size(slint::PhysicalSize::new(width, height));
            for font in 1..=3 {
                w.set_font_size_step(font);
                for page in [8, 9, 13] {
                    w.set_active_tab(page);
                    w.invoke_workspace_refresh();
                    slint::platform::update_timers_and_animations();
                    let label = if page == 13 {
                        "Work inbox"
                    } else {
                        "Add asset"
                    };
                    let action = ElementHandle::find_by_accessible_label(&w, label)
                        .next()
                        .expect("primary action");
                    let origin = action.absolute_position();
                    assert!(origin.x >= 0.0 && origin.y >= 0.0);
                    assert!(
                        origin.x + action.size().width <= width as f32 + 1.0,
                        "{label} outside viewport"
                    );
                    assert!(origin.y + action.size().height <= height as f32 + 1.0);
                }
            }
        }
        let reminder_key = w
            .get_workspace_reminders()
            .row_data(0)
            .expect("demo reminder")
            .key;
        crate::reminders::act(&mut repo.borrow_mut(), reminder_key.as_str(), 0)?;
        w.set_workspace_reminder_filter(1);
        w.invoke_workspace_refresh();
        assert!(
            w.get_workspace_reminders()
                .iter()
                .any(|item| item.key == reminder_key)
        );
        w.set_active_tab(8);
        w.invoke_workspace_refresh();
        assert!(
            w.get_workspace_reminders()
                .iter()
                .all(|item| item.key != reminder_key)
        );
        assert_eq!(
            w.get_workspace_reminder_filter(),
            1,
            "overview must not reset the reminder archive filter"
        );
        w.set_active_tab(13);
        w.invoke_workspace_refresh();
        w.set_workspace_search_open(true);
        ElementHandle::find_by_accessible_label(&w, "Sites")
            .next()
            .unwrap()
            .invoke_accessible_default_action();
        assert_eq!(
            w.get_active_tab(),
            13,
            "modal must block background navigation"
        );
        w.set_workspace_search_open(false);
        w.invoke_workspace_new(8);
        assert!(w.get_import_dialog_open());
        assert!(!w.get_importing_sites());
        assert_eq!(w.get_active_tab(), 0);
        Ok(())
    }
}

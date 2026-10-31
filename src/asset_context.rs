//! Optional maintenance context shared by the four live inventory resource types.
use std::collections::HashMap;

use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

use crate::{
    AssetContextForm,
    storage::{Repository, StorageError},
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AssetContext {
    pub owner: String,
    pub backup_contact: String,
    pub environment: String,
    pub documentation: String,
    pub documentation_state: i32,
    pub source: String,
}

#[derive(Clone, Debug, Default)]
pub struct ContextRecord {
    pub data: AssetContext,
    pub verified_at: Option<String>,
    pub verified_by: String,
    pub review_due: bool,
}

fn invalid(message: &str) -> StorageError {
    StorageError::InvalidImport(message.into())
}

/// Documentation opens only an explicit HTTP(S) address, never a shell or URI handler.
pub fn validate_documentation_link(link: &str) -> Result<(), StorageError> {
    let rest = link
        .strip_prefix("https://")
        .or_else(|| link.strip_prefix("http://"))
        .ok_or_else(|| invalid("Documentation must begin with https:// or http://"))?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if host.is_empty()
        || host.contains('@')
        || link.len() > 2048
        || link
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return Err(invalid(
            "Use a documentation URL without embedded credentials or spaces",
        ));
    }
    Ok(())
}

impl AssetContext {
    pub fn from_form(form: &AssetContextForm) -> Result<Self, StorageError> {
        let mut data = Self {
            owner: form.owner.trim().into(),
            backup_contact: form.backup_contact.trim().into(),
            environment: form.environment.trim().into(),
            documentation: form.documentation.trim().into(),
            documentation_state: form.documentation_state,
            source: form.source.trim().into(),
        };
        if [
            &data.owner,
            &data.backup_contact,
            &data.environment,
            &data.source,
        ]
        .iter()
        .any(|s| s.chars().count() > 512 || s.chars().any(char::is_control))
        {
            return Err(invalid(
                "Contact, environment and source must be single-line text of at most 512 characters",
            ));
        }
        if !(0..=3).contains(&data.documentation_state) {
            return Err(invalid("Select a documentation availability"));
        }
        if data.documentation_state == 3 {
            validate_documentation_link(&data.documentation)?;
        } else {
            data.documentation.clear();
        }
        if form.reviewer.chars().count() > 160 || form.reviewer.chars().any(char::is_control) {
            return Err(invalid(
                "Checked by must be single-line text of at most 160 characters",
            ));
        }
        Ok(data)
    }

    pub fn summary(&self) -> String {
        let owner = if self.owner.is_empty() {
            "Not recorded"
        } else {
            &self.owner
        };
        let mut text = format!("Contact: {owner}");
        if !self.backup_contact.is_empty() {
            text.push_str(&format!(" | Backup: {}", self.backup_contact));
        }
        if !self.environment.is_empty() {
            text.push_str(&format!(" | {}", self.environment));
        }
        text
    }

    pub fn searchable(&self) -> String {
        format!("{} {} {}", self.summary(), self.source, self.documentation).to_lowercase()
    }
}

impl ContextRecord {
    pub fn form(&self) -> AssetContextForm {
        AssetContextForm {
            owner: self.data.owner.as_str().into(),
            backup_contact: self.data.backup_contact.as_str().into(),
            environment: self.data.environment.as_str().into(),
            documentation: self.data.documentation.as_str().into(),
            documentation_state: self.data.documentation_state,
            source: self.data.source.as_str().into(),
            verification: self.verification().into(),
            reviewer: "".into(),
            checked_now: false,
        }
    }

    pub fn verification(&self) -> String {
        match &self.verified_at {
            None => "Not checked".into(),
            Some(date) => format!(
                "{} {}{}{}",
                if self.review_due {
                    "Review due; last checked"
                } else {
                    "Checked"
                },
                date,
                if self.verified_by.is_empty() {
                    ""
                } else {
                    " by "
                },
                self.verified_by
            ),
        }
    }
}

pub type ContextIndex = HashMap<(String, i64), ContextRecord>;

impl Repository {
    pub fn asset_contexts(&self) -> Result<ContextIndex, StorageError> {
        let mut statement = self.connection.prepare("SELECT resource_type,resource_id,data,verified_at,verified_by,verified_at IS NULL OR datetime(verified_at)<datetime('now','-90 days') FROM asset_context")?;
        let raw = statement.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, bool>(5)?,
            ))
        })?;
        let mut out = HashMap::new();
        for row in raw {
            let (kind, id, json, verified_at, verified_by, review_due) = row?;
            out.insert(
                (kind, id),
                ContextRecord {
                    data: serde_json::from_str(&json)?,
                    verified_at,
                    verified_by,
                    review_due,
                },
            );
        }
        Ok(out)
    }

    pub fn asset_context(&self, kind: &str, id: i64) -> Result<ContextRecord, StorageError> {
        let row = self.connection.query_row("SELECT data,verified_at,verified_by,verified_at IS NULL OR datetime(verified_at)<datetime('now','-90 days') FROM asset_context WHERE resource_type=?1 AND resource_id=?2", params![kind,id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,String>(2)?,r.get::<_,bool>(3)?))).optional()?;
        match row {
            Some((data, verified_at, verified_by, review_due)) => Ok(ContextRecord {
                data: serde_json::from_str(&data)?,
                verified_at,
                verified_by,
                review_due,
            }),
            None => Ok(ContextRecord::default()),
        }
    }
}

/// Called only inside the owning resource's write transaction after its identity is validated.
pub fn save_context(
    tx: &Transaction<'_>,
    kind: &str,
    id: i64,
    form: Option<&AssetContextForm>,
) -> Result<(), StorageError> {
    let Some(form) = form else { return Ok(()) };
    let data = serde_json::to_string(&AssetContext::from_form(form)?)?;
    let old: Option<String> = tx
        .query_row(
            "SELECT data FROM asset_context WHERE resource_type=?1 AND resource_id=?2",
            params![kind, id],
            |r| r.get(0),
        )
        .optional()?;
    let previous: AssetContext = old
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?
        .unwrap_or_default();
    let changed = previous != AssetContext::from_form(form)?;
    tx.execute("INSERT INTO asset_context(resource_type,resource_id,data,verified_at,verified_by) VALUES(?1,?2,?3,CASE WHEN ?4 THEN datetime('now') END,CASE WHEN ?4 THEN ?5 ELSE '' END) ON CONFLICT(resource_type,resource_id) DO UPDATE SET data=excluded.data,verified_at=CASE WHEN ?4 THEN datetime('now') WHEN ?6 THEN NULL ELSE asset_context.verified_at END,verified_by=CASE WHEN ?4 THEN ?5 WHEN ?6 THEN '' ELSE asset_context.verified_by END",params![kind,id,data,form.checked_now,form.reviewer.trim(),changed])?;
    if matches!(kind, "service" | "domain") && (changed || form.checked_now) {
        tx.execute("UPDATE operational_inventory SET verified_at=CASE WHEN ?2 THEN date('now') END WHERE id=?1",params![id,form.checked_now])?;
    }
    if changed || form.checked_now {
        tx.execute("INSERT INTO resource_history(resource_type,resource_id,action,summary) VALUES(?1,?2,'updated',?3)",params![kind,id,if form.checked_now {"Maintenance context checked"} else {"Maintenance context updated; check required"}])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::ServerDraft, operational::Details};

    fn checked() -> AssetContextForm {
        AssetContextForm {
            owner: "Operations / Alice".into(),
            backup_contact: "Bob".into(),
            environment: "Production".into(),
            documentation_state: 3,
            documentation: "https://example.com/runbook".into(),
            source: "Ticket 123".into(),
            checked_now: true,
            reviewer: "Alice".into(),
            ..Default::default()
        }
    }

    #[test]
    fn documentation_states_and_links_are_explicit() -> Result<(), StorageError> {
        let mut form = checked();
        for url in [
            "javascript:alert(1)",
            "file:///secret",
            "https://user:password@example.com",
            "https://",
            "https://example.com/a b",
        ] {
            form.documentation = url.into();
            assert!(AssetContext::from_form(&form).is_err());
        }
        form.documentation_state = 2;
        assert!(AssetContext::from_form(&form)?.documentation.is_empty());
        form.owner = "Unknown".into();
        assert_eq!(AssetContext::from_form(&form)?.owner, "Unknown");
        Ok(())
    }

    #[test]
    fn business_context_survives_noop_hide_and_expires_after_edit() -> Result<(), StorageError> {
        let mut repo = Repository::in_memory()?;
        let form = checked();
        let id = repo.save_business_with_context(0, "Commerce", "critical", "", Some(&form))?;
        let initial = repo.asset_context("business", id)?;
        assert!(initial.verified_at.is_some());
        assert_eq!(initial.verified_by, "Alice");
        let form = initial.form();
        repo.save_business_with_context(id, "Commerce", "critical", "", Some(&form))?;
        assert_eq!(
            repo.asset_context("business", id)?.verified_at,
            initial.verified_at
        );
        repo.connection.execute(
            "UPDATE businesses SET archived_at=CURRENT_TIMESTAMP WHERE id=?1",
            [id],
        )?;
        assert_eq!(
            repo.asset_context("business", id)?.data.owner,
            "Operations / Alice"
        );
        assert!(
            repo.save_business_with_context(id, "Hidden", "", "", Some(&form))
                .is_err()
        );
        repo.connection
            .execute("UPDATE businesses SET archived_at=NULL WHERE id=?1", [id])?;
        repo.save_business_with_context(id, "Commerce renamed", "critical", "", Some(&form))?;
        assert!(repo.asset_context("business", id)?.verified_at.is_none());
        repo.connection
            .execute("DELETE FROM businesses WHERE id=?1", [id])?;
        assert_eq!(
            repo.asset_context("business", id)?.data,
            AssetContext::default()
        );
        Ok(())
    }

    #[test]
    fn invalid_context_rolls_back_server_and_automatic_ssh() -> Result<(), StorageError> {
        let mut repo = Repository::in_memory()?;
        let servers = repo.server_records()?.len();
        let services = repo.inventory_entries()?.len();
        let draft = ServerDraft {
            name: "phase-one".into(),
            ip_address: "192.0.2.1".into(),
            environment: "Production".into(),
            status: "Active".into(),
            ports: "22".into(),
            purpose: "".into(),
            tags: vec!["production".into()],
        };
        let mut form = checked();
        form.documentation = "file:///secret".into();
        assert!(
            repo.save_server_with_context(None, &draft, Some(&form))
                .is_err()
        );
        assert_eq!(repo.server_records()?.len(), servers);
        assert_eq!(repo.inventory_entries()?.len(), services);
        form.documentation = "https://example.com".into();
        let id = repo.save_server_with_context(None, &draft, Some(&form))?;
        assert!(repo.asset_context("server", id)?.verified_at.is_some());
        let ssh = repo
            .inventory_entries()?
            .into_iter()
            .find(|e| e.hosts.contains(&id))
            .ok_or_else(|| invalid("Missing SSH"))?;
        assert_eq!(ssh.details.ports, "22");
        assert!(ssh.details.pending_confirmation);
        let loaded = repo.asset_context("server", id)?.form();
        repo.save_server_with_context(Some(id), &draft, Some(&loaded))?;
        assert!(repo.asset_context("server", id)?.verified_at.is_some());
        repo.update_server_cell(id, 5, "Changed purpose")?;
        assert!(repo.asset_context("server", id)?.verified_at.is_none());
        Ok(())
    }

    #[test]
    fn service_and_domain_context_checks_are_consistent() -> Result<(), StorageError> {
        let mut repo = Repository::in_memory()?;
        let server = repo.save_server(
            None,
            &ServerDraft {
                name: "Origin".into(),
                ip_address: "192.0.2.2".into(),
                environment: "Production".into(),
                status: "Active".into(),
                tags: vec!["origin".into()],
                ports: "22".into(),
                purpose: "".into(),
            },
        )?;
        for kind in [0, 1] {
            let details = Details {
                name: if kind == 0 { "API" } else { "example.com" }.into(),
                ports: if kind == 0 { "443" } else { "" }.into(),
                protocol: "https".into(),
                ..Default::default()
            };
            let resource = if kind == 0 { "service" } else { "domain" };
            let id = repo.save_inventory_with_context(
                0,
                kind,
                details.clone(),
                &[server],
                None,
                Some(&checked()),
            )?;
            assert!(repo.asset_context(resource, id)?.verified_at.is_some());
            let form = repo.asset_context(resource, id)?.form();
            repo.save_inventory_with_context(
                id,
                kind,
                details.clone(),
                &[server],
                None,
                Some(&form),
            )?;
            assert!(repo.asset_context(resource, id)?.verified_at.is_some());
            let mut changed = form.clone();
            changed.owner = "New maintainer".into();
            repo.save_inventory_with_context(
                id,
                kind,
                details.clone(),
                &[server],
                None,
                Some(&changed),
            )?;
            assert!(repo.asset_context(resource, id)?.verified_at.is_none());
            assert!(
                repo.inventory_entries()?
                    .iter()
                    .find(|e| e.id == id)
                    .is_some_and(|e| e.verified_at.is_none())
            );
            repo.mark_inventory_verified(id)?;
            assert!(repo.asset_context(resource, id)?.verified_at.is_some());
            repo.connection.execute("UPDATE operational_inventory SET data=json_set(data,'$.notes','Edited outside the form') WHERE id=?1", [id])?;
            assert!(repo.asset_context(resource, id)?.verified_at.is_none());
            assert!(
                repo.inventory_entries()?
                    .iter()
                    .find(|e| e.id == id)
                    .is_some_and(|e| e.verified_at.is_none())
            );
            repo.mark_inventory_verified(id)?;
            repo.save_inventory_with_context(id, kind, details, &[], None, None)?;
            assert!(repo.asset_context(resource, id)?.verified_at.is_none());
        }
        Ok(())
    }

    #[test]
    fn migration_preserves_existing_checks_and_review_deadline() -> Result<(), StorageError> {
        let mut repo = Repository::in_memory()?;
        let id = repo.save_inventory(
            0,
            0,
            Details {
                name: "Legacy API".into(),
                ..Default::default()
            },
            &[],
            None,
        )?;
        repo.mark_inventory_verified(id)?;
        repo.connection
            .execute_batch("DROP TABLE asset_context; PRAGMA user_version=20;")?;
        // Replay the additive migration against existing inventory.
        repo.connection
            .execute_batch(include_str!("../migrations/021_asset_context.sql"))?;
        assert!(repo.asset_context("service", id)?.verified_at.is_some());
        repo.connection.execute(
            "UPDATE asset_context SET verified_at=datetime('now','-91 days') WHERE resource_id=?1",
            [id],
        )?;
        assert!(repo.asset_context("service", id)?.review_due);
        Ok(())
    }
    #[test]
    fn file_migration_backup_restore_and_context_filtered_export() -> Result<(), StorageError> {
        let directory = std::env::temp_dir().join(format!(
            "itgla-context-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| invalid("Clock"))?
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).map_err(|_| invalid("Create test directory"))?;
        let path = directory.join("itgla.db");
        let mut repo = Repository::open(path.clone())?;
        let id = repo.save_business_with_context(0, "Commerce", "", "", Some(&checked()))?;
        let backup = directory.join("context-backup.db");
        repo.create_backup(&backup)?;
        repo.save_business(id, "Changed", "", "")?;
        repo.restore_backup(&backup)?;
        assert!(repo.asset_context("business", id)?.verified_at.is_some());
        let service = repo.save_inventory_with_context(
            0,
            0,
            Details {
                name: "API".into(),
                ..Default::default()
            },
            &[],
            None,
            Some(&checked()),
        )?;
        let csv_path = directory.join("services.csv");
        assert_eq!(
            repo.export_inventory_csv(&csv_path, 0, "alice", 0, false)?,
            1
        );
        let csv = std::fs::read_to_string(csv_path).map_err(|_| invalid("Read test CSV"))?;
        assert!(csv.contains("Technical contact"));
        assert!(csv.contains("Operations / Alice"));
        drop(repo);
        let repo = Repository::open(path.clone())?;
        assert_eq!(
            repo.asset_context("service", service)?.data.source,
            "Ticket 123"
        );
        // Keep schema-21 objects and reset only the version to exercise idempotent upgrades.
        repo.connection.execute_batch("PRAGMA user_version=20")?;
        drop(repo);
        let repo = Repository::open(path)?;
        assert!(directory.join("backups/pre-schema-21.db").exists());
        assert_eq!(
            repo.asset_context("business", id)?.data.owner,
            "Operations / Alice"
        );
        drop(repo);
        std::fs::remove_dir_all(directory).map_err(|_| invalid("Clean test directory"))?;
        Ok(())
    }

    #[test]
    fn inventory_ui_loads_context_and_new_draft_does_not_leak_contacts() -> Result<(), StorageError>
    {
        use slint::Model;
        use std::{cell::RefCell, rc::Rc};
        i_slint_backend_testing::init_no_event_loop();
        let mut repo = Repository::in_memory()?;
        let id = repo.save_inventory_with_context(
            0,
            0,
            Details {
                name: "API".into(),
                ..Default::default()
            },
            &[],
            None,
            Some(&checked()),
        )?;
        let repo = Rc::new(RefCell::new(repo));
        let window = crate::AppWindow::new().map_err(|_| invalid("Test window"))?;
        crate::operational::setup(&window, repo.clone());
        window.invoke_inventory_select(id as i32);
        assert_eq!(window.get_inventory_context().owner, "Operations / Alice");
        window.set_inventory_query("alice".into());
        window.invoke_inventory_refresh();
        assert_eq!(window.get_inventory_rows().row_count(), 1);
        window.invoke_inventory_new(1);
        assert!(window.get_inventory_context().owner.is_empty());
        let mut form = window.get_inventory_form();
        form.name = "example.com".into();
        window.set_inventory_form(form);
        let mut context = checked();
        context.owner = "Domain team".into();
        window.set_inventory_context(context);
        window.invoke_inventory_save();
        let domain_id = window.get_inventory_form().id;
        assert!(domain_id > 0);
        assert_eq!(
            repo.borrow()
                .asset_context("domain", i64::from(domain_id))?
                .data
                .owner,
            "Domain team"
        );
        window.invoke_inventory_select(id as i32);
        assert_eq!(window.get_inventory_context().owner, "Operations / Alice");
        Ok(())
    }
    #[test]
    fn narrow_largest_font_editor_gives_the_form_room_and_keeps_save_reachable() {
        use i_slint_backend_testing::ElementHandle;
        use slint::{ComponentHandle, PhysicalSize};
        i_slint_backend_testing::init_no_event_loop();
        let window = crate::AppWindow::new().expect("test window");
        window.window().set_size(PhysicalSize::new(1000, 640));
        window.set_onboarding_open(false);
        window.set_font_size_step(4);
        window.set_active_tab(4);
        window.show().expect("show test window");
        ElementHandle::find_by_accessible_label(&window, "New")
            .next()
            .expect("new record")
            .invoke_accessible_default_action();
        slint::platform::update_timers_and_animations();
        let field = ElementHandle::find_by_accessible_label(&window, "Service name or domain")
            .next()
            .expect("name field");
        assert!(field.size().width > 600.0);
        let save = ElementHandle::find_by_accessible_label(&window, "Save record")
            .next()
            .expect("save");
        assert!(save.absolute_position().y + save.size().height <= 640.0);
        assert!(
            ElementHandle::find_by_accessible_label(&window, "Export CSV")
                .next()
                .is_none()
        );
        ElementHandle::find_by_accessible_label(&window, "Close")
            .next()
            .expect("close")
            .invoke_accessible_default_action();
        assert!(
            ElementHandle::find_by_accessible_label(&window, "Export CSV")
                .next()
                .is_some()
        );
    }
}

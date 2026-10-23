//! Operational metadata uses the visible server inventory, not legacy planning assets.
use std::{cell::RefCell, rc::Rc, time::Duration};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::storage::{Repository, StorageError};
use crate::{AppWindow, InventoryChoice, InventoryForm, InventoryRow, domain::parse_tags, secrets};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Details {
    pub name: String,
    pub address: String,
    pub version: String,
    pub ports: String,
    pub parent: String,
    pub registrar: String,
    pub registered: String,
    pub expires: String,
    pub cdn: String,
    pub tags: String,
    pub notes: String,
}

pub struct Entry {
    pub id: i64,
    pub kind: i32,
    pub details: Details,
    pub hosts: Vec<i64>,
    pub has_credentials: bool,
    pub hidden: bool,
}

fn invalid(message: &str) -> StorageError {
    StorageError::InvalidImport(message.into())
}

fn valid_date(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let bytes = value.as_bytes();
    if !value.is_ascii() || bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        value[..4].parse::<u32>(),
        value[5..7].parse::<u32>(),
        value[8..].parse::<u32>(),
    ) else {
        return false;
    };
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    year >= 1900 && day > 0 && day <= days
}

fn valid_host(name: &str) -> bool {
    name.len() <= 253
        && name.contains('.')
        && name.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && !part.starts_with('-')
                && !part.ends_with('-')
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

impl Details {
    fn validate(&mut self, kind: i32) -> Result<(), StorageError> {
        for value in [
            &mut self.name,
            &mut self.address,
            &mut self.version,
            &mut self.ports,
            &mut self.parent,
            &mut self.registrar,
            &mut self.registered,
            &mut self.expires,
            &mut self.cdn,
            &mut self.tags,
            &mut self.notes,
        ] {
            *value = value.trim().to_owned();
            if value.chars().count() > 2048 {
                return Err(invalid("Each field must be 2048 characters or fewer"));
            }
        }
        if !(0..=1).contains(&kind) || self.name.is_empty() || self.name.chars().count() > 160 {
            return Err(invalid(
                "Choose a type and enter a name of 1 to 160 characters",
            ));
        }
        self.tags = parse_tags(&self.tags)?.join(", ");
        if kind == 0 {
            for port in self
                .ports
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if port.parse::<u16>().ok().is_none_or(|p| p == 0) {
                    return Err(invalid(
                        "Ports must be comma-separated numbers from 1 to 65535",
                    ));
                }
            }
        } else {
            self.name = self.name.to_ascii_lowercase();
            self.parent = self.parent.to_ascii_lowercase();
            if !valid_host(&self.name)
                || (!self.parent.is_empty()
                    && (!valid_host(&self.parent)
                        || !self.name.ends_with(&format!(".{}", self.parent))))
            {
                return Err(invalid(
                    "Use a hostname such as api.example.com; parent must be a DNS suffix such as example.com (use punycode for international names)",
                ));
            }
            if !valid_date(&self.registered)
                || !valid_date(&self.expires)
                || (!self.registered.is_empty()
                    && !self.expires.is_empty()
                    && self.registered > self.expires)
            {
                return Err(invalid(
                    "Use valid YYYY-MM-DD dates; expiry cannot precede registration",
                ));
            }
        }
        Ok(())
    }
}

impl Repository {
    pub fn inventory_entries(&self) -> Result<Vec<Entry>, StorageError> {
        let mut statement=self.connection.prepare("SELECT id,kind,data,credentials IS NOT NULL,hidden FROM operational_inventory ORDER BY id DESC LIMIT 5001")?;
        let values = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() > 5000 {
            return Err(invalid("Inventory exceeds the 5000-record limit"));
        }
        let mut hosts = self.connection.prepare(
            "SELECT server_id FROM operational_hosts WHERE inventory_id=?1 ORDER BY server_id",
        )?;
        values
            .into_iter()
            .map(|(id, kind, json, has_credentials, hidden)| {
                Ok(Entry {
                    id,
                    kind,
                    details: serde_json::from_str(&json)?,
                    has_credentials,
                    hidden,
                    hosts: hosts
                        .query_map([id], |row| row.get(0))?
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect()
    }

    /// None keeps existing credentials; Some(None) removes; Some(Some(blob)) replaces.
    pub fn save_inventory(
        &mut self,
        id: i64,
        kind: i32,
        mut details: Details,
        hosts: &[i64],
        credentials: Option<Option<Vec<u8>>>,
    ) -> Result<i64, StorageError> {
        details.validate(kind)?;
        if hosts.len() > 100 {
            return Err(invalid("Choose at most 100 hosting servers"));
        }
        let json = serde_json::to_string(&details)?;
        if json.len() > 32768 {
            return Err(invalid("Combined metadata is too large"));
        }
        let transaction = self.connection.transaction()?;
        for host in hosts {
            let exists:bool=transaction.query_row("SELECT EXISTS(SELECT 1 FROM server_records WHERE id=?1 AND (archived_at IS NULL OR EXISTS(SELECT 1 FROM operational_hosts WHERE inventory_id=?2 AND server_id=?1)))",params![host,id],|r|r.get(0))?;
            if !exists {
                return Err(invalid("A hosting server is hidden or no longer exists"));
            }
        }
        let saved = if id == 0 {
            let count: i64 =
                transaction.query_row("SELECT COUNT(*) FROM operational_inventory", [], |r| {
                    r.get(0)
                })?;
            if count >= 5000 {
                return Err(invalid("Inventory is limited to 5000 records"));
            }
            transaction.execute(
                "INSERT INTO operational_inventory(kind,data) VALUES(?1,?2)",
                params![kind, json],
            )?;
            transaction.last_insert_rowid()
        } else {
            if transaction.execute(
                "UPDATE operational_inventory SET data=?1 WHERE id=?2 AND kind=?3 AND hidden=0",
                params![json, id, kind],
            )? != 1
            {
                return Err(invalid(
                    "Record is hidden, missing, or has a different type",
                ));
            }
            id
        };
        if let Some(protected) = credentials {
            if kind != 0 && protected.is_some() {
                return Err(invalid("Credentials belong to service records"));
            }
            // Check the blob before it can become durable. No plaintext fallback.
            if let Some(ref value) = protected {
                let mut plain = secrets::unprotect(value).map_err(invalid)?;
                let valid = serde_json::from_slice::<(String, String)>(&plain).is_ok();
                plain.fill(0);
                if !valid {
                    return Err(invalid("Invalid protected account data"));
                }
            }
            transaction.execute(
                "UPDATE operational_inventory SET credentials=?1 WHERE id=?2",
                params![protected, saved],
            )?;
        }
        transaction.execute(
            "DELETE FROM operational_hosts WHERE inventory_id=?1",
            [saved],
        )?;
        for host in hosts {
            transaction.execute(
                "INSERT OR IGNORE INTO operational_hosts(inventory_id,server_id) VALUES(?1,?2)",
                params![saved, host],
            )?;
        }
        transaction.commit()?;
        Ok(saved)
    }

    pub fn inventory_credentials(&self, id: i64) -> Result<(String, String), StorageError> {
        let protected: Option<Vec<u8>> = self.connection.query_row(
            "SELECT credentials FROM operational_inventory WHERE id=?1 AND hidden=0",
            [id],
            |r| r.get(0),
        )?;
        let Some(protected) = protected else {
            return Ok((String::new(), String::new()));
        };
        let mut plain = secrets::unprotect(&protected).map_err(invalid)?;
        let result = serde_json::from_slice(&plain);
        plain.fill(0);
        result.map_err(StorageError::from)
    }

    pub fn set_inventory_hidden(&self, id: i64, hidden: bool) -> Result<(), StorageError> {
        if self.connection.execute(
            "UPDATE operational_inventory SET hidden=?1 WHERE id=?2",
            params![hidden, id],
        )? != 1
        {
            return Err(invalid("Inventory record no longer exists"));
        }
        Ok(())
    }

    pub fn add_inventory_relation(
        &self,
        source: i64,
        target: i64,
        kind: i32,
        notes: &str,
    ) -> Result<(), StorageError> {
        if source == target || !(0..=3).contains(&kind) || notes.chars().count() > 2048 {
            return Err(invalid(
                "Choose another service and keep relation notes under 2048 characters",
            ));
        }
        let count:i64=self.connection.query_row("SELECT COUNT(*) FROM operational_inventory WHERE id IN (?1,?2) AND kind=0 AND hidden=0",params![source,target],|r|r.get(0))?;
        if count != 2 {
            return Err(invalid("Relations require two active service records"));
        }
        let count: i64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM operational_relations", [], |r| {
                    r.get(0)
                })?;
        if count >= 20000 {
            return Err(invalid("Relations are limited to 20000 records"));
        }
        self.connection.execute("INSERT INTO operational_relations(source_id,target_id,kind,notes) VALUES(?1,?2,?3,?4) ON CONFLICT(source_id,target_id,kind) DO UPDATE SET notes=excluded.notes",params![source,target,kind,notes.trim()])?;
        Ok(())
    }
}

fn message(window: &AppWindow, result: Result<(), StorageError>) {
    match result {
        Ok(()) => {}
        Err(error) => crate::show_error(window, &error),
    }
}

pub fn clear_credentials(window: &AppWindow) {
    window.set_inventory_username("".into());
    window.set_inventory_password("".into());
    window.set_inventory_reveal(false);
    window.set_inventory_clear_credentials(false);
}

fn form(entry: &Entry) -> InventoryForm {
    let d = &entry.details;
    InventoryForm {
        id: entry.id as i32,
        kind: entry.kind,
        name: d.name.clone().into(),
        address: d.address.clone().into(),
        version: d.version.clone().into(),
        ports: d.ports.clone().into(),
        parent: d.parent.clone().into(),
        registrar: d.registrar.clone().into(),
        registered: d.registered.clone().into(),
        expires: d.expires.clone().into(),
        cdn: d.cdn.clone().into(),
        tags: d.tags.clone().into(),
        notes: d.notes.clone().into(),
        has_credentials: entry.has_credentials,
    }
}

pub fn refresh(window: &AppWindow, repository: &Repository) -> Result<(), StorageError> {
    let entries = repository.inventory_entries()?;
    let kind = window.get_inventory_form().kind;
    let query = window.get_inventory_query().trim().to_lowercase();
    let host_filter = i64::from(window.get_inventory_server_filter());
    let servers = repository.server_records()?;
    let hidden_servers = repository.hidden_server_records()?;
    let mut rows = Vec::new();
    for entry in &entries {
        let host_names = entry
            .hosts
            .iter()
            .map(|id| {
                servers
                    .iter()
                    .find(|s| s.id == *id)
                    .map(|s| format!("{} ({})", s.name, s.ip_address))
                    .or_else(|| {
                        hidden_servers
                            .iter()
                            .find(|s| s.id == *id)
                            .map(|s| format!("{} [hidden]", s.name))
                    })
                    .unwrap_or_else(|| "Missing server".into())
            })
            .collect::<Vec<_>>()
            .join(", ");
        let data = serde_json::to_string(&entry.details)?;
        if entry.kind == kind
            && entry.hidden == window.get_inventory_show_hidden()
            && (host_filter == 0 || entry.hosts.contains(&host_filter))
            && (query.is_empty()
                || format!("{data} {host_names}")
                    .to_lowercase()
                    .contains(&query))
        {
            let d = &entry.details;
            rows.push(InventoryRow {
                id: entry.id as i32,
                name: d.name.clone().into(),
                summary: if kind == 0 {
                    format!("{}  ·  {}  ·  {}", d.version, d.ports, d.address)
                } else {
                    format!("{}  ·  {}  ·  {}", d.parent, d.registrar, d.expires)
                }
                .into(),
                hosts: host_names.into(),
                tags: d.tags.clone().into(),
                notes: d.notes.clone().into(),
            });
        }
    }
    window.set_inventory_rows(ModelRc::new(VecModel::from(rows)));
    let selected = entries
        .iter()
        .find(|r| r.id == i64::from(window.get_inventory_form().id));
    // Refreshing search results must not discard the unsaved hosting choices.
    if window.get_inventory_hosts().row_count() == 0 {
        let choices = servers
            .iter()
            .chain(
                hidden_servers
                    .iter()
                    .filter(|s| selected.is_some_and(|e| e.hosts.contains(&s.id))),
            )
            .map(|s| InventoryChoice {
                id: s.id as i32,
                label: format!(
                    "{} ({}){}",
                    s.name,
                    s.ip_address,
                    if hidden_servers.iter().any(|h| h.id == s.id) {
                        " [hidden]"
                    } else {
                        ""
                    }
                )
                .into(),
                selected: selected.map_or(host_filter == s.id, |e| e.hosts.contains(&s.id)),
            })
            .collect::<Vec<_>>();
        window.set_inventory_hosts(ModelRc::new(VecModel::from(choices)));
    }
    window.set_inventory_targets(ModelRc::new(VecModel::from(
        entries
            .iter()
            .filter(|e| {
                e.kind == 0 && !e.hidden && e.id != i64::from(window.get_inventory_form().id)
            })
            .map(|e| InventoryChoice {
                id: e.id as i32,
                label: e.details.name.clone().into(),
                selected: false,
            })
            .collect::<Vec<_>>(),
    )));
    window.set_inventory_target_labels(ModelRc::new(VecModel::from(
        window
            .get_inventory_targets()
            .iter()
            .map(|t| t.label)
            .collect::<Vec<_>>(),
    )));
    let mut statement=repository.connection.prepare("SELECT id,source_id,target_id,kind,notes FROM operational_relations WHERE source_id=?1 OR target_id=?1 ORDER BY id LIMIT 20000")?;
    let relations = statement
        .query_map([i64::from(window.get_inventory_form().id)], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, usize>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let labels = [
        "depends on",
        "replicates to",
        "fails over to",
        "balances to",
    ];
    let rows = relations
        .into_iter()
        .filter_map(|(id, source, target, kind, notes)| {
            let from = entries.iter().find(|e| e.id == source)?;
            let to = entries.iter().find(|e| e.id == target)?;
            Some(InventoryRow {
                id: id as i32,
                name: format!(
                    "{}{} → {} → {}{}",
                    from.details.name,
                    if from.hidden { " [hidden]" } else { "" },
                    labels.get(kind).unwrap_or(&"related to"),
                    to.details.name,
                    if to.hidden { " [hidden]" } else { "" }
                )
                .into(),
                notes: notes.into(),
                ..Default::default()
            })
        })
        .collect::<Vec<_>>();
    window.set_inventory_relations(ModelRc::new(VecModel::from(rows)));
    window.set_inventory_filter_label(if host_filter == 0 {
        "All servers".into()
    } else {
        servers
            .iter()
            .find(|s| s.id == host_filter)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Selected server".into())
            .into()
    });
    Ok(())
}

pub fn setup(window: &AppWindow, repository: Rc<RefCell<Repository>>) {
    window.set_inventory_credentials_available(secrets::AVAILABLE);
    let timer = Rc::new(slint::Timer::default());
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_refresh(move || {
        if let Some(w) = weak.upgrade() {
            message(&w, refresh(&w, &repo.borrow()));
        }
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_select(move |id| {
        let Some(w) = weak.upgrade() else { return };
        clear_credentials(&w);
        w.set_inventory_target_index(0);
        w.set_inventory_relation_notes("".into());
        w.set_inventory_hosts(ModelRc::default());
        let result = (|| {
            let entries = repo.borrow().inventory_entries()?;
            if let Some(entry) = entries.iter().find(|e| e.id == i64::from(id)) {
                w.set_inventory_form(form(entry));
            }
            refresh(&w, &repo.borrow())
        })();
        message(&w, result);
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_new(move |kind| {
        let Some(w) = weak.upgrade() else { return };
        clear_credentials(&w);
        w.set_inventory_hosts(ModelRc::default());
        w.set_inventory_form(InventoryForm {
            kind,
            ..Default::default()
        });
        w.set_inventory_relation_notes("".into());
        w.set_inventory_target_index(0);
        message(&w, refresh(&w, &repo.borrow()));
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_save(move || {
        let Some(w) = weak.upgrade() else { return };
        let result = (|| {
            let f = w.get_inventory_form();
            let mut credentials = None;
            if w.get_inventory_clear_credentials() {
                credentials = Some(None);
            } else if !w.get_inventory_username().is_empty()
                || !w.get_inventory_password().is_empty()
            {
                if w.get_inventory_username().is_empty() || w.get_inventory_password().is_empty() {
                    return Err(invalid(
                        "Enter both username and password to replace the saved account",
                    ));
                }
                if w.get_inventory_username().len() > 1024
                    || w.get_inventory_password().len() > 4096
                {
                    return Err(invalid("Username or password is too long"));
                }
                let mut plain = serde_json::to_vec(&(
                    w.get_inventory_username().to_string(),
                    w.get_inventory_password().to_string(),
                ))?;
                let protected = secrets::protect(&plain);
                plain.fill(0);
                credentials = Some(Some(protected.map_err(invalid)?));
            }
            let details = Details {
                name: f.name.to_string(),
                address: f.address.to_string(),
                version: f.version.to_string(),
                ports: f.ports.to_string(),
                parent: f.parent.to_string(),
                registrar: f.registrar.to_string(),
                registered: f.registered.to_string(),
                expires: f.expires.to_string(),
                cdn: f.cdn.to_string(),
                tags: f.tags.to_string(),
                notes: f.notes.to_string(),
            };
            let hosts = w
                .get_inventory_hosts()
                .iter()
                .filter(|h| h.selected)
                .map(|h| i64::from(h.id))
                .collect::<Vec<_>>();
            let id = repo.borrow_mut().save_inventory(
                i64::from(f.id),
                f.kind,
                details,
                &hosts,
                credentials,
            )?;
            clear_credentials(&w);
            w.invoke_inventory_select(id as i32);
            crate::show_status(&w, "Inventory saved");
            Ok(())
        })();
        message(&w, result);
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_hide(move |id, hidden| {
        let Some(w) = weak.upgrade() else { return };
        let result = repo.borrow().set_inventory_hidden(i64::from(id), hidden);
        if result.is_ok() {
            let kind = w.get_inventory_form().kind;
            w.invoke_inventory_new(kind);
        }
        message(&w, result);
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    let expiry = timer.clone();
    window.on_inventory_unlock(move || {
        let Some(w) = weak.upgrade() else { return };
        match repo
            .borrow()
            .inventory_credentials(i64::from(w.get_inventory_form().id))
        {
            Ok((username, password)) => {
                w.set_inventory_username(username.into());
                w.set_inventory_password(password.into());
                let weak = w.as_weak();
                expiry.start(
                    slint::TimerMode::SingleShot,
                    Duration::from_secs(30),
                    move || {
                        if let Some(w) = weak.upgrade() {
                            clear_credentials(&w);
                        }
                    },
                );
                crate::show_status(&w, "Credentials unlocked for 30 seconds");
            }
            Err(error) => crate::show_error(&w, &error),
        }
    });
    let weak = window.as_weak();
    window.on_inventory_copy_secret(move |password| {
        let Some(w)=weak.upgrade() else{return};
        // Read from the editor only after an explicit unlock or user entry.
        let value=if password {w.get_inventory_password()}else{w.get_inventory_username()}.to_string();
        if value.is_empty(){crate::show_error(&w,&"Unlock or enter credentials first");return;}
        match arboard::Clipboard::new().and_then(|mut c|c.set_text(value.clone())) {
            Ok(())=>{
                crate::show_status(&w,"Copied; clipboard clears after 30 seconds if unchanged (clipboard history may retain a copy)");
                slint::Timer::single_shot(Duration::from_secs(30),move ||{if let Ok(mut c)=arboard::Clipboard::new() && c.get_text().ok().as_deref()==Some(&value){let _=c.clear();}});
            },Err(_)=>crate::show_error(&w,&"Clipboard is unavailable"),
        }
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_relation_add(move || {
        let Some(w) = weak.upgrade() else { return };
        let Some(target) = w
            .get_inventory_targets()
            .row_data(w.get_inventory_target_index().max(0) as usize)
        else {
            crate::show_error(&w, &"Add another service first");
            return;
        };
        let result = repo.borrow().add_inventory_relation(
            i64::from(w.get_inventory_form().id),
            i64::from(target.id),
            w.get_inventory_relation_kind(),
            &w.get_inventory_relation_notes(),
        );
        if result.is_ok() {
            let _ = refresh(&w, &repo.borrow());
            w.set_inventory_relation_notes("".into());
        }
        message(&w, result);
    });
    let weak = window.as_weak();
    window.on_inventory_relation_remove(move |id| {
        let Some(w) = weak.upgrade() else { return };
        let result = repository
            .borrow()
            .connection
            .execute(
                "DELETE FROM operational_relations WHERE id=?1 AND (source_id=?2 OR target_id=?2)",
                params![id, w.get_inventory_form().id],
            )
            .map(|_| ())
            .map_err(StorageError::from);
        if result.is_ok() {
            let _ = refresh(&w, &repository.borrow());
        }
        message(&w, result);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn service(name: &str) -> Details {
        Details {
            name: name.into(),
            ..Details::default()
        }
    }

    #[test]
    fn validation_rejects_invalid_dates_hosts_and_ports() {
        assert!(valid_date("2024-02-29"));
        assert!(!valid_date("2025-02-29"));
        assert!(!valid_date("2026-04-31"));
        assert!(!valid_date("１２３４-01-01"));
        let mut d = service("API");
        d.ports = "0,443".into();
        assert!(d.validate(0).is_err());
        d.ports = "443,8080".into();
        assert!(d.validate(0).is_ok());
        d = service("api.example.com");
        d.parent = "other.example.com".into();
        assert!(d.validate(1).is_err());
        d.parent = "example.com".into();
        assert!(d.validate(1).is_ok());
        d.registered = "2026-10-24".into();
        d.expires = "2026-10-23".into();
        assert!(d.validate(1).is_err());
    }

    #[test]
    fn inventory_relations_are_directed_bounded_and_validate_targets() -> Result<(), StorageError> {
        let mut r = Repository::in_memory()?;
        let a = r.save_inventory(0, 0, service("primary"), &[], None)?;
        let b = r.save_inventory(0, 0, service("replica"), &[], None)?;
        let domain = r.save_inventory(0, 1, service("example.com"), &[], None)?;
        r.add_inventory_relation(a, b, 1, "Streaming replication")?;
        assert!(r.add_inventory_relation(a, a, 0, "").is_err());
        assert!(r.add_inventory_relation(a, domain, 0, "").is_err());
        assert!(
            r.save_inventory(0, 0, service("bad host"), &[i64::MAX], None)
                .is_err()
        );
        assert_eq!(r.inventory_entries()?.len(), 3);
        let endpoints: (i64, i64) = r.connection.query_row(
            "SELECT source_id,target_id FROM operational_relations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(endpoints, (a, b));
        r.set_inventory_hidden(b, true)?;
        assert!(r.add_inventory_relation(a, b, 0, "").is_err());
        r.set_inventory_hidden(b, false)?;
        r.add_inventory_relation(a, b, 1, "Updated note")?;
        assert_eq!(
            r.connection
                .query_row("SELECT COUNT(*) FROM operational_relations", [], |row| row
                    .get::<_, i64>(
                    0
                ))?,
            1
        );
        Ok(())
    }

    #[test]
    fn ui_bridge_preserves_drafts_filters_hosts_and_shows_reverse_relations()
    -> Result<(), StorageError> {
        i_slint_backend_testing::init_no_event_loop();
        let mut r = Repository::in_memory()?;
        r.reset_demo_fixture()?;
        let hosts = r.server_records()?;
        let a = r.save_inventory(0, 0, service("primary"), &[hosts[0].id], None)?;
        let b = r.save_inventory(0, 0, service("replica"), &[hosts[1].id], None)?;
        r.add_inventory_relation(a, b, 1, "streaming")?;
        let repo = Rc::new(RefCell::new(r));
        let w = AppWindow::new().unwrap();
        setup(&w, repo.clone());
        w.invoke_inventory_select(b as i32);
        assert_eq!(w.get_inventory_relations().row_count(), 1);
        let relation = w.get_inventory_relations().row_data(0).unwrap();
        assert!(relation.name.starts_with("primary"));
        assert!(relation.name.ends_with("replica"));
        assert_eq!(relation.notes, "streaming");
        w.set_inventory_username("transient synthetic credential".into());
        w.invoke_inventory_new(0);
        assert!(w.get_inventory_username().is_empty());
        let mut f = w.get_inventory_form();
        f.name = "orders api".into();
        f.tags = "production, orders".into();
        w.set_inventory_form(f);
        let choices = w.get_inventory_hosts();
        let mut choice = choices.row_data(0).unwrap();
        choice.selected = true;
        choices.set_row_data(0, choice.clone());
        w.set_inventory_query("production".into());
        w.invoke_inventory_refresh();
        assert!(w.get_inventory_hosts().row_data(0).unwrap().selected);
        w.invoke_inventory_save();
        assert!(!w.get_status_is_error());
        let saved = repo
            .borrow()
            .inventory_entries()?
            .into_iter()
            .find(|e| e.details.name == "orders api")
            .unwrap();
        assert_eq!(saved.hosts, vec![i64::from(choice.id)]);
        assert_eq!(w.get_inventory_rows().row_count(), 1);
        w.set_inventory_server_filter(hosts[1].id as i32);
        w.invoke_inventory_refresh();
        assert_eq!(w.get_inventory_rows().row_count(), 0);
        w.set_inventory_query("".into());
        w.invoke_inventory_refresh();
        assert_eq!(w.get_inventory_rows().row_count(), 1);
        Ok(())
    }

    #[test]
    fn thousand_record_inventory_search_is_bounded() -> Result<(), StorageError> {
        i_slint_backend_testing::init_no_event_loop();
        let mut r = Repository::in_memory()?;
        for index in 0..1000 {
            let mut d = service(&format!("service-{index}"));
            d.tags = if index % 10 == 0 {
                "production"
            } else {
                "test"
            }
            .into();
            r.save_inventory(0, 0, d, &[], None)?;
        }
        let w = AppWindow::new().unwrap();
        w.set_inventory_query("production".into());
        let start = std::time::Instant::now();
        refresh(&w, &r)?;
        let elapsed = start.elapsed();
        assert_eq!(w.get_inventory_rows().row_count(), 100);
        eprintln!(
            "synthetic 1000-record inventory refresh: {elapsed:?}; excludes native paint, disk and credentials"
        );
        Ok(())
    }

    #[test]
    fn credential_failure_rolls_back_the_whole_save() -> Result<(), StorageError> {
        let mut r = Repository::in_memory()?;
        assert!(
            r.save_inventory(
                0,
                0,
                service("not saved"),
                &[],
                Some(Some(b"plaintext is not a protected blob".to_vec()))
            )
            .is_err()
        );
        assert!(r.inventory_entries()?.is_empty());
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn credentials_stay_out_of_metadata_and_survive_edits() -> Result<(), StorageError> {
        let mut r = Repository::in_memory()?;
        let protected =
            secrets::protect(br#"["synthetic-user","synthetic-password"]"#).map_err(invalid)?;
        let id = r.save_inventory(0, 0, service("database"), &[], Some(Some(protected)))?;
        r.save_inventory(id, 0, service("renamed database"), &[], None)?;
        assert_eq!(
            r.inventory_credentials(id)?,
            ("synthetic-user".into(), "synthetic-password".into())
        );
        let entries = r.inventory_entries()?;
        assert!(!serde_json::to_string(&entries[0].details)?.contains("synthetic-password"));
        r.save_inventory(id, 0, service("database"), &[], Some(None))?;
        assert!(!r.inventory_entries()?[0].has_credentials);
        Ok(())
    }
}

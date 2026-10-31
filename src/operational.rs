//! Operational metadata uses the visible server inventory, not legacy planning assets.
use std::{cell::RefCell, fs, path::Path, rc::Rc, time::Duration};

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::storage::{Repository, StorageError};
use crate::{
    AppWindow, InventoryChoice, InventoryForm, InventoryHost, InventoryRow, domain::parse_tags,
    secrets,
};

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
    #[serde(default)]
    pub business: String,
    pub tags: String,
    pub notes: String,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub transport: String,
    #[serde(default)]
    pub pending_confirmation: bool,
}

pub struct Entry {
    pub id: i64,
    pub kind: i32,
    pub details: Details,
    pub hosts: Vec<i64>,
    pub has_credentials: bool,
    pub hidden: bool,
    pub verified_at: Option<String>,
    pub verification_due: bool,
    pub updated_at: String,
}

fn invalid(message: &str) -> StorageError {
    StorageError::InvalidImport(message.into())
}

fn connection_target(details: &Details) -> Result<(bool, String), StorageError> {
    if details.pending_confirmation {
        return Err(invalid("Review and confirm this service before connecting"));
    }
    let port = details
        .ports
        .parse::<u16>()
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| invalid("Choose one port from 1 to 65535 for this service"))?;
    let host = details
        .address
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if host.is_empty()
        || host.starts_with('-')
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-:_".contains(&b))
    {
        return Err(invalid(
            "Use a server IP address or hostname without a URL, username or password",
        ));
    }
    let address = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    match details.protocol.trim().to_ascii_uppercase().as_str() {
        "HTTP" => Ok((true, format!("http://{address}"))),
        "HTTPS" => Ok((true, format!("https://{address}"))),
        "SSH" => Ok((false, format!("ssh -p {port} {host}"))),
        _ => Ok((false, address)),
    }
}

/// Move legacy access points into linked services inside the caller's transaction.
/// Existing services with the same host and port are kept, including their credentials.
pub(crate) fn seed_server_services(
    connection: &rusqlite::Connection,
    server_id: i64,
    include_default_ssh: bool,
) -> Result<(), StorageError> {
    let (name, address, tags, hidden): (String, String, String, bool) = connection.query_row(
        "SELECT name,ip_address,tags,archived_at IS NOT NULL FROM server_records WHERE id=?1",
        [server_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let mut statement = connection.prepare(
        "SELECT port,transport,protocol,purpose FROM server_endpoints WHERE server_id=?1 ORDER BY id",
    )?;
    let mut endpoints = statement
        .query_map([server_id], |row| {
            Ok((
                row.get::<_, u16>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if include_default_ssh && !endpoints.iter().any(|e| e.0 == 22) {
        endpoints.push((22, "TCP".into(), "SSH".into(), "ssh".into()));
    }
    for (port, transport, protocol, notes) in endpoints {
        let protocol = if protocol.trim().is_empty() && port == 22 {
            "SSH".to_owned()
        } else {
            protocol.trim().to_ascii_uppercase()
        };
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM operational_inventory i JOIN operational_hosts h ON h.inventory_id=i.id WHERE h.server_id=?1 AND i.kind=0 AND (',' || replace(json_extract(i.data,'$.ports'),' ','') || ',') LIKE ?2 AND (coalesce(json_extract(i.data,'$.transport'),'') IN ('','Unspecified') OR json_extract(i.data,'$.transport')=?3))",
            params![server_id, format!("%,{port},%"), transport], |r| r.get(0),
        )?;
        if exists {
            continue;
        }
        let count: i64 =
            connection.query_row("SELECT count(*) FROM operational_inventory", [], |r| {
                r.get(0)
            })?;
        if count >= 5000 {
            return Err(invalid(
                "Inventory is limited to 5000 records; no server or port changes were saved",
            ));
        }
        let details = Details {
            name: format!(
                "{} - {}",
                name.chars().take(110).collect::<String>(),
                if protocol.is_empty() {
                    format!("Port {port}")
                } else {
                    protocol.clone()
                }
            ),
            address: address.clone(),
            ports: port.to_string(),
            tags: tags.clone(),
            notes: if notes.is_empty() && port == 22 {
                "ssh".into()
            } else {
                notes
            },
            protocol,
            transport: if transport == "Unspecified" && port == 22 {
                "TCP".into()
            } else {
                transport
            },
            pending_confirmation: true,
            ..Default::default()
        };
        connection.execute(
            "INSERT INTO operational_inventory(kind,data,hidden) VALUES(0,?1,?2)",
            params![serde_json::to_string(&details)?, hidden],
        )?;
        let id = connection.last_insert_rowid();
        connection.execute(
            "INSERT INTO operational_hosts(inventory_id,server_id) VALUES(?1,?2)",
            params![id, server_id],
        )?;
    }
    Ok(())
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
            &mut self.business,
            &mut self.tags,
            &mut self.notes,
            &mut self.protocol,
            &mut self.transport,
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
        if self.business.chars().count() > 160 {
            return Err(invalid("Business name must be 160 characters or fewer"));
        }
        self.protocol = self.protocol.to_ascii_uppercase();
        if self.protocol.chars().count() > 40
            || !["", "Unspecified", "TCP", "UDP"].contains(&self.transport.as_str())
        {
            return Err(invalid(
                "Use a protocol of at most 40 characters and TCP, UDP or Unspecified transport",
            ));
        }
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
        let mut statement=self.connection.prepare("SELECT id,kind,data,credentials IS NOT NULL,hidden,verified_at,verified_at IS NULL OR date(verified_at)<date('now','-90 days'),updated_at FROM operational_inventory ORDER BY id DESC LIMIT 5001")?;
        let values = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, bool>(6)?,
                    row.get::<_, String>(7)?,
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
            .map(
                |(
                    id,
                    kind,
                    json,
                    has_credentials,
                    hidden,
                    verified_at,
                    verification_due,
                    updated_at,
                )| {
                    Ok(Entry {
                        id,
                        kind,
                        details: serde_json::from_str(&json)?,
                        has_credentials,
                        hidden,
                        verified_at,
                        verification_due,
                        updated_at,
                        hosts: hosts
                            .query_map([id], |row| row.get(0))?
                            .collect::<Result<Vec<_>, _>>()?,
                    })
                },
            )
            .collect()
    }

    /// None keeps existing credentials; Some(None) removes; Some(Some(blob)) replaces.
    pub fn save_inventory(
        &mut self,
        id: i64,
        kind: i32,
        details: Details,
        hosts: &[i64],
        credentials: Option<Option<Vec<u8>>>,
    ) -> Result<i64, StorageError> {
        self.save_inventory_with_context(id, kind, details, hosts, credentials, None)
    }

    pub fn save_inventory_with_context(
        &mut self,
        id: i64,
        kind: i32,
        mut details: Details,
        hosts: &[i64],
        credentials: Option<Option<Vec<u8>>>,
        context: Option<&crate::AssetContextForm>,
    ) -> Result<i64, StorageError> {
        details.validate(kind)?;
        if hosts.len() > 100 {
            return Err(invalid("Choose at most 100 hosting servers"));
        }
        let transaction = self.connection.transaction()?;
        let business_id = if details.business.trim().is_empty() {
            None
        } else {
            transaction.execute(
                "INSERT INTO businesses(name) VALUES(?1) ON CONFLICT DO NOTHING",
                [details.business.trim()],
            )?;
            Some(transaction.query_row(
                "SELECT id FROM businesses WHERE lower(trim(name))=lower(trim(?1)) AND archived_at IS NULL",
                [details.business.trim()],
                |row| row.get::<_, i64>(0),
            )?)
        };
        let parent_domain_id = if kind == 1 && !details.parent.trim().is_empty() {
            transaction
                .query_row(
                    "SELECT id FROM operational_inventory WHERE kind=1 AND hidden=0 AND lower(json_extract(data,'$.name'))=lower(?1) AND id<>?2 ORDER BY id LIMIT 1",
                    params![details.parent.trim(), id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
        } else {
            None
        };
        if let Some(business) = business_id {
            details.business = transaction.query_row(
                "SELECT name FROM businesses WHERE id=?1",
                [business],
                |r| r.get(0),
            )?;
        }
        if let Some(parent) = parent_domain_id {
            let cycle: bool = transaction.query_row("WITH RECURSIVE parents(id) AS (SELECT ?1 UNION SELECT parent_domain_id FROM operational_inventory JOIN parents ON operational_inventory.id=parents.id WHERE parent_domain_id IS NOT NULL) SELECT EXISTS(SELECT 1 FROM parents WHERE id=?2)", params![parent,id], |r| r.get(0))?;
            if cycle {
                return Err(invalid("A domain cannot contain its own ancestor"));
            }
            details.parent = transaction.query_row(
                "SELECT json_extract(data,'$.name') FROM operational_inventory WHERE id=?1",
                [parent],
                |r| r.get(0),
            )?;
        }
        if kind == 1
            && !details.parent.is_empty()
            && details.parent.eq_ignore_ascii_case(&details.name)
        {
            return Err(invalid("A domain cannot be its own parent"));
        }
        let json = serde_json::to_string(&details)?;
        if json.len() > 32768 {
            return Err(invalid("Combined metadata is too large"));
        }
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
                "INSERT INTO operational_inventory(kind,data,business_id,parent_domain_id) VALUES(?1,?2,?3,?4)",
                params![kind, json, business_id, parent_domain_id],
            )?;
            transaction.last_insert_rowid()
        } else {
            let old_data: String = transaction
                .query_row(
                    "SELECT data FROM operational_inventory WHERE id=?1 AND kind=?2 AND hidden=0",
                    params![id, kind],
                    |row| row.get(0),
                )
                .map_err(|_| invalid("Record is hidden, missing, or has a different type"))?;
            let mut host_statement = transaction.prepare(
                "SELECT server_id FROM operational_hosts WHERE inventory_id=?1 ORDER BY server_id",
            )?;
            let mut old_hosts = host_statement
                .query_map([id], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            let mut new_hosts = hosts.to_vec();
            old_hosts.sort_unstable();
            new_hosts.sort_unstable();
            new_hosts.dedup();
            let changed = old_data != json || old_hosts != new_hosts;
            if transaction.execute(
                "UPDATE operational_inventory SET verified_at=CASE WHEN ?1 THEN NULL ELSE verified_at END,data=?2,business_id=?3,parent_domain_id=?4 WHERE id=?5 AND kind=?6 AND hidden=0",
                params![changed, json, business_id, parent_domain_id, id, kind],
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
            "DELETE FROM operational_hosts WHERE inventory_id=?1 AND server_id NOT IN (SELECT CAST(value AS INTEGER) FROM json_each(?2))",
            params![saved, serde_json::to_string(hosts)?],
        )?;
        for host in hosts {
            transaction.execute(
                "INSERT OR IGNORE INTO operational_hosts(inventory_id,server_id) VALUES(?1,?2)",
                params![saved, host],
            )?;
        }
        transaction.execute("DELETE FROM resource_links WHERE target_type=?1 AND target_id=?2 AND (relation_type='owns' AND source_type='business' OR relation_type='contains' AND source_type='domain')",params![if kind==0 {"service"}else{"domain"},saved])?;
        if let Some(business) = business_id {
            transaction.execute("INSERT OR IGNORE INTO resource_links(source_type,source_id,target_type,target_id,relation_type) VALUES('business',?1,?2,?3,'owns')",params![business,if kind==0 {"service"}else{"domain"},saved])?;
        }
        if let Some(parent) = parent_domain_id {
            transaction.execute("INSERT OR IGNORE INTO resource_links(source_type,source_id,target_type,target_id,relation_type) VALUES('domain',?1,'domain',?2,'contains')",params![parent,saved])?;
        }
        if kind == 1 {
            transaction.execute("UPDATE operational_inventory SET data=json_set(data,'$.parent',?1) WHERE parent_domain_id=?2 AND json_extract(data,'$.parent') IS NOT ?1", params![details.name,saved])?;
        }
        crate::asset_context::save_context(
            &transaction,
            if kind == 0 { "service" } else { "domain" },
            saved,
            context,
        )?;
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

    pub fn mark_inventory_verified(&self, id: i64) -> Result<(), StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        if tx.execute("UPDATE operational_inventory SET verified_at=date('now'),data=json_set(data,'$.pending_confirmation',json('false')) WHERE id=?1 AND hidden=0", [id])? != 1 {
            return Err(invalid("Inventory record is hidden or no longer exists"));
        }
        tx.execute(
            "UPDATE operational_inventory SET verified_at=date('now') WHERE id=?1",
            [id],
        )?;
        tx.execute("INSERT INTO asset_context(resource_type,resource_id,verified_at) SELECT CASE kind WHEN 0 THEN 'service' ELSE 'domain' END,id,datetime('now') FROM operational_inventory WHERE id=?1 ON CONFLICT(resource_type,resource_id) DO UPDATE SET verified_at=excluded.verified_at,verified_by=''", [id])?;
        tx.execute("INSERT INTO resource_history(resource_type,resource_id,action,summary) SELECT CASE kind WHEN 0 THEN 'service' ELSE 'domain' END,id,'updated','Information checked' FROM operational_inventory WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn export_inventory_csv(
        &self,
        path: &Path,
        kind: i32,
        query: &str,
        host_filter: i64,
        show_hidden: bool,
    ) -> Result<usize, StorageError> {
        let entries = self.inventory_entries()?;
        let servers = self.server_records()?;
        let contexts = self.asset_contexts()?;
        let query = query.trim().to_lowercase();
        let mut output = String::from(
            "Type,Name,Address,Version,Ports,Parent,Registrar,Registered,Expires,CDN,Servers,Tags,Notes,Last verified,Protocol,Transport,Confirmation,Technical contact,Backup contact,Environment,Documentation status,Documentation URL,Information source,Checked by\r\n",
        );
        let mut count = 0;
        for entry in entries.iter().filter(|entry| {
            entry.hidden == show_hidden
                && entry.kind == kind
                && (host_filter == 0 || entry.hosts.contains(&host_filter))
        }) {
            let host_names = entry
                .hosts
                .iter()
                .filter_map(|id| servers.iter().find(|server| server.id == *id))
                .map(|server| server.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let context = contexts
                .get(&(
                    if kind == 0 { "service" } else { "domain" }.to_owned(),
                    entry.id,
                ))
                .cloned()
                .unwrap_or_default();
            let searchable = format!(
                "{} {host_names} {}",
                serde_json::to_string(&entry.details)?,
                context.data.searchable()
            )
            .to_lowercase();
            if !query.is_empty() && !searchable.contains(&query) {
                continue;
            }
            let values = [
                if kind == 0 { "Service" } else { "Domain" },
                &entry.details.name,
                &entry.details.address,
                &entry.details.version,
                &entry.details.ports,
                &entry.details.parent,
                &entry.details.registrar,
                &entry.details.registered,
                &entry.details.expires,
                &entry.details.cdn,
                &host_names,
                &entry.details.tags,
                &entry.details.notes,
                entry.verified_at.as_deref().unwrap_or(""),
                &entry.details.protocol,
                &entry.details.transport,
                if entry.details.pending_confirmation {
                    "Pending"
                } else {
                    "Confirmed"
                },
                &context.data.owner,
                &context.data.backup_contact,
                &context.data.environment,
                [
                    "Not recorded",
                    "Unknown",
                    "Not applicable",
                    "Link available",
                ]
                .get(context.data.documentation_state as usize)
                .copied()
                .unwrap_or("Not recorded"),
                &context.data.documentation,
                &context.data.source,
                &context.verified_by,
            ];
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push('"');
                let first = value
                    .trim_start_matches([' ', '\t', '\r', '\n'])
                    .chars()
                    .next();
                let safe_value = if matches!(first, Some('=' | '+' | '-' | '@')) {
                    format!("'{}", value)
                } else {
                    (*value).to_owned()
                };
                output.push_str(&safe_value.replace('"', "\"\""));
                output.push('"');
            }
            output.push_str("\r\n");
            count += 1;
        }
        fs::write(path, output).map_err(|error| {
            StorageError::InvalidImport(format!("Could not write CSV export: {error}"))
        })?;
        Ok(count)
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
        self.connection.execute(
            "INSERT INTO resource_links(source_type,source_id,target_type,target_id,relation_type,notes) VALUES('service',?1,'service',?2,?3,?4) ON CONFLICT(source_type,source_id,target_type,target_id,relation_type) DO UPDATE SET notes=excluded.notes,updated_at=CURRENT_TIMESTAMP",
            params![source, target, ["depends_on", "replicates_to", "fails_over_to", "balances_to"][kind as usize], notes.trim()],
        )?;
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
        business: d.business.clone().into(),
        tags: d.tags.clone().into(),
        notes: d.notes.clone().into(),
        has_credentials: entry.has_credentials,
        verified_at: entry.verified_at.clone().unwrap_or_default().into(),
        protocol: d.protocol.clone().into(),
        transport: d.transport.clone().into(),
        pending_confirmation: d.pending_confirmation,
    }
}

pub fn refresh(window: &AppWindow, repository: &Repository) -> Result<(), StorageError> {
    let entries = repository.inventory_entries()?;
    let kind = window.get_inventory_form().kind;
    let query = window.get_inventory_query().trim().to_lowercase();
    let host_filter = i64::from(window.get_inventory_server_filter());
    let servers = repository.server_records()?;
    let hidden_servers = repository.hidden_server_records()?;
    let contexts = repository.asset_contexts()?;
    let mut rows = Vec::new();
    let mut grid = Vec::new();
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
        let context = contexts
            .get(&(
                if entry.kind == 0 { "service" } else { "domain" }.to_owned(),
                entry.id,
            ))
            .cloned()
            .unwrap_or_default();
        if entry.kind == kind
            && entry.hidden == window.get_inventory_show_hidden()
            && (host_filter == 0 || entry.hosts.contains(&host_filter))
            && (query.is_empty()
                || format!("{data} {host_names} {}", context.data.searchable())
                    .to_lowercase()
                    .contains(&query))
        {
            let d = &entry.details;
            grid.push(crate::tables::row(
                entry.id as i32,
                if kind == 0 {
                    vec![
                        d.name.clone(),
                        host_names.clone(),
                        d.ports.clone(),
                        d.protocol.clone(),
                        d.version.clone(),
                        if d.pending_confirmation {
                            "Pending confirmation".into()
                        } else if entry.verification_due {
                            "Needs verification".into()
                        } else {
                            "Checked".into()
                        },
                        d.tags.clone(),
                    ]
                } else {
                    vec![
                        d.name.clone(),
                        host_names.clone(),
                        d.parent.clone(),
                        d.registrar.clone(),
                        d.expires.clone(),
                        d.cdn.clone(),
                        d.tags.clone(),
                    ]
                },
            ));
            rows.push(InventoryRow {
                id: entry.id as i32,
                name: d.name.clone().into(),
                summary: if kind == 0 {
                    format!(
                        "Port {}  |  {}  |  {}  |  {}",
                        d.ports, d.protocol, d.version, d.address
                    )
                } else {
                    format!("{}  ·  {}  ·  {}", d.parent, d.registrar, d.expires)
                }
                .into(),
                hosts: ModelRc::new(VecModel::from(
                    entry
                        .hosts
                        .iter()
                        .map(|id| InventoryHost {
                            id: *id as i32,
                            label: servers
                                .iter()
                                .find(|s| s.id == *id)
                                .map(|s| format!("{} · {}", s.name, s.ip_address))
                                .or_else(|| {
                                    hidden_servers
                                        .iter()
                                        .find(|s| s.id == *id)
                                        .map(|s| format!("{} [hidden]", s.name))
                                })
                                .unwrap_or_else(|| "Missing server".into())
                                .into(),
                        })
                        .collect::<Vec<_>>(),
                )),
                tags: d.tags.clone().into(),
                notes: format!(
                    "{} | {} | {}",
                    context.data.summary(),
                    context.verification(),
                    d.notes
                )
                .into(),
                verification: if entry.details.pending_confirmation {
                    "Pending confirmation - review service settings".to_owned()
                } else if entry.verification_due {
                    entry.verified_at.as_deref().map_or_else(
                        || "Not verified · due".to_owned(),
                        |date| format!("Last checked {date} · due"),
                    )
                } else {
                    format!(
                        "Last checked {}",
                        entry.verified_at.as_deref().unwrap_or_default()
                    )
                }
                .into(),
                verified: !entry.verification_due,
                pending_confirmation: d.pending_confirmation,
                updated_at: entry.updated_at.clone().into(),
            });
        }
    }
    window.set_inventory_grid(crate::tables::sorted(
        grid,
        if kind == 0 {
            window.get_service_grid_sort()
        } else {
            window.get_domain_grid_sort()
        },
    ));
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
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_host_details(move |id| {
        let Some(w) = weak.upgrade() else { return };
        let result = (|| {
            let repo = repo.borrow();
            let entries = repo.inventory_entries()?;
            let entry = entries.iter().find(|e| e.id == i64::from(id)).ok_or_else(|| invalid("Record no longer exists"))?;
            let mut servers = repo.server_records()?;
            servers.extend(repo.hidden_server_records()?);
            let mut sections = Vec::new();
            for host in &entry.hosts {
                if let Some(server) = servers.iter().find(|s| s.id == *host) {
                    let mut text = format!("{} ({})\nEnvironment: {} | Status: {}\nTags: {}\nNotes: {}\n\nRecent history (up to 100 events):",server.name,server.ip_address,server.environment,server.status,server.tags.join(", "),server.purpose);
                    let history = repo.resource_history("server", *host)?;
                    if history.is_empty() { text.push_str("\nNo recorded events."); }
                    for event in history { text.push_str(&format!("\n{} | {} | {}",event.occurred_at,event.action,event.summary)); }
                    sections.push(text);
                }
            }
            w.set_host_details_text(if sections.is_empty() { "No hosting servers assigned.".into() } else { sections.join("\n\n--------------------\n\n").into() });
            w.set_host_details_open(true);
            Ok(())
        })();
        message(&w,result);
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_connect(move || {
        let Some(w) = weak.upgrade() else { return };
        let result = (|| {
            let entries = repo.borrow().inventory_entries()?;
            let entry = entries
                .iter()
                .find(|entry| {
                    entry.id == i64::from(w.get_inventory_form().id)
                        && entry.kind == 0
                        && !entry.hidden
                })
                .ok_or_else(|| invalid("Save and select a service first"))?;
            let mut details = entry.details.clone();
            if details.address.is_empty() && entry.hosts.len() == 1 {
                details.address = repo.borrow().connection.query_row(
                    "SELECT ip_address FROM server_records WHERE id=?1",
                    [entry.hosts[0]],
                    |r| r.get(0),
                )?;
            }
            let (browser, target) = connection_target(&details)?;
            if browser {
                webbrowser::open(&target)
                    .map_err(|_| invalid("Could not open the service in your browser"))?;
            } else {
                w.invoke_copy_value(target.into(), "Service connection".into());
            }
            Ok(())
        })();
        message(&w, result);
    });
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
                w.set_inventory_context(
                    repo.borrow()
                        .asset_context(
                            if entry.kind == 0 { "service" } else { "domain" },
                            entry.id,
                        )?
                        .form(),
                );
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
        w.set_inventory_context(Default::default());
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
                business: f.business.to_string(),
                tags: f.tags.to_string(),
                notes: f.notes.to_string(),
                protocol: f.protocol.to_string(),
                transport: f.transport.to_string(),
                // The explicit Confirm service action saves the reviewed settings.
                pending_confirmation: false,
            };
            let hosts = w
                .get_inventory_hosts()
                .iter()
                .filter(|h| h.selected)
                .map(|h| i64::from(h.id))
                .collect::<Vec<_>>();
            let id = repo.borrow_mut().save_inventory_with_context(
                i64::from(f.id),
                f.kind,
                details,
                &hosts,
                credentials,
                Some(&w.get_inventory_context()),
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
    window.on_inventory_verify(move || {
        let Some(w) = weak.upgrade() else { return };
        let id = i64::from(w.get_inventory_form().id);
        let result = repo.borrow().mark_inventory_verified(id);
        if result.is_ok() {
            let _ = refresh(&w, &repo.borrow());
            w.invoke_inventory_select(id as i32);
            crate::show_status(&w, "Inventory marked as checked");
        }
        message(&w, result);
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_inventory_export(move || {
        let Some(w) = weak.upgrade() else { return };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("itgla-inventory.csv")
            .save_file()
        {
            let form = w.get_inventory_form();
            match repo.borrow().export_inventory_csv(
                &path,
                form.kind,
                &w.get_inventory_query(),
                i64::from(w.get_inventory_server_filter()),
                w.get_inventory_show_hidden(),
            ) {
                Ok(count) => {
                    crate::show_status(&w, &format!("Exported {count} records without credentials"))
                }
                Err(error) => message(&w, Err(error)),
            }
        }
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
    fn connection_actions_respect_protocol_pending_state_and_safe_hosts() {
        let mut details = Details {
            address: "203.0.113.10".into(),
            ports: "22".into(),
            protocol: "SSH".into(),
            ..Default::default()
        };
        assert_eq!(
            connection_target(&details).unwrap(),
            (false, "ssh -p 22 203.0.113.10".into())
        );
        details.protocol = "MySQL".into();
        details.ports = "3060".into();
        assert_eq!(
            connection_target(&details).unwrap(),
            (false, "203.0.113.10:3060".into())
        );
        details.protocol = "https".into();
        details.ports = "443".into();
        assert_eq!(
            connection_target(&details).unwrap(),
            (true, "https://203.0.113.10:443".into())
        );
        details.address = "2001:db8::1".into();
        assert_eq!(
            connection_target(&details).unwrap().1,
            "https://[2001:db8::1]:443"
        );
        details.pending_confirmation = true;
        assert!(connection_target(&details).is_err());
        details.pending_confirmation = false;
        for host in [
            "-oProxyCommand=bad",
            "example.com;bad",
            "user:password@example.com",
        ] {
            details.address = host.into();
            assert!(connection_target(&details).is_err());
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
    fn verification_resets_on_edit_and_csv_is_filtered_and_formula_safe() -> Result<(), StorageError>
    {
        let mut repository = Repository::in_memory()?;
        let mut details = service("production-api");
        details.tags = "production".into();
        details.notes = "=HYPERLINK(\"https://example.invalid\")".into();
        let id = repository.save_inventory(0, 0, details.clone(), &[], None)?;
        assert!(repository.inventory_entries()?[0].verification_due);
        repository.mark_inventory_verified(id)?;
        let checked = repository.inventory_entries()?.remove(0);
        assert!(!checked.verification_due);
        assert!(checked.verified_at.is_some());

        details.version = "2.0".into();
        repository.save_inventory(id, 0, details, &[], None)?;
        assert!(repository.inventory_entries()?[0].verification_due);
        let path = std::env::temp_dir().join(format!(
            "itgla-inventory-{}.csv",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert_eq!(
            repository.export_inventory_csv(&path, 0, "production", 0, false)?,
            1
        );
        let csv = fs::read_to_string(&path).unwrap();
        assert!(csv.contains("'=HYPERLINK"));
        assert!(csv.contains("production-api"));
        assert_eq!(
            repository.export_inventory_csv(&path, 0, "not-a-match", 0, false)?,
            0
        );
        let _ = fs::remove_file(path);
        Ok(())
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

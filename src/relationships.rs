//! Shared business/resource links and the safe, non-secret operation timeline.
use rusqlite::{OptionalExtension, params};

use crate::storage::{Repository, StorageError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BusinessRecord {
    pub id: i64,
    pub name: String,
    pub tags: String,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceLink {
    pub id: i64,
    pub source_type: String,
    pub source_id: i64,
    pub target_type: String,
    pub target_id: i64,
    pub relation_type: String,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEvent {
    pub action: String,
    pub summary: String,
    pub occurred_at: String,
}

fn invalid(message: &str) -> StorageError {
    StorageError::InvalidImport(message.into())
}

fn valid_type(value: &str) -> bool {
    matches!(value, "business" | "domain" | "service" | "server")
}

fn valid_relation(value: &str) -> bool {
    matches!(
        value,
        "owns"
            | "contains"
            | "resolves_to"
            | "hosted_on"
            | "origin_on"
            | "depends_on"
            | "replicates_to"
            | "fails_over_to"
            | "balances_to"
            | "related_to"
    )
}

fn validate_semantics(source: &str, target: &str, relation: &str) -> Result<(), StorageError> {
    let valid = match relation {
        "owns" => source == "business" && matches!(target, "domain" | "service" | "server"),
        "contains" => source == "domain" && target == "domain",
        "resolves_to" => source == "domain" && matches!(target, "server" | "service"),
        "hosted_on" => source == "service" && target == "server",
        "origin_on" => source == "domain" && target == "server",
        "depends_on" | "replicates_to" | "fails_over_to" | "balances_to" => {
            source == "service" && target == "service"
        }
        "related_to" => true,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(invalid(
            "This relationship does not apply to the selected resource types",
        ))
    }
}
fn read_link(tx: &rusqlite::Transaction<'_>, id: i64) -> Result<ResourceLink, StorageError> {
    tx.query_row("SELECT id,source_type,source_id,target_type,target_id,relation_type,notes FROM resource_links WHERE id=?1",[id],|r|Ok(ResourceLink{id:r.get(0)?,source_type:r.get(1)?,source_id:r.get(2)?,target_type:r.get(3)?,target_id:r.get(4)?,relation_type:r.get(5)?,notes:r.get(6)?})).optional()?.ok_or_else(||invalid("Relationship no longer exists"))
}
fn sync_link(
    tx: &rusqlite::Transaction<'_>,
    l: &ResourceLink,
    add: bool,
) -> Result<(), StorageError> {
    match l.relation_type.as_str() {
        "hosted_on" | "origin_on" => {
            if add {
                tx.execute(
                    "INSERT OR IGNORE INTO operational_hosts(inventory_id,server_id) VALUES(?1,?2)",
                    params![l.source_id, l.target_id],
                )?;
            } else {
                tx.execute(
                    "DELETE FROM operational_hosts WHERE inventory_id=?1 AND server_id=?2",
                    params![l.source_id, l.target_id],
                )?;
            }
        }
        "owns" if l.target_type != "server" => {
            if add {
                let name: String = tx.query_row(
                    "SELECT name FROM businesses WHERE id=?1",
                    [l.source_id],
                    |r| r.get(0),
                )?;
                tx.execute("DELETE FROM resource_links WHERE source_type='business' AND source_id<>?1 AND target_type=?2 AND target_id=?3 AND relation_type='owns'",params![l.source_id,l.target_type,l.target_id])?;
                tx.execute("UPDATE operational_inventory SET business_id=?1,data=json_set(data,'$.business',?2) WHERE id=?3",params![l.source_id,name,l.target_id])?;
            } else {
                tx.execute("UPDATE operational_inventory SET business_id=NULL,data=json_set(data,'$.business','') WHERE id=?1 AND business_id=?2",params![l.target_id,l.source_id])?;
            }
        }
        "contains" => {
            if add {
                let cycle:bool=tx.query_row("WITH RECURSIVE parents(id) AS (SELECT ?1 UNION SELECT parent_domain_id FROM operational_inventory JOIN parents ON operational_inventory.id=parents.id WHERE parent_domain_id IS NOT NULL) SELECT EXISTS(SELECT 1 FROM parents WHERE id=?2)",params![l.source_id,l.target_id],|r|r.get(0))?;
                if cycle {
                    return Err(invalid("A domain cannot contain its own ancestor"));
                }
                let name: String = tx.query_row(
                    "SELECT json_extract(data,'$.name') FROM operational_inventory WHERE id=?1",
                    [l.source_id],
                    |r| r.get(0),
                )?;
                tx.execute("DELETE FROM resource_links WHERE source_type='domain' AND source_id<>?1 AND target_type='domain' AND target_id=?2 AND relation_type='contains'",params![l.source_id,l.target_id])?;
                tx.execute("UPDATE operational_inventory SET parent_domain_id=?1,data=json_set(data,'$.parent',?2) WHERE id=?3",params![l.source_id,name,l.target_id])?;
            } else {
                tx.execute("UPDATE operational_inventory SET parent_domain_id=NULL,data=json_set(data,'$.parent','') WHERE id=?1 AND parent_domain_id=?2",params![l.target_id,l.source_id])?;
            }
        }
        "depends_on" | "replicates_to" | "fails_over_to" | "balances_to" => {
            let kind = match l.relation_type.as_str() {
                "depends_on" => 0,
                "replicates_to" => 1,
                "fails_over_to" => 2,
                _ => 3,
            };
            if add {
                tx.execute("INSERT INTO operational_relations(source_id,target_id,kind,notes) VALUES(?1,?2,?3,?4) ON CONFLICT(source_id,target_id,kind) DO UPDATE SET notes=excluded.notes",params![l.source_id,l.target_id,kind,l.notes])?;
            } else {
                tx.execute("DELETE FROM operational_relations WHERE source_id=?1 AND target_id=?2 AND kind=?3",params![l.source_id,l.target_id,kind])?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl Repository {
    pub fn businesses(&self) -> Result<Vec<BusinessRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id,name,tags,notes FROM businesses WHERE archived_at IS NULL ORDER BY lower(name),id",
        )?;
        statement
            .query_map([], |row| {
                Ok(BusinessRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    tags: row.get(2)?,
                    notes: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn save_business(
        &mut self,
        id: i64,
        name: &str,
        tags: &str,
        notes: &str,
    ) -> Result<i64, StorageError> {
        let name = name.trim();
        let tags = tags.trim();
        let notes = notes.trim();
        if name.is_empty() || name.chars().count() > 160 {
            return Err(invalid("Business name must contain 1 to 160 characters"));
        }
        if tags.chars().count() > 2048 || notes.chars().count() > 2048 {
            return Err(invalid(
                "Business tags and notes are limited to 2048 characters",
            ));
        }
        let tx = self.connection.transaction()?;
        let saved = if id == 0 {
            tx.execute(
                "INSERT INTO businesses(name,tags,notes) VALUES(?1,?2,?3)",
                params![name, tags, notes],
            )?;
            tx.last_insert_rowid()
        } else {
            if tx.execute(
                "UPDATE businesses SET name=?1,tags=?2,notes=?3,updated_at=CURRENT_TIMESTAMP WHERE id=?4 AND archived_at IS NULL",
                params![name,tags,notes,id],
            )? != 1 {
                return Err(invalid("Business is hidden or no longer exists"));
            }
            id
        };
        tx.execute("UPDATE operational_inventory SET data=json_set(data,'$.business',?1) WHERE business_id=?2",params![name,saved])?;
        tx.commit()?;
        Ok(saved)
    }

    pub fn resource_links(&self) -> Result<Vec<ResourceLink>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id,source_type,source_id,target_type,target_id,relation_type,notes FROM resource_links ORDER BY id",
        )?;
        statement
            .query_map([], |row| {
                Ok(ResourceLink {
                    id: row.get(0)?,
                    source_type: row.get(1)?,
                    source_id: row.get(2)?,
                    target_type: row.get(3)?,
                    target_id: row.get(4)?,
                    relation_type: row.get(5)?,
                    notes: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    // One explicit directed edge: identity, endpoints, type and annotation.
    #[allow(clippy::too_many_arguments)]
    pub fn save_resource_link(
        &mut self,
        id: i64,
        source_type: &str,
        source_id: i64,
        target_type: &str,
        target_id: i64,
        relation_type: &str,
        notes: &str,
    ) -> Result<i64, StorageError> {
        if !valid_type(source_type)
            || !valid_type(target_type)
            || !valid_relation(relation_type)
            || source_id <= 0
            || target_id <= 0
            || (source_type == target_type && source_id == target_id)
            || notes.chars().count() > 2048
        {
            return Err(invalid(
                "Choose two different resources and a valid relationship",
            ));
        }
        validate_semantics(source_type, target_type, relation_type)?;
        let tx = self.connection.transaction()?;
        for (kind, resource_id) in [(source_type, source_id), (target_type, target_id)] {
            let exists: bool = match kind {
                "business" => tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM businesses WHERE id=?1 AND archived_at IS NULL)",
                    [resource_id],
                    |row| row.get(0),
                )?,
                "domain" => tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM operational_inventory WHERE id=?1 AND kind=1 AND hidden=0)",
                    [resource_id],
                    |row| row.get(0),
                )?,
                "service" => tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM operational_inventory WHERE id=?1 AND kind=0 AND hidden=0)",
                    [resource_id],
                    |row| row.get(0),
                )?,
                "server" => tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM server_records WHERE id=?1 AND archived_at IS NULL)",
                    [resource_id],
                    |row| row.get(0),
                )?,
                _ => false,
            };
            if !exists {
                return Err(invalid("Both resources must be active before linking them"));
            }
        }
        if id != 0 {
            let old = read_link(&tx, id)?;
            if old.source_type != source_type
                || old.source_id != source_id
                || old.target_type != target_type
                || old.target_id != target_id
                || old.relation_type != relation_type
            {
                sync_link(&tx, &old, false)?;
            }
        }
        let saved = if id == 0 {
            tx.execute(
                "INSERT INTO resource_links(source_type,source_id,target_type,target_id,relation_type,notes) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(source_type,source_id,target_type,target_id,relation_type) DO UPDATE SET notes=excluded.notes,updated_at=CURRENT_TIMESTAMP",
                params![source_type,source_id,target_type,target_id,relation_type,notes.trim()],
            )?;
            tx.query_row("SELECT id FROM resource_links WHERE source_type=?1 AND source_id=?2 AND target_type=?3 AND target_id=?4 AND relation_type=?5",params![source_type,source_id,target_type,target_id,relation_type],|r|r.get(0))?
        } else {
            // Removing a mirrored legacy relation can also remove its common link.
            // Upsert by the original ID to preserve selection across that trigger.
            tx.execute(
                "INSERT INTO resource_links(source_type,source_id,target_type,target_id,relation_type,notes,id) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET source_type=excluded.source_type,source_id=excluded.source_id,target_type=excluded.target_type,target_id=excluded.target_id,relation_type=excluded.relation_type,notes=excluded.notes,updated_at=CURRENT_TIMESTAMP",
                params![source_type,source_id,target_type,target_id,relation_type,notes.trim(),id],
            )?;
            id
        };
        sync_link(
            &tx,
            &ResourceLink {
                id: saved,
                source_type: source_type.into(),
                source_id,
                target_type: target_type.into(),
                target_id,
                relation_type: relation_type.into(),
                notes: notes.trim().into(),
            },
            true,
        )?;
        tx.commit()?;
        Ok(saved)
    }

    pub fn delete_resource_link(&mut self, id: i64) -> Result<(), StorageError> {
        let tx = self.connection.transaction()?;
        let link = read_link(&tx, id)?;
        sync_link(&tx, &link, false)?;
        tx.execute("DELETE FROM resource_links WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn resource_history(
        &self,
        resource_type: &str,
        resource_id: i64,
    ) -> Result<Vec<HistoryEvent>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT action,summary,occurred_at FROM resource_history WHERE resource_type=?1 AND resource_id=?2 ORDER BY occurred_at DESC,id DESC LIMIT 100",
        )?;
        statement
            .query_map(params![resource_type, resource_id], |row| {
                Ok(HistoryEvent {
                    action: row.get(0)?,
                    summary: row.get(1)?,
                    occurred_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operational::Details;

    #[test]
    fn common_links_validate_and_history_redacts_credentials() -> Result<(), StorageError> {
        let mut repo = Repository::in_memory()?;
        let server = repo.save_server(
            None,
            &crate::domain::ServerDraft {
                name: "app-01".into(),
                tags: vec!["prod".into()],
                ip_address: "192.0.2.10".into(),
                ports: "22".into(),
                environment: "production".into(),
                status: "active".into(),
                purpose: "app".into(),
            },
        )?;
        let service = repo.save_inventory(
            0,
            0,
            Details {
                name: "SSH".into(),
                ports: "22".into(),
                protocol: "SSH".into(),
                ..Default::default()
            },
            &[server],
            None,
        )?;
        assert!(
            repo.save_resource_link(0, "service", service, "service", service, "depends_on", "")
                .is_err()
        );
        let links = repo.resource_links()?;
        assert!(links.iter().any(|link| link.source_id == service
            && link.target_id == server
            && link.relation_type == "hosted_on"));
        let history = repo.resource_history("service", service)?;
        assert!(history.iter().any(|event| event.action == "created"));
        assert!(
            history
                .iter()
                .all(|event| !event.summary.contains("password"))
        );
        Ok(())
    }
}

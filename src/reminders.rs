//! Local attention projection. Domain reviews never claim a renewal succeeded.
use crate::{
    AppWindow, WorkspaceItem,
    storage::{Repository, StorageError},
    workspace::AssetSummary,
};
use rusqlite::params;
use slint::{ModelRc, VecModel};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Reminder {
    pub key: String,
    pub title: String,
    pub description: String,
    pub state: String,
    pub due: String,
}

/// Calculate against a caller-supplied local calendar date, including leap years.
fn day_number(date: &str) -> Option<i64> {
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
    {
        return None;
    }
    let year = date[..4].parse::<i64>().ok()?;
    let month = date[5..7].parse::<usize>().ok()?;
    let day = date[8..].parse::<i64>().ok()?;
    if year < 1 || !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day < 1 || day > months[month - 1] {
        return None;
    }
    let previous = year - 1;
    Some(
        previous * 365 + previous / 4 - previous / 100
            + previous / 400
            + months[..month - 1].iter().sum::<i64>()
            + day,
    )
}

pub fn items(repo: &Repository, all: &[AssetSummary]) -> Result<Vec<Reminder>, StorageError> {
    let today: String = repo
        .connection
        .query_row("SELECT date('now','localtime')", [], |r| r.get(0))?;
    items_at(repo, all, &today)
}

/// One batched state read; no query per asset. Domain dates are not certificate dates.
fn items_at(
    repo: &Repository,
    all: &[AssetSummary],
    today: &str,
) -> Result<Vec<Reminder>, StorageError> {
    let today_number = day_number(today)
        .ok_or_else(|| StorageError::InvalidImport("Invalid local calendar date".into()))?;
    let mut statement = repo.connection.prepare("SELECT resource_id,source_date,status,snoozed_until FROM reminder_reviews WHERE resource_type='domain'")?;
    let saved_states = statement
        .query_map([], |r| {
            Ok((
                (r.get::<_, i64>(0)?, r.get::<_, String>(1)?),
                (r.get::<_, String>(2)?, r.get::<_, String>(3)?),
            ))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;
    let mut out = Vec::new();
    for a in all
        .iter()
        .filter(|a| a.resource.kind == "domain" && !a.expires.is_empty())
    {
        let Some(expiry_number) = day_number(&a.expires) else {
            continue;
        };
        let days = expiry_number - today_number;
        if days > 30 {
            continue;
        }
        let saved = saved_states.get(&(a.resource.id, a.expires.clone()));
        let state = match &saved {
            Some((s, _)) if s == "reviewed" => "Reviewed",
            Some((_, date)) if date.as_str() > today => "Postponed",
            _ => "Open",
        };
        out.push(Reminder {
            key: format!("domain:{}:{}", a.resource.id, a.expires),
            title: format!(
                "{} · {}",
                a.name,
                if days < 0 {
                    format!("recorded expiry {} days ago", -days)
                } else if days == 0 {
                    "expires today".into()
                } else {
                    format!("expires in {days} days")
                }
            ),
            description: format!(
                "Due {} · Contact: {} · {}{}",
                a.expires,
                if a.owner.is_empty() {
                    "Not recorded"
                } else {
                    &a.owner
                },
                state,
                if state == "Postponed" {
                    format!(
                        " until {}",
                        saved.as_ref().map(|s| s.1.as_str()).unwrap_or_default()
                    )
                } else {
                    String::new()
                }
            ),
            state: state.into(),
            due: a.expires.clone(),
        });
    }
    for work in repo.work_items()? {
        if matches!(
            work.status,
            crate::domain::WorkStatus::Done | crate::domain::WorkStatus::Cancelled
        ) {
            continue;
        }
        if day_number(&work.due_at).is_some_and(|due| due <= today_number + 30) {
            out.push(Reminder {
                key: format!("work:{}", work.id),
                title: work.title,
                description: format!(
                    "Work · Due {} · {} · Open Work to review its checklist",
                    work.due_at, work.owner
                ),
                state: "Open".into(),
                due: work.due_at,
            });
        }
    }
    out.sort_by(|a, b| a.due.cmp(&b.due).then_with(|| a.key.cmp(&b.key)));
    Ok(out)
}

pub fn refresh(w: &AppWindow, r: &Repository, all: &[AssetSummary]) -> Result<(), StorageError> {
    let all = items(r, all)?;
    w.set_workspace_attention_count(all.iter().filter(|a| a.state == "Open").count() as i32);
    // Overview always shows actionable items, independent of the reminder archive filter.
    let mode = if w.get_active_tab() == 8 {
        0
    } else {
        w.get_workspace_reminder_filter()
    };
    w.set_workspace_reminders(ModelRc::new(VecModel::from(
        all.iter()
            .filter(|a| {
                if mode == 0 {
                    a.state == "Open"
                } else if mode == 1 {
                    a.state == "Reviewed"
                } else {
                    a.state == "Postponed"
                }
            })
            .map(|a| WorkspaceItem {
                key: a.key.clone().into(),
                title: a.title.clone().into(),
                subtitle: a.description.clone().into(),
            })
            .collect::<Vec<_>>(),
    )));
    Ok(())
}

pub fn act(r: &mut Repository, key: &str, action: i32) -> Result<(), StorageError> {
    let today: String = r
        .connection
        .query_row("SELECT date('now','localtime')", [], |r| r.get(0))?;
    act_at(r, key, action, &today)
}
fn act_at(r: &mut Repository, key: &str, action: i32, today: &str) -> Result<(), StorageError> {
    if day_number(today).is_none() {
        return Err(StorageError::InvalidImport(
            "Invalid local calendar date".into(),
        ));
    }
    let parts = key.split(':').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "domain" || !matches!(action, 0..=2) {
        return Err(StorageError::InvalidImport(
            "Open the original Work to change its status.".into(),
        ));
    }
    let id = parts[1]
        .parse::<i64>()
        .map_err(|_| StorageError::InvalidImport("Invalid reminder resource".into()))?;
    let current = r
        .inventory_entries()?
        .into_iter()
        .find(|e| e.id == id && e.kind == 1 && !e.hidden && e.details.expires == parts[2])
        .ok_or_else(|| {
            StorageError::InvalidImport(
                "This expiry changed or the domain is unavailable. Refresh reminders.".into(),
            )
        })?;
    let tx = r.connection.transaction()?;
    if action == 2 {
        tx.execute("DELETE FROM reminder_reviews WHERE resource_type='domain' AND resource_id=?1 AND source_date=?2",params![id,current.details.expires])?;
    } else {
        tx.execute("INSERT INTO reminder_reviews(resource_type,resource_id,source_date,status,snoozed_until,reviewed_at) VALUES('domain',?1,?2,?3,CASE WHEN ?3='postponed' THEN date(?4,'+1 day') ELSE '' END,CASE WHEN ?3='reviewed' THEN CURRENT_TIMESTAMP ELSE NULL END) ON CONFLICT(resource_type,resource_id,source_date) DO UPDATE SET status=excluded.status,snoozed_until=excluded.snoozed_until,reviewed_at=excluded.reviewed_at,updated_at=CURRENT_TIMESTAMP",params![id,current.details.expires,if action==0{"reviewed"}else{"postponed"},today])?;
    }
    tx.execute(
        "INSERT INTO reminder_activity(resource_id,source_date,action) VALUES(?1,?2,?3)",
        params![
            id,
            current.details.expires,
            ["reviewed", "postponed", "reopened"][action as usize]
        ],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cycle_changes_reopen_and_hidden_assets_leave_attention() -> Result<(), StorageError> {
        let mut r = Repository::in_memory()?;
        let date: String =
            r.connection
                .query_row("SELECT date('now','localtime','+12 days')", [], |row| {
                    row.get(0)
                })?;
        let id = r.save_inventory(
            0,
            1,
            crate::operational::Details {
                name: "example.com".into(),
                expires: date.clone(),
                ..Default::default()
            },
            &[],
            None,
        )?;
        let key = format!("domain:{id}:{date}");
        assert!(
            items(&r, &crate::workspace::assets(&r)?)?
                .iter()
                .any(|i| i.key == key && i.state == "Open")
        );
        act(&mut r, &key, 0)?;
        assert!(
            items(&r, &crate::workspace::assets(&r)?)?
                .iter()
                .any(|i| i.key == key && i.state == "Reviewed")
        );
        act(&mut r, &key, 1)?;
        assert!(
            items(&r, &crate::workspace::assets(&r)?)?
                .iter()
                .any(|i| i.key == key && i.state == "Postponed")
        );
        act(&mut r, &key, 2)?;
        assert!(
            items(&r, &crate::workspace::assets(&r)?)?
                .iter()
                .any(|i| i.key == key && i.state == "Open")
        );
        r.set_inventory_hidden(id, true)?;
        assert!(
            !items(&r, &crate::workspace::assets(&r)?)?
                .iter()
                .any(|i| i.key == key)
        );
        assert!(act(&mut r, &key, 0).is_err());
        Ok(())
    }
    #[test]
    fn calendar_boundaries_and_new_expiry_cycles() -> Result<(), StorageError> {
        assert!(day_number("2025-02-29").is_none());
        assert!(day_number("2026-13-01").is_none());
        assert!(day_number("2026-1-01").is_none());
        for (a, b) in [
            ("2024-02-28", "2024-02-29"),
            ("2024-02-29", "2024-03-01"),
            ("2026-12-31", "2027-01-01"),
            ("2026-03-08", "2026-03-09"),
        ] {
            assert_eq!(day_number(b).unwrap() - day_number(a).unwrap(), 1);
        }
        let mut r = Repository::in_memory()?;
        let id = r.save_inventory(
            0,
            1,
            crate::operational::Details {
                name: "cycle.example.test".into(),
                expires: "2026-12-31".into(),
                ..Default::default()
            },
            &[],
            None,
        )?;
        let key = format!("domain:{id}:2026-12-31");
        act_at(&mut r, &key, 1, "2026-12-31")?;
        assert_eq!(
            items_at(&r, &crate::workspace::assets(&r)?, "2026-12-31")?[0].state,
            "Postponed"
        );
        assert_eq!(
            items_at(&r, &crate::workspace::assets(&r)?, "2027-01-01")?[0].state,
            "Open"
        );
        act_at(&mut r, &key, 0, "2026-12-31")?;
        let mut details = r
            .inventory_entries()?
            .into_iter()
            .find(|e| e.id == id)
            .unwrap()
            .details;
        assert_eq!(details.expires, "2026-12-31");
        details.expires = "2027-01-02".into();
        r.save_inventory(id, 1, details, &[], None)?;
        let next = items_at(&r, &crate::workspace::assets(&r)?, "2026-12-31")?;
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].state, "Open");
        assert!(next[0].key.ends_with("2027-01-02"));
        assert!(act_at(&mut r, &key, 0, "2026-12-31").is_err());
        assert!(act_at(&mut r, "work:1", 0, "2026-12-31").is_err());
        r.set_inventory_hidden(id, true)?;
        assert!(items_at(&r, &crate::workspace::assets(&r)?, "2026-12-31")?.is_empty());
        r.set_inventory_hidden(id, false)?;
        assert_eq!(
            items_at(&r, &crate::workspace::assets(&r)?, "2026-12-31")?[0].state,
            "Open"
        );
        Ok(())
    }

    #[test]
    fn due_order_boundary_and_failure_are_atomic() -> Result<(), StorageError> {
        let mut r = Repository::in_memory()?;
        for (name, date) in [
            ("late.example.test", "2026-12-30"),
            ("edge.example.test", "2027-01-30"),
            ("future.example.test", "2027-01-31"),
            ("unknown.example.test", ""),
        ] {
            r.save_inventory(
                0,
                1,
                crate::operational::Details {
                    name: name.into(),
                    expires: date.into(),
                    ..Default::default()
                },
                &[],
                None,
            )?;
        }
        let all = crate::workspace::assets(&r)?;
        let reminders = items_at(&r, &all, "2026-12-31")?;
        assert_eq!(reminders.len(), 2);
        assert_eq!(reminders[0].due, "2026-12-30");
        r.connection.execute_batch("CREATE TRIGGER fail_reminder BEFORE INSERT ON reminder_activity BEGIN SELECT RAISE(ABORT,'test rollback'); END;")?;
        assert!(act_at(&mut r, &reminders[0].key, 0, "2026-12-31").is_err());
        let count: i64 =
            r.connection
                .query_row("SELECT count(*) FROM reminder_reviews", [], |r| r.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[test]
    fn schema21_upgrade_backup_restart_and_restore() -> Result<(), StorageError> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "itgla-workspace-migration-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("temporary directory");
        let path = directory.join("test.db");
        let r = Repository::open(path.clone())?;
        // Remove only the additive schema22 objects to create a real schema21 fixture.
        r.connection.execute_batch("DROP TRIGGER reminder_inventory_deleted; DROP TABLE reminder_reviews; DROP TABLE reminder_activity; PRAGMA user_version=21;")?;
        drop(r);
        let mut r = Repository::open(path.clone())?;
        let backup = directory.join("backups/pre-schema-22.db");
        let old = rusqlite::Connection::open(&backup)?;
        assert_eq!(
            old.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?,
            21
        );
        assert_eq!(
            old.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))?,
            "ok"
        );
        drop(old);
        let id = r.save_inventory(
            0,
            1,
            crate::operational::Details {
                name: "persistent.example.test".into(),
                expires: "2026-12-31".into(),
                ..Default::default()
            },
            &[],
            None,
        )?;
        let key = format!("domain:{id}:2026-12-31");
        act_at(&mut r, &key, 0, "2026-12-01")?;
        let full = directory.join("full.db");
        r.create_backup(&full)?;
        drop(r);
        let mut r = Repository::open(path)?;
        assert_eq!(
            items_at(&r, &crate::workspace::assets(&r)?, "2026-12-01")?[0].state,
            "Reviewed"
        );
        act_at(&mut r, &key, 2, "2026-12-01")?;
        r.restore_backup(&full)?;
        assert_eq!(
            items_at(&r, &crate::workspace::assets(&r)?, "2026-12-01")?[0].state,
            "Reviewed"
        );
        r.connection
            .execute("DELETE FROM operational_inventory WHERE id=?1", [id])?;
        assert_eq!(
            r.connection
                .query_row("SELECT count(*) FROM reminder_activity", [], |r| r
                    .get::<_, i64>(0))?,
            0
        );
        drop(r);
        std::fs::remove_dir_all(directory).expect("clean isolated fixture");
        Ok(())
    }
    #[test]
    fn failed_upgrade_keeps_schema21_and_preupgrade_backup() -> Result<(), StorageError> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "itgla-workspace-failed-migration-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("temporary directory");
        let path = directory.join("test.db");
        let r = Repository::open(path.clone())?;
        r.connection.execute_batch("DROP TRIGGER reminder_inventory_deleted; DROP TABLE reminder_reviews; DROP TABLE reminder_activity; CREATE TABLE reminder_activity_resource_idx(collision TEXT); PRAGMA user_version=21;")?;
        drop(r);
        assert!(Repository::open(path.clone()).is_err());
        let db = rusqlite::Connection::open(path)?;
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?,
            21
        );
        assert_eq!(db.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('reminder_reviews','reminder_activity')",[],|r|r.get::<_,i64>(0))?,0);
        assert!(directory.join("backups/pre-schema-22.db").exists());
        drop(db);
        std::fs::remove_dir_all(directory).expect("clean isolated fixture");
        Ok(())
    }
}

//! Free local ownership workflows. No network calls, billing or telemetry.
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use slint::ComponentHandle;

use crate::{
    AppWindow,
    storage::{Repository, StorageError},
};

/// Export an allowlisted plain-text inventory, never decrypted secrets or free-form notes.
/// Metadata can itself be sensitive: the user must review this before sharing.
pub fn handover_summary(repo: &Repository) -> Result<String, StorageError> {
    let all = crate::workspace::assets(repo)?;
    let names = all.iter().map(|a| (a.key(), a)).collect::<HashMap<_, _>>();
    let entries = repo
        .inventory_entries()?
        .into_iter()
        .map(|e| (e.id, e))
        .collect::<HashMap<_, _>>();
    let mut out = String::from(
        "ITGLA handover summary\n\nManually recorded metadata, not a live health check or full backup.\nReview before sharing: names, addresses and contacts may be sensitive.\nCredentials, custom fields, documentation links and free-form notes are excluded.\nHidden records are excluded. Pending services still require confirmation.\n\nASSETS\n",
    );
    // Keep each user-provided value on one line, even in a plain-text report.
    let line = |s: &str| {
        s.chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect::<String>()
    };
    for a in &all {
        out.push_str(&format!("\n{} [{}]\n  Address / host: {}\n  Environment: {}\n  Contact: {}\n  Last checked: {}\n", line(&a.name), a.key(), line(&a.address), line(&a.environment), line(&a.owner), line(&a.checked)));
        if a.resource.kind == "server" {
            out.push_str(&format!("  Purpose: {}\n", line(&a.purpose)));
        }
        if let Some(e) = entries.get(&a.resource.id).filter(|e| {
            (e.kind == 0 && a.resource.kind == "service")
                || (e.kind == 1 && a.resource.kind == "domain")
        }) {
            if e.kind == 0 {
                out.push_str(&format!(
                    "  Version: {}\n  Confirmation: {}\n",
                    line(&e.details.version),
                    if e.details.pending_confirmation {
                        "Pending"
                    } else {
                        "Confirmed"
                    }
                ));
            } else {
                out.push_str(&format!(
                    "  Expiry: {}\n  Registrar: {}\n",
                    line(&e.details.expires),
                    line(&e.details.registrar)
                ));
            }
        }
    }
    out.push_str("\nRELATIONSHIPS\n");
    for l in repo.resource_links()? {
        let source = format!("{}:{}", l.source_type, l.source_id);
        let target = format!("{}:{}", l.target_type, l.target_id);
        if let (Some(s), Some(t)) = (names.get(&source), names.get(&target)) {
            out.push_str(&format!(
                "{} [{}] -- {} -- {} [{}]\n",
                line(&s.name),
                source,
                l.relation_type.replace('_', " "),
                line(&t.name),
                target
            ));
        }
    }
    Ok(out)
}

pub fn setup(window: &AppWindow, repo: Rc<RefCell<Repository>>) {
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_save_full_backup(move || {
        let Some(w) = weak.upgrade() else { return };
        if w.get_demo_mode() { return; }
        if let Some(path) = rfd::FileDialog::new().set_title("Save full database backup (contains sensitive data)").add_filter("SQLite backup", &["db"]).set_file_name("itgla-full-backup.db").save_file() {
            match r.borrow().create_backup(&path) {
                Ok(()) => crate::show_status(&w, &format!("Full backup saved: {}. Store it securely; it is not an encrypted container.", path.display())),
                Err(e) => crate::show_error(&w, &e),
            }
        }
    });
    let weak = window.as_weak();
    window.on_choose_restore(move || {
        let Some(w) = weak.upgrade() else { return };
        if w.get_demo_mode() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Choose full database backup")
            .add_filter("SQLite backup", &["db"])
            .pick_file()
        {
            w.set_restore_source(path.to_string_lossy().as_ref().into());
            w.set_restore_confirm_open(true);
        }
    });
    let weak = window.as_weak();
    let r = repo.clone();
    window.on_confirm_restore(move || {
        let Some(w) = weak.upgrade() else { return };
        if w.get_demo_mode() || !w.get_restore_confirm_open() || w.get_restore_source().is_empty() {
            return;
        }
        let source = std::path::PathBuf::from(w.get_restore_source().as_str());
        match r.borrow_mut().restore_backup(&source) {
            Ok(()) => {
                // Closing prevents stale editors/models from writing into the restored workspace.
                if let Err(e) = slint::quit_event_loop() {
                    crate::show_error(&w, &e);
                }
            }
            Err(e) => {
                w.set_restore_confirm_open(false);
                w.set_restore_source("".into());
                crate::show_error(&w, &e);
            }
        }
    });
    let weak = window.as_weak();
    window.on_export_handover(move || {
        let Some(w) = weak.upgrade() else { return };
        if let Some(path) = rfd::FileDialog::new().set_title("Export metadata; review before sharing").add_filter("Plain text", &["txt"]).set_file_name("itgla-handover.txt").save_file() {
            let result = handover_summary(&repo.borrow()).and_then(|text| {
                use std::io::Write;
                // A new file only: never overwrite a database or an existing backup.
                std::fs::OpenOptions::new().write(true).create_new(true).open(&path)
                    .and_then(|mut file| file.write_all(text.as_bytes()))
                    .map_err(|e| StorageError::InvalidImport(format!("Could not save handover summary. Choose a new filename: {e}")))
            });
            match result {
                Ok(()) => crate::show_status(&w, "Handover summary saved. Review names, addresses and contacts before sharing. This is not a full backup."),
                Err(e) => crate::show_error(&w, &e),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_import_resets_stale_mapping_and_restore_cancel_does_not_write()
    -> Result<(), StorageError> {
        use i_slint_backend_testing::ElementHandle;
        i_slint_backend_testing::init_no_event_loop();
        let repo = Rc::new(RefCell::new(Repository::in_memory()?));
        let w = AppWindow::new().expect("test window");
        crate::workspace::setup(&w, repo.clone());
        setup(&w, repo.clone());
        w.set_import_ready(true);
        w.set_importing_sites(true);
        w.set_import_error("stale error".into());
        w.invoke_workspace_new(8);
        assert!(w.get_import_dialog_open());
        assert!(!w.get_import_ready());
        assert!(!w.get_importing_sites());
        assert!(w.get_import_error().is_empty());
        w.set_import_dialog_open(false);
        w.set_onboarding_open(false);
        w.set_active_tab(8);
        w.window().set_size(slint::PhysicalSize::new(1100, 720));
        w.show().expect("show window");
        let weak = w.as_weak();
        w.on_tab_selected(move |tab| weak.upgrade().expect("window").set_active_tab(tab));
        let before: i64 =
            repo.borrow()
                .connection
                .query_row("SELECT total_changes()", [], |r| r.get(0))?;
        w.set_restore_source("not-an-existing-backup.db".into());
        w.set_restore_confirm_open(true);
        let settings = ElementHandle::find_by_accessible_label(&w, "Settings")
            .next()
            .expect("settings");
        settings.invoke_accessible_default_action();
        assert_eq!(w.get_active_tab(), 8, "restore owns the input surface");
        w.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed {
                text: slint::platform::Key::Escape.into(),
            });
        assert!(!w.get_restore_confirm_open());
        assert!(w.get_restore_source().is_empty());
        w.invoke_confirm_restore();
        let after: i64 = repo
            .borrow()
            .connection
            .query_row("SELECT total_changes()", [], |r| r.get(0))?;
        assert_eq!(before, after, "cancelling cannot mutate the database");
        Ok(())
    }

    #[test]
    fn handover_omits_notes_secrets_hidden_records_and_links_to_them() -> Result<(), StorageError> {
        let mut r = Repository::in_memory()?;
        crate::topology::reset_demo(&mut r)?;
        r.connection.execute("UPDATE operational_inventory SET data=json_set(data,'$.notes','private-note-sentinel')", [])?;
        r.connection.execute(
            "UPDATE businesses SET notes='private-business-sentinel'",
            [],
        )?;
        #[cfg(windows)]
        {
            let e = r
                .inventory_entries()?
                .into_iter()
                .find(|e| e.kind == 0)
                .expect("demo service");
            let protected = crate::secrets::protect(
                br#"["private-user-sentinel","private-password-sentinel"]"#,
            )
            .map_err(|e| StorageError::InvalidImport(e.into()))?;
            r.save_inventory(e.id, e.kind, e.details, &e.hosts, Some(Some(protected)))?;
        }
        let text = handover_summary(&r)?;
        assert!(text.contains("PostgreSQL primary"));
        assert!(text.contains("5432"));
        assert!(text.contains("RELATIONSHIPS"));
        assert!(!text.contains("private-note-sentinel"));
        assert!(!text.contains("private-business-sentinel"));
        assert!(!text.contains("private-user-sentinel"));
        assert!(!text.contains("private-password-sentinel"));
        let entry = r
            .inventory_entries()?
            .into_iter()
            .find(|e| e.details.name == "PostgreSQL primary")
            .expect("demo service");
        r.set_inventory_hidden(entry.id, true)?;
        assert!(!handover_summary(&r)?.contains("PostgreSQL primary"));
        Ok(())
    }
}

//! Structured access points; the legacy ports field remains a numeric projection.
use std::{cell::RefCell, rc::Rc};

use rusqlite::params;
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{
    AppWindow, EndpointRow, TableState, refresh,
    storage::{Repository, StorageError},
};

#[derive(Debug)]
pub struct Endpoint {
    pub id: i64,
    pub port: u16,
    pub transport: String,
    pub protocol: String,
    pub purpose: String,
}

fn invalid(message: &str) -> StorageError {
    StorageError::InvalidImport(message.into())
}

impl Repository {
    pub fn endpoints(&self, server: i64) -> Result<Vec<Endpoint>, StorageError> {
        let mut query = self.connection.prepare(
            "SELECT id,port,transport,protocol,purpose FROM server_endpoints WHERE server_id=?1 ORDER BY port,transport,id")?;
        Ok(query
            .query_map([server], |r| {
                Ok(Endpoint {
                    id: r.get(0)?,
                    port: r.get(1)?,
                    transport: r.get(2)?,
                    protocol: r.get(3)?,
                    purpose: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn save_endpoint(
        &mut self,
        server: i64,
        id: i64,
        port: &str,
        transport: &str,
        protocol: &str,
        purpose: &str,
    ) -> Result<(), StorageError> {
        let port = port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .ok_or_else(|| invalid("Port must be a number from 1 to 65535."))?;
        if !["Unspecified", "TCP", "UDP"].contains(&transport)
            || protocol.chars().count() > 40
            || purpose.chars().count() > 240
            || protocol.chars().any(char::is_control)
            || purpose.chars().any(char::is_control)
        {
            return Err(invalid(
                "Choose a transport; protocol is limited to 40 characters and purpose to 240.",
            ));
        }
        let tx = self.connection.transaction()?;
        let duplicate: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM server_endpoints WHERE server_id=?1 AND port=?2 AND transport=?3 AND id<>?4)",
            params![server,port,transport,id], |r|r.get(0))?;
        if duplicate {
            return Err(invalid(
                "This port and transport already exist. Edit that entry or choose another transport.",
            ));
        }
        if id == 0 {
            if tx.query_row("SELECT count(*) FROM server_endpoints WHERE server_id=?1",[server],|r|r.get::<_,i64>(0))? >= 32 {
                return Err(invalid("Use at most 32 endpoints per server."));
            }
            tx.execute("INSERT INTO server_endpoints(server_id,port,transport,protocol,purpose) VALUES(?1,?2,?3,?4,?5)",params![server,port,transport,protocol.trim(),purpose.trim()])?;
        } else if tx.execute("UPDATE server_endpoints SET port=?1,transport=?2,protocol=?3,purpose=?4 WHERE id=?5 AND server_id=?6",params![port,transport,protocol.trim(),purpose.trim(),id,server])? == 0 {
            return Err(invalid("Endpoint no longer exists. Reopen Ports and try again."));
        }
        project_ports(&tx, server)?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_endpoint(&mut self, server: i64, id: i64) -> Result<(), StorageError> {
        let tx = self.connection.transaction()?;
        if tx.query_row(
            "SELECT count(*) FROM server_endpoints WHERE server_id=?1",
            [server],
            |r| r.get::<_, i64>(0),
        )? <= 1
        {
            return Err(invalid("Keep at least one port for this server."));
        }
        if tx.execute(
            "DELETE FROM server_endpoints WHERE id=?1 AND server_id=?2",
            params![id, server],
        )? == 0
        {
            return Err(invalid("Endpoint no longer exists."));
        }
        project_ports(&tx, server)?;
        tx.commit()?;
        Ok(())
    }
}

fn project_ports(tx: &rusqlite::Transaction<'_>, server: i64) -> Result<(), StorageError> {
    tx.execute("UPDATE server_records SET ports=(SELECT group_concat(port, ', ') FROM (SELECT DISTINCT port FROM server_endpoints WHERE server_id=?1 ORDER BY port)),updated_at=CURRENT_TIMESTAMP WHERE id=?1",[server])?;
    Ok(())
}

fn address(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub fn to_row(host: &str, endpoint: Endpoint) -> EndpointRow {
    let address = address(host, endpoint.port);
    let protocol = endpoint.protocol.trim().to_ascii_uppercase();
    let connect_command = if protocol == "SSH" {
        format!("ssh -p {} {}", endpoint.port, host)
    } else {
        address.clone()
    };
    EndpointRow {
        id: i32::try_from(endpoint.id).unwrap_or(i32::MAX),
        port: endpoint.port.to_string().into(),
        transport: endpoint.transport.into(),
        protocol: protocol.into(),
        purpose: endpoint.purpose.into(),
        address: address.into(),
        connect_command: connect_command.into(),
    }
}

fn update(w: &AppWindow, repo: &Repository) -> Result<(), StorageError> {
    let server = i64::from(w.get_endpoint_server());
    let (name, host): (String, String) = repo.connection.query_row(
        "SELECT name,ip_address FROM server_records WHERE id=?1",
        [server],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    w.set_endpoint_title(format!("Ports - {name}").into());
    if w.get_editing_id() == w.get_endpoint_server() {
        let ports: String = repo.connection.query_row(
            "SELECT ports FROM server_records WHERE id=?1",
            [server],
            |r| r.get(0),
        )?;
        w.set_editing_ports(ports.into());
    }
    let rows = repo
        .endpoints(server)?
        .into_iter()
        .map(|e| to_row(&host, e))
        .collect::<Vec<_>>();
    w.set_endpoint_rows(ModelRc::new(VecModel::from(rows)));
    Ok(())
}

pub fn bind(
    window: &AppWindow,
    repository: Rc<RefCell<Repository>>,
    state: Rc<RefCell<TableState>>,
) {
    let weak = window.as_weak();
    let repo = repository.clone();
    window.on_open_endpoints(move |server| {
        let Some(w) = weak.upgrade() else { return };
        w.set_endpoint_server(server);
        w.set_endpoint_error("".into());
        w.invoke_endpoint_new();
        match update(&w, &repo.borrow()) {
            Ok(()) => w.set_endpoints_open(true),
            Err(e) => w.set_status_message(e.to_string().into()),
        }
    });
    let weak = window.as_weak();
    window.on_open_endpoint(move |protocol, address| {
        let Some(w) = weak.upgrade() else { return };
        let scheme = protocol.trim().to_ascii_lowercase();
        let target = match scheme.as_str() {
            "http" | "https" => format!("{scheme}://{address}"),
            _ => {
                w.set_endpoint_error(
                    "Only HTTP and HTTPS can open in a browser. Copy the SSH command or address for other protocols.".into(),
                );
                return;
            }
        };
        if let Err(error) = webbrowser::open(&target) {
            w.set_endpoint_error(format!("Could not open {target}: {error}").into());
        } else {
            w.set_endpoint_error("Opened without including stored credentials.".into());
        }
    });
    let weak = window.as_weak();
    let repo = repository.clone();
    let view = state.clone();
    window.on_endpoint_save(move || {
        let Some(w) = weak.upgrade() else { return };
        let transport = ["Unspecified", "TCP", "UDP"]
            .get(w.get_endpoint_transport() as usize)
            .copied()
            .unwrap_or("Unspecified");
        let result = repo.borrow_mut().save_endpoint(
            i64::from(w.get_endpoint_server()),
            i64::from(w.get_endpoint_id()),
            &w.get_endpoint_port(),
            transport,
            &w.get_endpoint_protocol(),
            &w.get_endpoint_purpose(),
        );
        let result = result.and_then(|()| {
            update(&w, &repo.borrow())?;
            refresh(&w, &repo.borrow(), &view.borrow())
        });
        match result {
            Ok(()) => {
                w.set_endpoint_error("".into());
                w.invoke_endpoint_new();
            }
            Err(e) => w.set_endpoint_error(e.to_string().into()),
        }
    });
    let weak = window.as_weak();
    window.on_endpoint_delete(move |id| {
        let Some(w) = weak.upgrade() else { return };
        let result = repository
            .borrow_mut()
            .delete_endpoint(i64::from(w.get_endpoint_server()), i64::from(id));
        let result = result.and_then(|()| {
            update(&w, &repository.borrow())?;
            refresh(&w, &repository.borrow(), &state.borrow())
        });
        match result {
            Ok(()) => {
                w.set_endpoint_error("".into());
                w.invoke_endpoint_new();
            }
            Err(e) => w.set_endpoint_error(e.to_string().into()),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ServerDraft;

    fn fixture() -> Result<(Repository, i64), StorageError> {
        let mut repo = Repository::in_memory()?;
        let id = repo.save_server(
            None,
            &ServerDraft {
                name: "gateway".into(),
                ip_address: "2001:db8::1".into(),
                ports: "22, 443".into(),
                environment: "Production".into(),
                status: "Active".into(),
                purpose: "test".into(),
                tags: vec!["test".into()],
            },
        )?;
        Ok((repo, id))
    }

    #[test]
    fn legacy_ports_are_independent_without_guessed_protocols() -> Result<(), StorageError> {
        let (mut repo, server) = fixture()?;
        let rows = repo.endpoints(server)?;
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|e| e.transport == "Unspecified" && e.protocol.is_empty())
        );
        repo.save_endpoint(
            server,
            rows[0].id,
            "22",
            "TCP",
            "SSH",
            "Administrator login",
        )?;
        repo.update_server_cell(server, 0, "renamed")?;
        let rows = repo.endpoints(server)?;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].purpose, "Administrator login");
        repo.update_server_cell(server, 2, "22, 8443")?;
        let rows = repo.endpoints(server)?;
        assert_eq!(rows.iter().map(|e| e.port).collect::<Vec<_>>(), [22, 8443]);
        assert_eq!(rows[0].protocol, "SSH");
        Ok(())
    }

    #[test]
    fn transport_duplicates_validation_and_atomic_failure() -> Result<(), StorageError> {
        let (mut repo, server) = fixture()?;
        repo.save_endpoint(server, 0, "53", "TCP", "DNS", "Zone transfer")?;
        repo.save_endpoint(server, 0, "53", "UDP", "DNS", "Queries")?;
        assert_eq!(repo.endpoints(server)?.len(), 4);
        assert!(
            repo.save_endpoint(server, 0, "53", "UDP", "DNS", "")
                .is_err()
        );
        for port in ["0", "65536", "x", "22, 443"] {
            assert!(repo.save_endpoint(server, 0, port, "TCP", "", "").is_err());
        }
        assert!(
            repo.save_endpoint(server, 999999, "25", "TCP", "SMTP", "")
                .is_err()
        );
        assert!(repo.save_endpoint(server, 0, "25", "SCTP", "", "").is_err());
        assert_eq!(repo.endpoints(server)?.len(), 4);
        let ports: String = repo.connection.query_row(
            "SELECT ports FROM server_records WHERE id=?1",
            [server],
            |r| r.get(0),
        )?;
        assert_eq!(ports, "22, 53, 443");
        for row in repo.endpoints(server)?.into_iter().take(3) {
            repo.delete_endpoint(server, row.id)?;
        }
        let row = repo.endpoints(server)?.remove(0);
        assert!(repo.delete_endpoint(server, row.id).is_err());
        assert_eq!(repo.endpoints(server)?.len(), 1);
        assert_eq!(address("2001:db8::1", 443), "[2001:db8::1]:443");
        Ok(())
    }

    #[test]
    fn migration_backfills_existing_rows_and_reopens_metadata() -> Result<(), StorageError> {
        let (mut repo, server) = fixture()?;
        repo.connection.execute_batch("DROP TRIGGER server_endpoint_insert; DROP TRIGGER server_endpoint_sync; DROP TABLE server_endpoints; PRAGMA user_version=17;")?;
        repo.connection
            .execute_batch(include_str!("../migrations/018_server_endpoints.sql"))?;
        let rows = repo.endpoints(server)?;
        assert_eq!(rows.len(), 2);
        repo.save_endpoint(server, rows[0].id, "22", "TCP", "SSH", "Admin")?;
        let path = std::env::temp_dir().join(format!(
            "itgla-endpoints-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        repo.connection.pragma_update(None, "user_version", 18)?;
        repo.connection.backup(rusqlite::MAIN_DB, &path, None)?;
        let reopened = Repository::open(path.clone())?;
        assert_eq!(reopened.endpoints(server)?[0].purpose, "Admin");
        drop(reopened);
        let _ = std::fs::remove_file(path);
        Ok(())
    }
}

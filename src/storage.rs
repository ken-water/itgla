use std::{env, fs, path::PathBuf};

use rusqlite::Connection;
#[cfg(test)]
use rusqlite::OptionalExtension;
use thiserror::Error;

use crate::domain::{Asset, Health, Project, ResourceKind};

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("cannot determine a user data directory; set ITGLA_DATA_DIR")]
    DataDirectoryUnavailable,
    #[error("cannot create data directory {path}")]
    CreateDataDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("database schema {found} is newer than supported version {supported}")]
    UnsupportedSchema { found: i64, supported: i64 },
    #[error("database operation failed")]
    Database(#[from] rusqlite::Error),
}

pub struct Repository {
    connection: Connection,
}

impl Repository {
    pub fn open_default() -> Result<Self, StorageError> {
        let directory = data_directory()?;
        fs::create_dir_all(&directory).map_err(|source| StorageError::CreateDataDirectory {
            path: directory.clone(),
            source,
        })?;
        Self::open(directory.join("itgla.db"))
    }

    pub fn open(path: PathBuf) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    #[cfg(test)]
    fn in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, StorageError> {
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
        let mut repository = Self { connection };
        repository.migrate()?;
        repository.seed_if_empty()?;
        Ok(repository)
    }

    fn migrate(&mut self) -> Result<(), StorageError> {
        let current: i64 = self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if current > SCHEMA_VERSION {
            return Err(StorageError::UnsupportedSchema {
                found: current,
                supported: SCHEMA_VERSION,
            });
        }
        if current == 0 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/001_initial.sql"))?;
            transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            transaction.commit()?;
        }
        Ok(())
    }

    fn seed_if_empty(&mut self) -> Result<(), StorageError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?;
        if count != 0 {
            return Ok(());
        }
        let transaction = self.connection.transaction()?;
        transaction.execute_batch(include_str!("../migrations/seed.sql"))?;
        transaction.commit()?;
        Ok(())
    }

    pub fn projects(&self) -> Result<Vec<Project>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name FROM projects WHERE archived_at IS NULL ORDER BY position, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn assets_for_project(&self, project_id: i64) -> Result<Vec<Asset>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, kind, name, detail, status_detail, environment, health
             FROM assets WHERE project_id = ?1 AND archived_at IS NULL ORDER BY position, id",
        )?;
        let rows = statement.query_map([project_id], |row| {
            let kind: String = row.get(2)?;
            let health: String = row.get(7)?;
            Ok(Asset {
                id: row.get(0)?,
                project_id: row.get(1)?,
                kind: parse_kind(&kind),
                name: row.get(3)?,
                detail: row.get(4)?,
                status_detail: row.get(5)?,
                environment: row.get(6)?,
                health: parse_health(&health),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    #[cfg(test)]
    fn schema_version(&self) -> Result<i64, StorageError> {
        self.connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn project_named(&self, name: &str) -> Result<Option<i64>, StorageError> {
        self.connection
            .query_row("SELECT id FROM projects WHERE name = ?1", [name], |row| {
                row.get(0)
            })
            .optional()
            .map_err(Into::into)
    }
}

fn data_directory() -> Result<PathBuf, StorageError> {
    if let Some(path) = env::var_os("ITGLA_DATA_DIR").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    #[cfg(target_os = "windows")]
    let base = env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".local/share"))
    });
    base.map(|path| path.join("itgla"))
        .ok_or(StorageError::DataDirectoryUnavailable)
}

fn parse_kind(value: &str) -> ResourceKind {
    match value {
        "website" => ResourceKind::Website,
        "domain" => ResourceKind::Domain,
        "certificate" => ResourceKind::Certificate,
        "server" => ResourceKind::Server,
        _ => ResourceKind::Service,
    }
}

fn parse_health(value: &str) -> Health {
    match value {
        "warning" => Health::Warning,
        "critical" => Health::Critical,
        _ => Health::Healthy,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn migrates_and_seeds_an_empty_database() -> Result<(), StorageError> {
        let repository = Repository::in_memory()?;
        assert_eq!(repository.schema_version()?, 1);
        assert_eq!(repository.projects()?.len(), 3);
        assert_eq!(repository.assets_for_project(1)?.len(), 6);
        Ok(())
    }

    #[test]
    fn file_database_survives_reopen_without_duplicate_seed() -> Result<(), StorageError> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = env::temp_dir().join(format!("itgla-storage-{nonce}.db"));
        {
            let repository = Repository::open(path.clone())?;
            assert_eq!(repository.project_named("Cloudnote")?, Some(1));
        }
        let repository = Repository::open(path.clone())?;
        assert_eq!(repository.projects()?.len(), 3);
        drop(repository);
        let _ = fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn rejects_unknown_newer_schema() -> Result<(), StorageError> {
        let connection = Connection::open_in_memory()?;
        connection.pragma_update(None, "user_version", 99)?;
        let error = Repository::from_connection(connection).err();
        assert!(matches!(
            error,
            Some(StorageError::UnsupportedSchema {
                found: 99,
                supported: 1
            })
        ));
        Ok(())
    }
}

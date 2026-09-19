use std::{env, fs, path::PathBuf};

#[cfg(test)]
use rusqlite::OptionalExtension;
use rusqlite::{Connection, ErrorCode, params};
use thiserror::Error;

use crate::domain::{
    Asset, AssetDraft, Health, Project, ResourceKind, ValidationError, validate_name,
};

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
    #[error("{0}")]
    Validation(#[from] ValidationError),
    #[error("同一范围内已存在同名项目或资源")]
    Conflict,
    #[error("至少需要保留一个项目")]
    LastProject,
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
            "SELECT p.id, p.name, COUNT(a.id),
                    COALESCE(SUM(CASE WHEN a.health != 'healthy' THEN 1 ELSE 0 END), 0)
             FROM projects p
             LEFT JOIN assets a ON a.project_id = p.id AND a.archived_at IS NULL
             WHERE p.archived_at IS NULL
             GROUP BY p.id, p.name, p.position
             ORDER BY p.position, p.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                asset_count: row.get(2)?,
                attention_count: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn assets_for_project(&self, project_id: i64) -> Result<Vec<Asset>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT a.id, a.project_id, a.kind, a.name, a.detail, a.status_detail, a.environment, a.health,
                    COALESCE((SELECT group_concat(name, ', ') FROM (
                        SELECT t.name FROM tags t
                        JOIN asset_tags at ON at.tag_id = t.id
                        WHERE at.asset_id = a.id ORDER BY t.name COLLATE NOCASE
                    )), '')
             FROM assets a WHERE a.project_id = ?1 AND a.archived_at IS NULL ORDER BY a.position, a.id",
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
                tags: row
                    .get::<_, String>(8)?
                    .split(", ")
                    .filter(|tag| !tag.is_empty())
                    .map(ToOwned::to_owned)
                    .collect(),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_project(&mut self, id: Option<i64>, name: &str) -> Result<i64, StorageError> {
        let name = validate_name(name, 120)?;
        let result = if let Some(id) = id {
            self.connection
                .execute(
                    "UPDATE projects SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND archived_at IS NULL",
                    params![name, id],
                )
                .map(|_| id)
        } else {
            self.connection
                .execute(
                    "INSERT INTO projects (name, position) VALUES (?1, (SELECT COALESCE(MAX(position), -1) + 1 FROM projects))",
                    [name],
                )
                .map(|_| self.connection.last_insert_rowid())
        };
        result.map_err(map_write_error)
    }

    pub fn archive_project(&mut self, id: i64) -> Result<(), StorageError> {
        let active: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM projects WHERE archived_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        if active <= 1 {
            return Err(StorageError::LastProject);
        }
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE assets SET archived_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE project_id = ?1 AND archived_at IS NULL",
            [id],
        )?;
        transaction.execute(
            "UPDATE projects SET archived_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND archived_at IS NULL",
            [id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn save_asset(&mut self, id: Option<i64>, draft: &AssetDraft) -> Result<i64, StorageError> {
        let name = validate_name(&draft.name, 240)?;
        let transaction = self.connection.transaction()?;
        let write_result = if let Some(id) = id {
            transaction
                .execute(
                    "UPDATE assets SET kind = ?1, name = ?2, detail = ?3, status_detail = ?4,
                     environment = ?5, health = ?6, updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?7 AND project_id = ?8 AND archived_at IS NULL",
                    params![
                        draft.kind.key(),
                        name,
                        draft.detail.trim(),
                        draft.status_detail.trim(),
                        draft.environment.trim(),
                        draft.health.key(),
                        id,
                        draft.project_id
                    ],
                )
                .map(|_| id)
        } else {
            transaction
                .execute(
                    "INSERT INTO assets (project_id, kind, name, detail, status_detail, environment, health, position)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                       (SELECT COALESCE(MAX(position), -1) + 1 FROM assets WHERE project_id = ?1))",
                    params![draft.project_id, draft.kind.key(), name, draft.detail.trim(), draft.status_detail.trim(), draft.environment.trim(), draft.health.key()],
                )
                .map(|_| transaction.last_insert_rowid())
        };
        let asset_id = write_result.map_err(map_write_error)?;
        transaction.execute("DELETE FROM asset_tags WHERE asset_id = ?1", [asset_id])?;
        for tag in &draft.tags {
            transaction.execute(
                "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
                [tag],
            )?;
            transaction.execute(
                "INSERT INTO asset_tags (asset_id, tag_id)
                 SELECT ?1, id FROM tags WHERE name = ?2 COLLATE NOCASE",
                params![asset_id, tag],
            )?;
        }
        transaction.commit()?;
        Ok(asset_id)
    }

    pub fn archive_asset(&self, id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE assets SET archived_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND archived_at IS NULL",
            [id],
        )?;
        Ok(())
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

fn map_write_error(error: rusqlite::Error) -> StorageError {
    if matches!(
        error,
        rusqlite::Error::SqliteFailure(ref value, _)
            if value.code == ErrorCode::ConstraintViolation
    ) {
        StorageError::Conflict
    } else {
        StorageError::Database(error)
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

    #[test]
    fn creates_updates_tags_and_archives_assets() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let draft = AssetDraft {
            project_id: 1,
            kind: ResourceKind::Service,
            name: "webhook-worker".into(),
            detail: "Docker".into(),
            status_detail: "ready".into(),
            environment: "staging".into(),
            health: Health::Healthy,
            tags: vec!["backend".into(), "payments".into()],
        };
        let id = repository.save_asset(None, &draft)?;
        let created = repository
            .assets_for_project(1)?
            .into_iter()
            .find(|asset| asset.id == id);
        assert_eq!(created.map(|asset| asset.tags), Some(draft.tags.clone()));

        let mut updated = draft;
        updated.name = "webhook-consumer".into();
        updated.tags = vec!["backend".into()];
        repository.save_asset(Some(id), &updated)?;
        assert_eq!(
            repository
                .assets_for_project(1)?
                .into_iter()
                .find(|asset| asset.id == id)
                .map(|asset| (asset.name, asset.tags)),
            Some((updated.name, updated.tags))
        );

        repository.archive_asset(id)?;
        assert!(
            repository
                .assets_for_project(1)?
                .iter()
                .all(|asset| asset.id != id)
        );
        Ok(())
    }

    #[test]
    fn project_crud_enforces_uniqueness_and_last_project_rule() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let id = repository.save_project(None, "Atlas")?;
        repository.save_project(Some(id), "Atlas Cloud")?;
        assert!(matches!(
            repository.save_project(None, "Cloudnote"),
            Err(StorageError::Conflict)
        ));
        repository.archive_project(id)?;
        assert!(
            repository
                .projects()?
                .iter()
                .all(|project| project.id != id)
        );

        for project in repository.projects()?.into_iter().skip(1) {
            repository.archive_project(project.id)?;
        }
        let remaining = repository.projects()?[0].id;
        assert!(matches!(
            repository.archive_project(remaining),
            Err(StorageError::LastProject)
        ));
        Ok(())
    }
}

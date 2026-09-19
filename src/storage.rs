use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(test)]
use rusqlite::OptionalExtension;
use rusqlite::{Connection, ErrorCode, MAIN_DB, Transaction, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    Asset, AssetDraft, GlobalAsset, Health, Project, Relationship, RelationshipKind, ResourceKind,
    ValidationError, validate_name,
};

const SCHEMA_VERSION: i64 = 2;
const PORTABLE_FORMAT_VERSION: u32 = 1;
const MAX_IMPORT_BYTES: u64 = 10 * 1024 * 1024;
const MAX_PROJECTS: usize = 1_000;
const MAX_ASSETS: usize = 10_000;
const MAX_RELATIONSHIPS: usize = 50_000;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceFile {
    format_version: u32,
    projects: Vec<PortableProject>,
    assets: Vec<PortableAsset>,
    relationships: Vec<PortableRelationship>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableProject {
    id: i64,
    name: String,
    description: String,
    position: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableAsset {
    id: i64,
    project_id: i64,
    kind: String,
    name: String,
    detail: String,
    status_detail: String,
    environment: String,
    health: String,
    position: i64,
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableRelationship {
    id: i64,
    source_asset_id: i64,
    target_asset_id: i64,
    kind: String,
}

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
    #[error("{operation}失败：{path}")]
    FileOperation {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON 数据无效")]
    Json(#[from] serde_json::Error),
    #[error("导入文件超过 {maximum_mb} MiB 限制")]
    ImportTooLarge { maximum_mb: u64 },
    #[error("不支持 JSON 格式版本 {found}，当前支持 {supported}")]
    UnsupportedImportVersion { found: u32, supported: u32 },
    #[error("导入内容无效：{0}")]
    InvalidImport(String),
    #[error("备份数据库 schema {found} 与当前 schema {supported} 不兼容")]
    IncompatibleBackup { found: i64, supported: i64 },
    #[error("备份数据库无效：{0}")]
    InvalidBackup(String),
}

pub struct Repository {
    connection: Connection,
    database_path: Option<PathBuf>,
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
        let connection = Connection::open(&path)?;
        Self::from_connection(connection, Some(path))
    }

    #[cfg(test)]
    fn in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?, None)
    }

    fn from_connection(
        connection: Connection,
        database_path: Option<PathBuf>,
    ) -> Result<Self, StorageError> {
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
        let mut repository = Self {
            connection,
            database_path,
        };
        repository.migrate_initial()?;
        repository.seed_if_empty()?;
        repository.migrate_remaining()?;
        Ok(repository)
    }

    pub fn default_portability_paths() -> Result<(PathBuf, PathBuf), StorageError> {
        let directory = data_directory()?;
        Ok((
            directory.join("exports/itgla-workspace.json"),
            directory.join("backups/itgla-backup.db"),
        ))
    }

    fn schema_version(&self) -> Result<i64, StorageError> {
        self.connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(Into::into)
    }

    fn migrate_initial(&mut self) -> Result<(), StorageError> {
        let current = self.schema_version()?;
        if current > SCHEMA_VERSION {
            return Err(StorageError::UnsupportedSchema {
                found: current,
                supported: SCHEMA_VERSION,
            });
        }
        if current == 0 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/001_initial.sql"))?;
            transaction.pragma_update(None, "user_version", 1)?;
            transaction.commit()?;
        }
        Ok(())
    }

    fn migrate_remaining(&mut self) -> Result<(), StorageError> {
        if self.schema_version()? == 1 {
            let transaction = self.connection.transaction()?;
            transaction
                .execute_batch(include_str!("../migrations/002_relationship_lifecycle.sql"))?;
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

    pub fn search_assets(
        &self,
        query: &str,
        attention_only: bool,
    ) -> Result<Vec<GlobalAsset>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT p.name, a.id, a.project_id, a.kind, a.name, a.detail, a.status_detail,
                    a.environment, a.health,
                    COALESCE((SELECT group_concat(name, ', ') FROM (
                        SELECT t.name FROM tags t
                        JOIN asset_tags at ON at.tag_id = t.id
                        WHERE at.asset_id = a.id ORDER BY t.name COLLATE NOCASE
                    )), '')
             FROM assets a JOIN projects p ON p.id = a.project_id
             WHERE a.archived_at IS NULL AND p.archived_at IS NULL
             ORDER BY CASE a.health WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
                      p.position, a.position, a.id",
        )?;
        let rows = statement.query_map([], |row| {
            let kind: String = row.get(3)?;
            let health: String = row.get(8)?;
            Ok(GlobalAsset {
                project_name: row.get(0)?,
                asset: Asset {
                    id: row.get(1)?,
                    project_id: row.get(2)?,
                    kind: parse_kind(&kind),
                    name: row.get(4)?,
                    detail: row.get(5)?,
                    status_detail: row.get(6)?,
                    environment: row.get(7)?,
                    health: parse_health(&health),
                    tags: row
                        .get::<_, String>(9)?
                        .split(", ")
                        .filter(|tag| !tag.is_empty())
                        .map(ToOwned::to_owned)
                        .collect(),
                },
            })
        })?;
        let normalized = query.trim().to_lowercase();
        rows.filter_map(|row| match row {
            Ok(result)
                if (!attention_only || result.asset.health != Health::Healthy)
                    && (normalized.is_empty()
                        || result.asset.name.to_lowercase().contains(&normalized)
                        || result.asset.detail.to_lowercase().contains(&normalized)
                        || result.project_name.to_lowercase().contains(&normalized)
                        || result.asset.kind.label().contains(&normalized)
                        || result
                            .asset
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase().contains(&normalized))) =>
            {
                Some(Ok(result))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .take(100)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
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

    pub fn relationships_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<Relationship>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT r.id, r.source_asset_id, source.name, r.target_asset_id, target.name, r.kind
             FROM relationships r
             JOIN assets source ON source.id = r.source_asset_id
             JOIN assets target ON target.id = r.target_asset_id
             WHERE source.project_id = ?1 AND target.project_id = ?1
               AND source.archived_at IS NULL AND target.archived_at IS NULL
               AND r.archived_at IS NULL
             ORDER BY source.name COLLATE NOCASE, target.name COLLATE NOCASE, r.id",
        )?;
        let rows = statement.query_map([project_id], |row| {
            let kind: String = row.get(5)?;
            Ok(Relationship {
                id: row.get(0)?,
                source_asset_id: row.get(1)?,
                source_name: row.get(2)?,
                target_asset_id: row.get(3)?,
                target_name: row.get(4)?,
                kind: parse_relationship_kind(&kind),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_relationship(
        &self,
        project_id: i64,
        source_asset_id: i64,
        target_asset_id: i64,
        kind: RelationshipKind,
    ) -> Result<i64, StorageError> {
        if source_asset_id == target_asset_id {
            return Err(StorageError::Conflict);
        }
        let valid_endpoints: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM assets
             WHERE id IN (?1, ?2) AND project_id = ?3 AND archived_at IS NULL",
            params![source_asset_id, target_asset_id, project_id],
            |row| row.get(0),
        )?;
        if valid_endpoints != 2 {
            return Err(StorageError::Conflict);
        }
        self.connection
            .execute(
                "INSERT INTO relationships (source_asset_id, target_asset_id, kind)
                 VALUES (?1, ?2, ?3)",
                params![source_asset_id, target_asset_id, kind.key()],
            )
            .map_err(map_write_error)?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn archive_relationship(&self, id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE relationships SET archived_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND archived_at IS NULL",
            [id],
        )?;
        Ok(())
    }

    pub fn export_json(&self, path: &Path) -> Result<(), StorageError> {
        self.ensure_external_path(path)?;
        let workspace = self.read_workspace()?;
        let contents = serde_json::to_vec_pretty(&workspace)?;
        write_atomic(path, &contents)
    }

    pub fn import_json(&mut self, path: &Path) -> Result<(), StorageError> {
        self.ensure_external_path(path)?;
        let workspace = read_workspace_file(path)?;
        validate_workspace(&workspace)?;
        self.automatic_backup("pre-import.db")?;
        let transaction = self.connection.transaction()?;
        replace_workspace(&transaction, &workspace)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn create_backup(&self, path: &Path) -> Result<(), StorageError> {
        self.ensure_external_path(path)?;
        create_parent(path)?;
        self.connection.backup(MAIN_DB, path, None)?;
        validate_backup(path)
    }

    pub fn restore_backup(&mut self, path: &Path) -> Result<(), StorageError> {
        self.ensure_external_path(path)?;
        if self.automatic_backup_path("pre-restore.db").as_deref() == Some(path) {
            return Err(StorageError::InvalidImport(
                "恢复源不能与自动恢复点使用同一路径".into(),
            ));
        }
        validate_backup(path)?;
        self.automatic_backup("pre-restore.db")?;
        self.connection
            .restore(MAIN_DB, path, None::<fn(rusqlite::backup::Progress)>)?;
        self.connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
        if self.schema_version()? != SCHEMA_VERSION {
            return Err(StorageError::IncompatibleBackup {
                found: self.schema_version()?,
                supported: SCHEMA_VERSION,
            });
        }
        Ok(())
    }

    fn read_workspace(&self) -> Result<WorkspaceFile, StorageError> {
        let mut project_statement = self.connection.prepare(
            "SELECT id, name, description, position FROM projects
             WHERE archived_at IS NULL ORDER BY position, id",
        )?;
        let projects = project_statement
            .query_map([], |row| {
                Ok(PortableProject {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    position: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut asset_statement = self.connection.prepare(
            "SELECT id, project_id, kind, name, detail, status_detail, environment, health, position
             FROM assets WHERE archived_at IS NULL ORDER BY project_id, position, id",
        )?;
        let asset_rows = asset_statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut assets = Vec::with_capacity(asset_rows.len());
        for (id, project_id, kind, name, detail, status_detail, environment, health, position) in
            asset_rows
        {
            let mut tag_statement = self.connection.prepare(
                "SELECT t.name FROM tags t JOIN asset_tags at ON at.tag_id = t.id
                 WHERE at.asset_id = ?1 ORDER BY t.name COLLATE NOCASE",
            )?;
            let tags = tag_statement
                .query_map([id], |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            assets.push(PortableAsset {
                id,
                project_id,
                kind,
                name,
                detail,
                status_detail,
                environment,
                health,
                position,
                tags,
            });
        }

        let mut relationship_statement = self.connection.prepare(
            "SELECT r.id, r.source_asset_id, r.target_asset_id, r.kind
             FROM relationships r
             JOIN assets source ON source.id = r.source_asset_id
             JOIN assets target ON target.id = r.target_asset_id
             JOIN projects project ON project.id = source.project_id
             WHERE r.archived_at IS NULL AND source.archived_at IS NULL
               AND target.archived_at IS NULL AND project.archived_at IS NULL
               AND source.project_id = target.project_id
             ORDER BY r.id",
        )?;
        let relationships = relationship_statement
            .query_map([], |row| {
                Ok(PortableRelationship {
                    id: row.get(0)?,
                    source_asset_id: row.get(1)?,
                    target_asset_id: row.get(2)?,
                    kind: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WorkspaceFile {
            format_version: PORTABLE_FORMAT_VERSION,
            projects,
            assets,
            relationships,
        })
    }

    fn automatic_backup(&self, filename: &str) -> Result<(), StorageError> {
        let Some(path) = self.automatic_backup_path(filename) else {
            return Ok(());
        };
        create_parent(&path)?;
        self.connection.backup(MAIN_DB, &path, None)?;
        Ok(())
    }

    fn automatic_backup_path(&self, filename: &str) -> Option<PathBuf> {
        self.database_path
            .as_deref()
            .and_then(Path::parent)
            .map(|directory| directory.join("backups").join(filename))
    }

    fn ensure_external_path(&self, path: &Path) -> Result<(), StorageError> {
        if path.as_os_str().is_empty() || self.database_path.as_deref() == Some(path) {
            return Err(StorageError::InvalidImport(
                "请选择与当前数据库不同的有效文件路径".into(),
            ));
        }
        Ok(())
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

fn create_parent(path: &Path) -> Result<(), StorageError> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|source| StorageError::FileOperation {
        operation: "创建目录",
        path: parent.to_owned(),
        source,
    })
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), StorageError> {
    if path.as_os_str().is_empty() {
        return Err(StorageError::InvalidImport("导出路径不能为空".into()));
    }
    create_parent(path)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("json");
    let temporary = path.with_extension(format!("{extension}.tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = File::create(&temporary).map_err(|source| StorageError::FileOperation {
            operation: "创建临时导出文件",
            path: temporary.clone(),
            source,
        })?;
        file.write_all(contents)
            .and_then(|()| file.sync_all())
            .map_err(|source| StorageError::FileOperation {
                operation: "写入导出文件",
                path: temporary.clone(),
                source,
            })?;
        fs::rename(&temporary, path).map_err(|source| StorageError::FileOperation {
            operation: "提交导出文件",
            path: path.to_owned(),
            source,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_workspace_file(path: &Path) -> Result<WorkspaceFile, StorageError> {
    let metadata = fs::metadata(path).map_err(|source| StorageError::FileOperation {
        operation: "读取导入文件",
        path: path.to_owned(),
        source,
    })?;
    if metadata.len() > MAX_IMPORT_BYTES {
        return Err(StorageError::ImportTooLarge {
            maximum_mb: MAX_IMPORT_BYTES / 1024 / 1024,
        });
    }
    let file = File::open(path).map_err(|source| StorageError::FileOperation {
        operation: "打开导入文件",
        path: path.to_owned(),
        source,
    })?;
    let mut contents = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_IMPORT_BYTES + 1)
        .read_to_end(&mut contents)
        .map_err(|source| StorageError::FileOperation {
            operation: "读取导入文件",
            path: path.to_owned(),
            source,
        })?;
    if contents.len() as u64 > MAX_IMPORT_BYTES {
        return Err(StorageError::ImportTooLarge {
            maximum_mb: MAX_IMPORT_BYTES / 1024 / 1024,
        });
    }
    serde_json::from_slice(&contents).map_err(Into::into)
}

fn validate_workspace(workspace: &WorkspaceFile) -> Result<(), StorageError> {
    if workspace.format_version != PORTABLE_FORMAT_VERSION {
        return Err(StorageError::UnsupportedImportVersion {
            found: workspace.format_version,
            supported: PORTABLE_FORMAT_VERSION,
        });
    }
    if workspace.projects.is_empty() || workspace.projects.len() > MAX_PROJECTS {
        return Err(StorageError::InvalidImport(format!(
            "项目数量必须在 1 到 {MAX_PROJECTS} 之间"
        )));
    }
    if workspace.assets.len() > MAX_ASSETS || workspace.relationships.len() > MAX_RELATIONSHIPS {
        return Err(StorageError::InvalidImport(format!(
            "最多允许 {MAX_ASSETS} 个资源和 {MAX_RELATIONSHIPS} 条关系"
        )));
    }

    let mut project_ids = HashSet::new();
    let mut project_names = HashSet::new();
    for project in &workspace.projects {
        validate_name(&project.name, 120)?;
        if project.id <= 0
            || !project_ids.insert(project.id)
            || !project_names.insert(project.name.trim().to_lowercase())
        {
            return Err(StorageError::InvalidImport("项目标识或名称重复".into()));
        }
    }

    let mut asset_projects = HashMap::new();
    let mut asset_names = HashSet::new();
    for asset in &workspace.assets {
        validate_name(&asset.name, 240)?;
        if !project_ids.contains(&asset.project_id)
            || asset.id <= 0
            || asset_projects.insert(asset.id, asset.project_id).is_some()
            || parse_kind_checked(&asset.kind).is_none()
            || parse_health_checked(&asset.health).is_none()
        {
            return Err(StorageError::InvalidImport(
                "资源标识、项目、类型或状态无效".into(),
            ));
        }
        if !asset_names.insert((
            asset.project_id,
            asset.kind.clone(),
            asset.name.trim().to_lowercase(),
        )) {
            return Err(StorageError::InvalidImport("同项目资源重复".into()));
        }
        let mut tags = HashSet::new();
        if asset.tags.len() > 20
            || asset.tags.iter().any(|tag| {
                tag.trim().is_empty()
                    || tag.chars().count() > 48
                    || !tags.insert(tag.trim().to_lowercase())
            })
        {
            return Err(StorageError::InvalidImport("资源标签无效".into()));
        }
    }

    let mut relationship_ids = HashSet::new();
    let mut relationships = HashSet::new();
    for relationship in &workspace.relationships {
        let source_project = asset_projects.get(&relationship.source_asset_id);
        let target_project = asset_projects.get(&relationship.target_asset_id);
        if relationship.id <= 0
            || !relationship_ids.insert(relationship.id)
            || relationship.source_asset_id == relationship.target_asset_id
            || source_project.is_none()
            || source_project != target_project
            || parse_relationship_kind_checked(&relationship.kind).is_none()
            || !relationships.insert((
                relationship.source_asset_id,
                relationship.target_asset_id,
                relationship.kind.clone(),
            ))
        {
            return Err(StorageError::InvalidImport("资源关系无效或重复".into()));
        }
    }
    Ok(())
}

fn replace_workspace(
    transaction: &Transaction<'_>,
    workspace: &WorkspaceFile,
) -> Result<(), StorageError> {
    transaction.execute_batch(
        "DELETE FROM relationships; DELETE FROM asset_tags; DELETE FROM tags;
         DELETE FROM assets; DELETE FROM projects;",
    )?;
    for project in &workspace.projects {
        transaction.execute(
            "INSERT INTO projects (id, name, description, position) VALUES (?1, ?2, ?3, ?4)",
            params![
                project.id,
                project.name.trim(),
                project.description,
                project.position
            ],
        )?;
    }
    for asset in &workspace.assets {
        transaction.execute(
            "INSERT INTO assets
             (id, project_id, kind, name, detail, status_detail, environment, health, position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                asset.id,
                asset.project_id,
                asset.kind,
                asset.name.trim(),
                asset.detail,
                asset.status_detail,
                asset.environment,
                asset.health,
                asset.position
            ],
        )?;
        for tag in &asset.tags {
            transaction.execute(
                "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
                [tag.trim()],
            )?;
            transaction.execute(
                "INSERT INTO asset_tags (asset_id, tag_id)
                 SELECT ?1, id FROM tags WHERE name = ?2 COLLATE NOCASE",
                params![asset.id, tag.trim()],
            )?;
        }
    }
    for relationship in &workspace.relationships {
        transaction.execute(
            "INSERT INTO relationships (id, source_asset_id, target_asset_id, kind)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                relationship.id,
                relationship.source_asset_id,
                relationship.target_asset_id,
                relationship.kind
            ],
        )?;
    }
    Ok(())
}

fn validate_backup(path: &Path) -> Result<(), StorageError> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version != SCHEMA_VERSION {
        return Err(StorageError::IncompatibleBackup {
            found: version,
            supported: SCHEMA_VERSION,
        });
    }
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StorageError::InvalidBackup(
            "备份数据库完整性检查失败".into(),
        ));
    }
    let active_projects: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE archived_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|_| StorageError::InvalidBackup("缺少必需的项目数据表".into()))?;
    if active_projects == 0 {
        return Err(StorageError::InvalidBackup(
            "备份中没有可恢复的有效项目".into(),
        ));
    }
    connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM assets) + (SELECT COUNT(*) FROM tags)
                  + (SELECT COUNT(*) FROM asset_tags) + (SELECT COUNT(*) FROM relationships)",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| StorageError::InvalidBackup("缺少必需的资源或关系数据表".into()))?;
    let foreign_key_violation = connection.prepare("PRAGMA foreign_key_check")?.exists([])?;
    let cross_project_relationships: i64 = connection.query_row(
        "SELECT COUNT(*) FROM relationships r
         JOIN assets source ON source.id = r.source_asset_id
         JOIN assets target ON target.id = r.target_asset_id
         WHERE source.project_id != target.project_id",
        [],
        |row| row.get(0),
    )?;
    if foreign_key_violation || cross_project_relationships != 0 {
        return Err(StorageError::InvalidBackup(
            "存在外键违规或跨项目关系".into(),
        ));
    }
    Ok(())
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
    parse_kind_checked(value).unwrap_or(ResourceKind::Service)
}

fn parse_kind_checked(value: &str) -> Option<ResourceKind> {
    match value {
        "website" => Some(ResourceKind::Website),
        "domain" => Some(ResourceKind::Domain),
        "certificate" => Some(ResourceKind::Certificate),
        "server" => Some(ResourceKind::Server),
        "service" => Some(ResourceKind::Service),
        _ => None,
    }
}

fn parse_health(value: &str) -> Health {
    parse_health_checked(value).unwrap_or(Health::Healthy)
}

fn parse_health_checked(value: &str) -> Option<Health> {
    match value {
        "healthy" => Some(Health::Healthy),
        "warning" => Some(Health::Warning),
        "critical" => Some(Health::Critical),
        _ => None,
    }
}

fn parse_relationship_kind(value: &str) -> RelationshipKind {
    parse_relationship_kind_checked(value).unwrap_or(RelationshipKind::DependsOn)
}

fn parse_relationship_kind_checked(value: &str) -> Option<RelationshipKind> {
    match value {
        "deploys_to" => Some(RelationshipKind::DeploysTo),
        "uses_domain" => Some(RelationshipKind::UsesDomain),
        "protected_by" => Some(RelationshipKind::ProtectedBy),
        "depends_on" => Some(RelationshipKind::DependsOn),
        "serves" => Some(RelationshipKind::Serves),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temporary_path(label: &str, extension: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        env::temp_dir().join(format!("itgla-{label}-{nonce}.{extension}"))
    }

    #[test]
    fn migrates_and_seeds_an_empty_database() -> Result<(), StorageError> {
        let repository = Repository::in_memory()?;
        assert_eq!(repository.schema_version()?, 2);
        assert_eq!(repository.projects()?.len(), 3);
        assert_eq!(repository.assets_for_project(1)?.len(), 6);
        Ok(())
    }

    #[test]
    fn file_database_survives_reopen_without_duplicate_seed() -> Result<(), StorageError> {
        let path = temporary_path("storage", "db");
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
        let error = Repository::from_connection(connection, None).err();
        assert!(matches!(
            error,
            Some(StorageError::UnsupportedSchema {
                found: 99,
                supported: 2
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
    fn global_search_crosses_projects_and_filters_attention() -> Result<(), StorageError> {
        let repository = Repository::in_memory()?;
        let northstar = repository.search_assets("northstar", false)?;
        assert_eq!(northstar.len(), 4);
        assert!(
            northstar
                .iter()
                .all(|result| result.project_name == "Northstar API")
        );

        let attention = repository.search_assets("", true)?;
        assert_eq!(attention.len(), 3);
        assert!(
            attention
                .iter()
                .all(|result| result.asset.health != Health::Healthy)
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

    #[test]
    fn migrates_v1_and_manages_typed_relationships() -> Result<(), StorageError> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(include_str!("../migrations/001_initial.sql"))?;
        connection.execute_batch(include_str!("../migrations/seed.sql"))?;
        connection.pragma_update(None, "user_version", 1)?;
        let repository = Repository::from_connection(connection, None)?;
        assert_eq!(repository.schema_version()?, 2);
        assert_eq!(repository.relationships_for_project(1)?.len(), 4);
        let cross_project: i64 = repository.connection.query_row(
            "SELECT COUNT(*) FROM relationships r
             JOIN assets source ON source.id = r.source_asset_id
             JOIN assets target ON target.id = r.target_asset_id
             WHERE source.project_id != target.project_id",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(cross_project, 0);

        let id = repository.save_relationship(1, 5, 1, RelationshipKind::Serves)?;
        assert!(
            repository
                .relationships_for_project(1)?
                .iter()
                .any(|relationship| relationship.id == id
                    && relationship.kind == RelationshipKind::Serves)
        );
        repository.archive_relationship(id)?;
        assert!(
            repository
                .relationships_for_project(1)?
                .iter()
                .all(|relationship| relationship.id != id)
        );
        assert!(matches!(
            repository.save_relationship(1, 1, 1, RelationshipKind::DependsOn),
            Err(StorageError::Conflict)
        ));
        Ok(())
    }

    #[test]
    fn json_export_import_round_trips_workspace() -> Result<(), StorageError> {
        let mut source = Repository::in_memory()?;
        let project_id = source.save_project(None, "Portable Project")?;
        let path = temporary_path("workspace", "json");
        source.export_json(&path)?;
        source.export_json(&path)?;

        let mut target = Repository::in_memory()?;
        target.import_json(&path)?;
        assert_eq!(target.project_named("Portable Project")?, Some(project_id));
        assert_eq!(target.relationships_for_project(1)?.len(), 4);
        let _ = fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn invalid_json_import_preserves_existing_workspace() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let path = temporary_path("invalid-workspace", "json");
        fs::write(
            &path,
            r#"{"format_version":99,"projects":[],"assets":[],"relationships":[]}"#,
        )
        .map_err(|source| StorageError::FileOperation {
            operation: "写入测试文件",
            path: path.clone(),
            source,
        })?;
        assert!(matches!(
            repository.import_json(&path),
            Err(StorageError::UnsupportedImportVersion {
                found: 99,
                supported: 1
            })
        ));
        assert_eq!(repository.projects()?.len(), 3);
        let _ = fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn database_backup_restores_previous_state() -> Result<(), StorageError> {
        let directory = temporary_path("restore", "data");
        fs::create_dir_all(&directory).map_err(|source| StorageError::FileOperation {
            operation: "创建测试目录",
            path: directory.clone(),
            source,
        })?;
        let database_path = directory.join("source.db");
        let backup_path = directory.join("backup.db");
        let mut repository = Repository::open(database_path.clone())?;
        repository.create_backup(&backup_path)?;
        repository.save_project(None, "After Backup")?;
        assert!(repository.project_named("After Backup")?.is_some());

        repository.restore_backup(&backup_path)?;
        assert_eq!(repository.project_named("After Backup")?, None);
        drop(repository);
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }
}

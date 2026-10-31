use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::OptionalExtension;
use rusqlite::{Connection, ErrorCode, MAIN_DB, Transaction, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    Asset, AssetDraft, ChecklistItem, DueState, GlobalAsset, Health, ManagedOption, ManagedTag,
    Project, Relationship, RelationshipKind, ResourceKind, ServerColumn, ServerDraft, ServerRecord,
    SiteDraft, SiteRecord, ValidationError, WorkAssetChoice, WorkDraft, WorkHealth,
    WorkHealthAction, WorkImpactChoice, WorkItem, WorkKind, WorkProjectChoice, WorkServerChoice,
    WorkStatus, validate_name, validate_server, validate_site,
};
use crate::import_data::TabularData;

const SCHEMA_VERSION: i64 = 21;
const PORTABLE_FORMAT_VERSION: u32 = 1;
const MAX_IMPORT_BYTES: u64 = 10 * 1024 * 1024;
const MAX_PROJECTS: usize = 1_000;
const MAX_ASSETS: usize = 10_000;
const MAX_RELATIONSHIPS: usize = 50_000;
const MAX_SERVER_CUSTOM_COLUMNS: usize = 32;
const MAX_INFERRED_IMPACTS: usize = 50;
const DEFAULT_COLUMN_WIDTHS: [f32; 7] = [160.0, 140.0, 110.0, 130.0, 110.0, 220.0, 170.0];

fn migrate_server_column_widths(transaction: &Transaction<'_>) -> Result<(), StorageError> {
    let stored = transaction.query_row(
        "SELECT value FROM app_settings WHERE key='column_widths'",
        [],
        |row| row.get::<_, String>(0),
    );
    let Ok(stored) = stored else { return Ok(()) };
    let Ok(old) = serde_json::from_str::<Vec<f32>>(&stored) else {
        return Ok(());
    };
    if old.len() < 3 {
        return Ok(());
    }

    // Before schema 14 the table order was Tags, IP Address, Ports, then
    // custom columns. Preserve those user-selected widths while inserting
    // the four new first-class fields in their new positions.
    let mut migrated = vec![
        DEFAULT_COLUMN_WIDTHS[0],
        old[1],
        old[2],
        DEFAULT_COLUMN_WIDTHS[3],
        DEFAULT_COLUMN_WIDTHS[4],
        DEFAULT_COLUMN_WIDTHS[5],
        old[0],
    ];
    migrated.extend_from_slice(&old[3..]);
    let value = serde_json::to_string(&migrated)
        .map_err(|error| StorageError::InvalidImport(error.to_string()))?;
    transaction.execute(
        "UPDATE app_settings SET value=?1, updated_at=CURRENT_TIMESTAMP WHERE key='column_widths'",
        [value],
    )?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerImportTarget {
    Skip,
    Name,
    Tags,
    IpAddress,
    Ports,
    Environment,
    Status,
    Purpose,
    ExistingCustom(i64),
    NewCustom(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteImportTarget {
    Skip,
    Name,
    Url,
    Environment,
    Status,
    Purpose,
    Tags,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteImportDefaults {
    pub environment: String,
    pub status: String,
    pub purpose: String,
    pub tags: String,
    pub duplicate_policy: ImportDuplicatePolicy,
}

impl Default for SiteImportDefaults {
    fn default() -> Self {
        Self {
            environment: "production".into(),
            status: "active".into(),
            purpose: String::new(),
            tags: String::new(),
            duplicate_policy: ImportDuplicatePolicy::CreateNew,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPlan {
    pub project_id: i64,
    pub name: String,
    pub description: String,
    pub start_at: String,
    pub priority: String,
    pub launch_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationRecord {
    pub id: i64,
    pub name: String,
    pub publish_at: String,
    pub media: String,
    pub channel: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriorityOption {
    pub label: String,
    pub color: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportDuplicatePolicy {
    CreateNew,
    Skip,
    UpdateExisting,
}

impl ImportDuplicatePolicy {
    pub fn from_index(index: i64) -> Self {
        match index {
            1 => Self::Skip,
            2 => Self::UpdateExisting,
            _ => Self::CreateNew,
        }
    }

    pub fn index(self) -> i64 {
        match self {
            Self::CreateNew => 0,
            Self::Skip => 1,
            Self::UpdateExisting => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspacePreferences {
    pub theme_index: i64,
    pub font_size_step: i64,
    pub table_density: i64,
    pub show_cell_tooltips: bool,
    pub fit_columns_on_open: bool,
    pub site_fit_columns_on_open: bool,
    pub server_default_sort: i64,
    pub site_default_sort: i64,
    pub server_visible_columns: Vec<bool>,
    pub server_default_ports: String,
    pub server_default_tags: String,
    pub site_default_tags: String,
    pub import_duplicate_policy: ImportDuplicatePolicy,
    pub backup_retention: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerImportDefaults {
    pub name: String,
    pub tags: String,
    pub ip_address: String,
    pub ports: String,
    pub environment: String,
    pub status: String,
    pub purpose: String,
    pub duplicate_policy: ImportDuplicatePolicy,
}

impl Default for ServerImportDefaults {
    fn default() -> Self {
        Self {
            name: String::new(),
            tags: "imported".into(),
            ip_address: String::new(),
            ports: String::new(),
            environment: "production".into(),
            status: "active".into(),
            purpose: String::new(),
            duplicate_policy: ImportDuplicatePolicy::CreateNew,
        }
    }
}

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
    #[error("A project or resource with this name already exists")]
    Conflict,
    #[error("At least one project must remain")]
    LastProject,
    #[error("{operation} failed: {path}")]
    FileOperation {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON data is invalid")]
    Json(#[from] serde_json::Error),
    #[error("The import exceeds the {maximum_mb} MiB limit")]
    ImportTooLarge { maximum_mb: u64 },
    #[error("JSON format {found} is not supported; this version supports {supported}")]
    UnsupportedImportVersion { found: u32, supported: u32 },
    #[error("Import data is invalid: {0}")]
    InvalidImport(String),
    #[error("Backup schema {found} is incompatible with current schema {supported}")]
    IncompatibleBackup { found: i64, supported: i64 },
    #[error("Backup database is invalid: {0}")]
    InvalidBackup(String),
}

pub struct Repository {
    pub(crate) connection: Connection,
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

    pub fn in_memory() -> Result<Self, StorageError> {
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
            transaction.pragma_update(None, "user_version", 2)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 2 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/003_server_records.sql"))?;
            transaction.pragma_update(None, "user_version", 3)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 3 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/004_server_tags.sql"))?;
            transaction.pragma_update(None, "user_version", 4)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 4 {
            let transaction = self.connection.transaction()?;
            transaction
                .execute_batch(include_str!("../migrations/005_server_custom_columns.sql"))?;
            transaction.pragma_update(None, "user_version", 5)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 5 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/006_planning.sql"))?;
            transaction.pragma_update(None, "user_version", 6)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 6 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/007_app_settings.sql"))?;
            transaction.pragma_update(None, "user_version", 7)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 7 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/008_work_items.sql"))?;
            transaction.pragma_update(None, "user_version", 8)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 8 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/009_work_scope.sql"))?;
            transaction.pragma_update(None, "user_version", 9)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 9 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/010_work_assets.sql"))?;
            transaction.pragma_update(None, "user_version", 10)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 10 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/011_impact_reviews.sql"))?;
            transaction.pragma_update(None, "user_version", 11)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 11 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/012_work_asset_origin.sql"))?;
            transaction.pragma_update(None, "user_version", 12)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 12 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/013_sites.sql"))?;
            transaction.pragma_update(None, "user_version", 13)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 13 {
            let transaction = self.connection.transaction()?;
            migrate_server_column_widths(&transaction)?;
            transaction.execute_batch(include_str!("../migrations/014_server_core_fields.sql"))?;
            transaction.pragma_update(None, "user_version", 14)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 14 {
            let transaction = self.connection.transaction()?;
            transaction
                .execute_batch(include_str!("../migrations/015_workspace_preferences.sql"))?;
            transaction.pragma_update(None, "user_version", 15)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 15 {
            self.automatic_backup("pre-schema-16.db")?;
            let transaction = self.connection.transaction()?;
            transaction
                .execute_batch(include_str!("../migrations/016_operational_inventory.sql"))?;
            transaction.pragma_update(None, "user_version", 16)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 16 {
            self.automatic_backup("pre-schema-17.db")?;
            let transaction = self.connection.transaction()?;
            transaction
                .execute_batch(include_str!("../migrations/017_inventory_verification.sql"))?;
            transaction.pragma_update(None, "user_version", 17)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 17 {
            self.automatic_backup("pre-schema-18.db")?;
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/018_server_endpoints.sql"))?;
            transaction.pragma_update(None, "user_version", 18)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 18 {
            self.automatic_backup("pre-schema-19.db")?;
            let transaction = self.connection.transaction()?;
            let ids = {
                let mut statement =
                    transaction.prepare("SELECT id FROM server_records ORDER BY id")?;
                statement
                    .query_map([], |row| row.get::<_, i64>(0))?
                    .collect::<Result<Vec<_>, _>>()?
            };
            for id in ids {
                crate::operational::seed_server_services(&transaction, id, false)?;
            }
            transaction.pragma_update(None, "user_version", 19)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 19 {
            self.automatic_backup("pre-schema-20.db")?;
            let transaction = self.connection.transaction()?;
            let has_inventory_metadata: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('operational_inventory') WHERE name='created_at')",
                [],
                |row| row.get(0),
            )?;
            // A test/backup can legitimately carry schema-20 objects while its
            // user_version is reset to 19. The migration is transactional, so
            // recognizing that complete shape is safe and avoids duplicate ALTERs.
            if !has_inventory_metadata {
                transaction.execute_batch(include_str!(
                    "../migrations/020_inventory_relationship_graph.sql"
                ))?;
            }
            transaction.pragma_update(None, "user_version", 20)?;
            transaction.commit()?;
        }
        if self.schema_version()? == 20 {
            self.automatic_backup("pre-schema-21.db")?;
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/021_asset_context.sql"))?;
            transaction.pragma_update(None, "user_version", 21)?;
            transaction.commit()?;
        }
        Ok(())
    }

    pub fn font_size_step(&self) -> Result<i64, StorageError> {
        let value = self.connection.query_row(
            "SELECT value FROM app_settings WHERE key='font_size_step'",
            [],
            |row| row.get::<_, String>(0),
        )?;
        Ok(value.parse::<i64>().unwrap_or(2).clamp(1, 3))
    }

    pub fn set_font_size_step(&self, step: i64) -> Result<(), StorageError> {
        self.set_setting("font_size_step", &step.clamp(1, 3).to_string())
    }

    fn setting(&self, key: &str) -> Option<String> {
        self.connection
            .query_row(
                "SELECT value FROM app_settings WHERE key=?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .ok()
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO app_settings (key,value,updated_at) VALUES (?1,?2,CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=CURRENT_TIMESTAMP",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn workspace_preferences(&self) -> WorkspacePreferences {
        let number = |key: &str, fallback: i64| {
            self.setting(key)
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(fallback)
        };
        let visible = self
            .setting("server_visible_columns")
            .and_then(|value| serde_json::from_str::<Vec<bool>>(&value).ok())
            .filter(|values| values.len() == 7)
            .unwrap_or_else(|| vec![true; 7]);
        WorkspacePreferences {
            theme_index: number("theme_index", 0).clamp(0, 4),
            font_size_step: number("font_size_step", 2).clamp(1, 3),
            table_density: number("table_density", 1).clamp(0, 2),
            show_cell_tooltips: number("show_cell_tooltips", 1) != 0,
            fit_columns_on_open: number("fit_columns_on_open", 0) != 0,
            site_fit_columns_on_open: number(
                "site_fit_columns_on_open",
                number("fit_columns_on_open", 0),
            ) != 0,
            server_default_sort: number("server_default_sort", 0).max(0),
            site_default_sort: number("site_default_sort", 0).clamp(0, 11),
            server_visible_columns: visible,
            server_default_ports: self
                .setting("server_default_ports")
                .unwrap_or_else(|| "22, 443".into()),
            server_default_tags: self.setting("server_default_tags").unwrap_or_default(),
            site_default_tags: self.setting("site_default_tags").unwrap_or_default(),
            import_duplicate_policy: ImportDuplicatePolicy::from_index(number(
                "import_duplicate_policy",
                0,
            )),
            backup_retention: usize::try_from(number("backup_retention", 7).clamp(1, 30))
                .unwrap_or(7),
        }
    }

    pub fn set_workspace_preferences(
        &self,
        preferences: &WorkspacePreferences,
    ) -> Result<(), StorageError> {
        let visible = if preferences.server_visible_columns.len() == 7 {
            preferences.server_visible_columns.clone()
        } else {
            vec![true; 7]
        };
        let entries = [
            (
                "theme_index",
                preferences.theme_index.clamp(0, 4).to_string(),
            ),
            (
                "font_size_step",
                preferences.font_size_step.clamp(1, 3).to_string(),
            ),
            (
                "table_density",
                preferences.table_density.clamp(0, 2).to_string(),
            ),
            (
                "show_cell_tooltips",
                i64::from(preferences.show_cell_tooltips).to_string(),
            ),
            (
                "fit_columns_on_open",
                i64::from(preferences.fit_columns_on_open).to_string(),
            ),
            (
                "site_fit_columns_on_open",
                i64::from(preferences.site_fit_columns_on_open).to_string(),
            ),
            (
                "server_default_sort",
                preferences.server_default_sort.max(0).to_string(),
            ),
            (
                "site_default_sort",
                preferences.site_default_sort.clamp(0, 11).to_string(),
            ),
            (
                "server_visible_columns",
                serde_json::to_string(&visible)
                    .map_err(|error| StorageError::InvalidImport(error.to_string()))?,
            ),
            (
                "server_default_ports",
                preferences.server_default_ports.trim().to_owned(),
            ),
            (
                "server_default_tags",
                preferences.server_default_tags.trim().to_owned(),
            ),
            (
                "site_default_tags",
                preferences.site_default_tags.trim().to_owned(),
            ),
            (
                "import_duplicate_policy",
                preferences.import_duplicate_policy.index().to_string(),
            ),
            (
                "backup_retention",
                preferences.backup_retention.clamp(1, 30).to_string(),
            ),
        ];
        let transaction = self.connection.unchecked_transaction()?;
        for (key, value) in entries {
            transaction.execute(
                "INSERT INTO app_settings(key,value,updated_at) VALUES(?1,?2,CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=CURRENT_TIMESTAMP",
                params![key, value],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn work_impact_view(&self, work_id: i64) -> Result<(i64, i64), StorageError> {
        let read = |key: String| -> i64 {
            self.connection
                .query_row(
                    "SELECT value FROM app_settings WHERE key=?1",
                    [key],
                    |row| row.get::<_, String>(0),
                )
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0)
        };
        Ok((
            read(format!("work_{work_id}_impact_depth")).clamp(1, 2),
            read(format!("work_{work_id}_impact_filter")).clamp(0, 5),
        ))
    }

    pub fn set_work_impact_view(
        &self,
        work_id: i64,
        depth: i64,
        filter: i64,
    ) -> Result<(), StorageError> {
        for (key, value) in [
            (format!("work_{work_id}_impact_depth"), depth.clamp(1, 2)),
            (format!("work_{work_id}_impact_filter"), filter.clamp(0, 5)),
        ] {
            self.connection.execute(
                "INSERT INTO app_settings (key,value,updated_at) VALUES (?1,?2,CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=CURRENT_TIMESTAMP",
                params![key, value.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn column_widths(&self) -> Result<Vec<f32>, StorageError> {
        let value = self.connection.query_row(
            "SELECT value FROM app_settings WHERE key='column_widths'",
            [],
            |row| row.get::<_, String>(0),
        );
        let mut widths = value
            .ok()
            .and_then(|value| serde_json::from_str::<Vec<f32>>(&value).ok())
            .unwrap_or_else(|| DEFAULT_COLUMN_WIDTHS.to_vec());
        for width in &mut widths {
            *width = width.clamp(96.0, 420.0);
        }
        if widths.len() < DEFAULT_COLUMN_WIDTHS.len() {
            widths.extend_from_slice(&DEFAULT_COLUMN_WIDTHS[widths.len()..]);
        }
        Ok(widths)
    }

    pub fn set_column_widths(&self, widths: &[f32]) -> Result<(), StorageError> {
        let widths = widths
            .iter()
            .map(|width| width.clamp(96.0, 420.0))
            .collect::<Vec<_>>();
        let value = serde_json::to_string(&widths)
            .map_err(|error| StorageError::InvalidImport(error.to_string()))?;
        self.connection.execute(
            "INSERT INTO app_settings (key,value,updated_at) VALUES ('column_widths',?1,CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=CURRENT_TIMESTAMP",
            [value],
        )?;
        Ok(())
    }

    pub fn project_plans(&self) -> Result<Vec<ProjectPlan>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT p.id, p.name, p.description, pp.start_at, pp.priority, pp.launch_at
             FROM projects p LEFT JOIN project_plans pp ON pp.project_id = p.id
             WHERE p.archived_at IS NULL ORDER BY p.position, p.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ProjectPlan {
                project_id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                start_at: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                priority: row
                    .get::<_, Option<String>>(4)?
                    .unwrap_or_else(|| "P2".into()),
                launch_at: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn operations(&self) -> Result<Vec<OperationRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, publish_at, media, channel FROM operations
             WHERE archived_at IS NULL ORDER BY position, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(OperationRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                publish_at: row.get(2)?,
                media: row.get(3)?,
                channel: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn work_items(&self) -> Result<Vec<WorkItem>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT w.id, w.title, w.kind, w.status, w.priority, w.owner, w.start_at, w.due_at, w.notes,
                    (SELECT COUNT(*) FROM checklist_items c WHERE c.work_item_id = w.id),
                    (SELECT COUNT(*) FROM checklist_items c WHERE c.work_item_id = w.id AND c.completed = 1),
                    (SELECT COUNT(*) FROM work_item_projects p WHERE p.work_item_id = w.id),
                    (SELECT COUNT(*) FROM work_item_servers s WHERE s.work_item_id = w.id),
                    (SELECT COUNT(*) FROM work_item_assets a WHERE a.work_item_id = w.id),
                    CASE
                      WHEN trim(w.due_at) = '' THEN 'none'
                      WHEN length(w.due_at) = 10 AND date(w.due_at) = w.due_at THEN
                        CASE
                          WHEN date(w.due_at) < date('now','localtime') THEN 'overdue'
                          WHEN date(w.due_at) = date('now','localtime') THEN 'today'
                          WHEN date(w.due_at) <= date('now','localtime','+7 days') THEN 'soon'
                          ELSE 'scheduled'
                        END
                      ELSE 'flexible'
                    END
             FROM work_items w
             WHERE w.archived_at IS NULL
             ORDER BY CASE w.status WHEN 'done' THEN 1 WHEN 'cancelled' THEN 2 ELSE 0 END, w.position, w.id",
        )?;
        let rows = statement.query_map([], |row| {
            let kind: String = row.get(2)?;
            let status: String = row.get(3)?;
            let due_state: String = row.get(14)?;
            Ok(WorkItem {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: WorkKind::parse(&kind).unwrap_or(WorkKind::Task),
                status: WorkStatus::parse(&status).unwrap_or(WorkStatus::Inbox),
                priority: row.get(4)?,
                owner: row.get(5)?,
                start_at: row.get(6)?,
                due_at: row.get(7)?,
                notes: row.get(8)?,
                checklist_total: row.get(9)?,
                checklist_completed: row.get(10)?,
                project_count: row.get(11)?,
                server_count: row.get(12)?,
                asset_count: row.get(13)?,
                due_state: DueState::parse(&due_state),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_work_item(
        &mut self,
        id: Option<i64>,
        draft: &WorkDraft,
    ) -> Result<i64, StorageError> {
        let title = validate_name(&draft.title, 200)?;
        let priority = validate_name(&draft.priority, 48)?;
        let owner = draft.owner.trim();
        let start_at = draft.start_at.trim();
        let due_at = draft.due_at.trim();
        let notes = draft.notes.trim();
        if draft.status == WorkStatus::Done {
            let Some(work_id) = id else {
                return Err(StorageError::InvalidImport(
                    "Create the Work and complete its checklist before marking Done".into(),
                ));
            };
            if self.incomplete_checklist_count(work_id)? > 0 {
                return Err(StorageError::InvalidImport(
                    "Complete every checklist item before marking Work Done".into(),
                ));
            }
        }
        if owner.chars().count() > 160
            || start_at.chars().count() > 80
            || due_at.chars().count() > 80
            || notes.chars().count() > 4_000
        {
            return Err(StorageError::InvalidImport(
                "Work details exceed the supported length".into(),
            ));
        }
        let result = if let Some(id) = id {
            self.connection.execute(
                "UPDATE work_items SET title=?1,kind=?2,status=?3,priority=?4,owner=?5,start_at=?6,due_at=?7,notes=?8,updated_at=CURRENT_TIMESTAMP
                 WHERE id=?9 AND archived_at IS NULL",
                params![
                    title,
                    draft.kind.key(),
                    draft.status.key(),
                    priority,
                    owner,
                    start_at,
                    due_at,
                    notes,
                    id
                ],
            )?;
            id
        } else {
            self.connection.execute(
                "INSERT INTO work_items (title,kind,status,priority,owner,start_at,due_at,notes,position)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,(SELECT COALESCE(MAX(position),-1)+1 FROM work_items))",
                params![
                    title,
                    draft.kind.key(),
                    draft.status.key(),
                    priority,
                    owner,
                    start_at,
                    due_at,
                    notes
                ],
            )?;
            self.connection.last_insert_rowid()
        };
        if id.is_none() {
            let items = match draft.kind {
                WorkKind::Renewal => [
                    "Confirm domains and servers",
                    "Deploy the renewed certificate",
                    "Verify every affected domain",
                ],
                WorkKind::Change => [
                    "Confirm the work scope",
                    "Apply the change",
                    "Verify the result",
                ],
                _ => [
                    "Define the done state",
                    "Complete the work",
                    "Record the result",
                ],
            };
            for (position, item) in items.iter().enumerate() {
                self.connection.execute(
                    "INSERT INTO checklist_items (work_item_id,title,position) VALUES (?1,?2,?3)",
                    params![result, item, position as i64],
                )?;
            }
        }
        Ok(result)
    }

    pub fn archive_work_item(&self, id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE work_items SET archived_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND archived_at IS NULL",
            [id],
        )?;
        Ok(())
    }

    pub fn set_work_status(&self, id: i64, status: WorkStatus) -> Result<(), StorageError> {
        if status == WorkStatus::Done && self.incomplete_checklist_count(id)? > 0 {
            return Err(StorageError::InvalidImport(
                "Complete every checklist item before marking Work Done".into(),
            ));
        }
        let changed = self.connection.execute(
            "UPDATE work_items SET status=?1,updated_at=CURRENT_TIMESTAMP
             WHERE id=?2 AND archived_at IS NULL",
            params![status.key(), id],
        )?;
        if changed != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }

    pub fn incomplete_checklist_count(&self, work_id: i64) -> Result<i64, StorageError> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM checklist_items WHERE work_item_id=?1 AND completed=0",
                [work_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn checklist_items(&self, work_id: i64) -> Result<Vec<ChecklistItem>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id,title,completed FROM checklist_items
             WHERE work_item_id=?1 ORDER BY position,id",
        )?;
        let rows = statement.query_map([work_id], |row| {
            Ok(ChecklistItem {
                id: row.get(0)?,
                title: row.get(1)?,
                completed: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn add_checklist_item(&self, work_id: i64, title: &str) -> Result<i64, StorageError> {
        let title = validate_name(title, 240)?;
        self.connection.execute(
            "INSERT INTO checklist_items(work_item_id,title,position)
             VALUES(?1,?2,(SELECT COALESCE(MAX(position),-1)+1 FROM checklist_items WHERE work_item_id=?1))",
            params![work_id, title],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn set_checklist_item_completed(
        &self,
        id: i64,
        completed: bool,
    ) -> Result<(), StorageError> {
        let changed = self.connection.execute(
            "UPDATE checklist_items SET completed=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            params![completed, id],
        )?;
        if changed != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }

    pub fn delete_checklist_item(&self, id: i64) -> Result<(), StorageError> {
        let changed = self
            .connection
            .execute("DELETE FROM checklist_items WHERE id=?1", [id])?;
        if changed != 1 {
            return Err(StorageError::Conflict);
        }
        Ok(())
    }

    pub fn move_checklist_item(&mut self, id: i64, direction: i64) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        let (work_id, position): (i64, i64) = transaction.query_row(
            "SELECT work_item_id,position FROM checklist_items WHERE id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let neighbor = if direction < 0 {
            transaction.query_row(
                "SELECT id,position FROM checklist_items
                 WHERE work_item_id=?1 AND (position < ?2 OR (position=?2 AND id < ?3))
                 ORDER BY position DESC,id DESC LIMIT 1",
                params![work_id, position, id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
        } else {
            transaction.query_row(
                "SELECT id,position FROM checklist_items
                 WHERE work_item_id=?1 AND (position > ?2 OR (position=?2 AND id > ?3))
                 ORDER BY position,id LIMIT 1",
                params![work_id, position, id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
        };
        if let Ok((neighbor_id, neighbor_position)) = neighbor {
            transaction.execute(
                "UPDATE checklist_items SET position=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                params![neighbor_position, id],
            )?;
            transaction.execute(
                "UPDATE checklist_items SET position=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                params![position, neighbor_id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn work_scope(
        &self,
        work_id: i64,
    ) -> Result<(Vec<WorkProjectChoice>, Vec<WorkServerChoice>), StorageError> {
        let mut projects = self.connection.prepare(
            "SELECT p.id, p.name, EXISTS(SELECT 1 FROM work_item_projects wp WHERE wp.work_item_id=?1 AND wp.project_id=p.id)
             FROM projects p WHERE p.archived_at IS NULL ORDER BY p.position, p.id",
        )?;
        let project_rows = projects.query_map([work_id], |row| {
            Ok(WorkProjectChoice {
                id: row.get(0)?,
                name: row.get(1)?,
                selected: row.get(2)?,
            })
        })?;
        let projects = project_rows.collect::<Result<Vec<_>, _>>()?;
        let mut servers = self.connection.prepare(
            "SELECT id, tags || ' · ' || ip_address, EXISTS(SELECT 1 FROM work_item_servers ws WHERE ws.work_item_id=?1 AND ws.server_id=server_records.id)
             FROM server_records WHERE archived_at IS NULL ORDER BY position, id",
        )?;
        let server_rows = servers.query_map([work_id], |row| {
            Ok(WorkServerChoice {
                id: row.get(0)?,
                label: row.get(1)?,
                selected: row.get(2)?,
            })
        })?;
        let servers = server_rows.collect::<Result<Vec<_>, _>>()?;
        Ok((projects, servers))
    }

    pub fn work_asset_scope(&self, work_id: i64) -> Result<Vec<WorkAssetChoice>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT a.id, p.name || ' · ' || a.name || ' (' || a.kind || ')',
                    EXISTS(SELECT 1 FROM work_item_assets wa WHERE wa.work_item_id=?1 AND wa.asset_id=a.id),
                    COALESCE((SELECT origin FROM work_item_assets wa WHERE wa.work_item_id=?1 AND wa.asset_id=a.id),'')
             FROM assets a JOIN projects p ON p.id=a.project_id
             WHERE a.archived_at IS NULL AND p.archived_at IS NULL
             ORDER BY p.position, p.id, a.position, a.id",
        )?;
        let rows = statement.query_map([work_id], |row| {
            Ok(WorkAssetChoice {
                id: row.get(0)?,
                label: row.get(1)?,
                selected: row.get(2)?,
                origin: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Returns one-hop relationship neighbors of the directly selected assets.
    /// The result is intentionally bounded, deduplicated, and stable so it can be
    /// shown as an explainable preview rather than silently changing saved scope.
    pub fn inferred_work_assets(
        &self,
        work_id: i64,
    ) -> Result<Vec<WorkImpactChoice>, StorageError> {
        let direct = self
            .connection
            .prepare("SELECT asset_id FROM work_item_assets WHERE work_item_id=?1")?
            .query_map([work_id], |row| row.get::<_, i64>(0))?
            .collect::<Result<HashSet<_>, _>>()?;
        if direct.is_empty() {
            return Ok(Vec::new());
        }
        let mut statement = self.connection.prepare(
            "SELECT r.source_asset_id, r.target_asset_id, r.kind,
                    source.name, target.name, source.kind, target.kind,
                    sp.name, tp.name
             FROM relationships r
             JOIN assets source ON source.id=r.source_asset_id
             JOIN assets target ON target.id=r.target_asset_id
             JOIN projects sp ON sp.id=source.project_id
             JOIN projects tp ON tp.id=target.project_id
             WHERE r.archived_at IS NULL AND source.archived_at IS NULL
               AND target.archived_at IS NULL AND sp.archived_at IS NULL AND tp.archived_at IS NULL
             ORDER BY source.name COLLATE NOCASE, target.name COLLATE NOCASE, r.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;
        let mut inferred = HashMap::<i64, WorkImpactChoice>::new();
        for row in rows {
            let (
                source_id,
                target_id,
                kind,
                source_name,
                target_name,
                source_kind,
                target_kind,
                source_project,
                target_project,
            ) = row?;
            let (neighbor_id, label, category, reason) =
                if direct.contains(&source_id) && !direct.contains(&target_id) {
                    (
                        target_id,
                        format!("{target_project} · {target_name} ({target_kind})"),
                        display_resource_kind(&target_kind),
                        format!(
                            "via {source_name} · {}",
                            parse_relationship_kind(&kind).label()
                        ),
                    )
                } else if direct.contains(&target_id) && !direct.contains(&source_id) {
                    (
                        source_id,
                        format!("{source_project} · {source_name} ({source_kind})"),
                        display_resource_kind(&source_kind),
                        format!(
                            "via {target_name} · {}",
                            parse_relationship_kind(&kind).label()
                        ),
                    )
                } else {
                    continue;
                };
            inferred.entry(neighbor_id).or_insert(WorkImpactChoice {
                id: neighbor_id,
                label,
                category,
                reason,
            });
        }
        let mut result = inferred.into_values().collect::<Vec<_>>();
        let excluded = self.excluded_impact_ids(work_id)?;
        result.retain(|item| !excluded.contains(&item.id));
        result.sort_by_key(|item| (item.category.clone(), item.label.to_lowercase(), item.id));
        result.truncate(MAX_INFERRED_IMPACTS);
        Ok(result)
    }

    pub fn inferred_work_assets_with_depth(
        &self,
        work_id: i64,
        depth: i64,
    ) -> Result<Vec<WorkImpactChoice>, StorageError> {
        let depth = depth.clamp(1, 2);
        let ids = self
            .connection
            .prepare(
                "WITH RECURSIVE reach(asset_id, hops) AS (
                    SELECT wa.asset_id, 0
                    FROM work_item_assets wa
                    JOIN assets seed ON seed.id=wa.asset_id
                    JOIN projects seed_project ON seed_project.id=seed.project_id
                    WHERE wa.work_item_id=?1 AND seed.archived_at IS NULL
                      AND seed_project.archived_at IS NULL
                    UNION
                    SELECT CASE WHEN r.source_asset_id = reach.asset_id THEN r.target_asset_id ELSE r.source_asset_id END, reach.hops + 1
                    FROM reach
                    JOIN relationships r ON r.source_asset_id = reach.asset_id OR r.target_asset_id = reach.asset_id
                    JOIN assets neighbor ON neighbor.id=CASE WHEN r.source_asset_id=reach.asset_id THEN r.target_asset_id ELSE r.source_asset_id END
                    JOIN projects neighbor_project ON neighbor_project.id=neighbor.project_id
                    WHERE reach.hops < ?2 AND r.archived_at IS NULL
                      AND neighbor.archived_at IS NULL AND neighbor_project.archived_at IS NULL
                )
                SELECT asset_id, MIN(hops) FROM reach WHERE hops > 0 GROUP BY asset_id ORDER BY asset_id",
            )?
            .query_map(params![work_id, depth], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let direct = self
            .connection
            .prepare("SELECT asset_id FROM work_item_assets WHERE work_item_id=?1")?
            .query_map([work_id], |row| row.get::<_, i64>(0))?
            .collect::<Result<HashSet<_>, _>>()?;
        let mut result = Vec::new();
        let excluded = self.excluded_impact_ids(work_id)?;
        for (id, hops) in ids {
            if direct.contains(&id) || excluded.contains(&id) {
                continue;
            }
            let item = self.connection.query_row(
                "SELECT p.name, a.name, a.kind FROM assets a JOIN projects p ON p.id=a.project_id
                 WHERE a.id=?1 AND a.archived_at IS NULL AND p.archived_at IS NULL",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            );
            let Ok((project, name, kind)) = item else {
                continue;
            };
            result.push(WorkImpactChoice {
                id,
                label: format!("{project} · {name} ({kind})"),
                category: display_resource_kind(&kind),
                reason: format!(
                    "within {hops} relationship hop{}",
                    if hops == 1 { "" } else { "s" }
                ),
            });
        }
        result.sort_by_key(|item| (item.category.clone(), item.label.to_lowercase(), item.id));
        result.truncate(MAX_INFERRED_IMPACTS);
        Ok(result)
    }

    pub fn inferred_work_asset_total_with_depth(
        &self,
        work_id: i64,
        depth: i64,
    ) -> Result<i64, StorageError> {
        let depth = depth.clamp(1, 2);
        self.connection
            .query_row(
                "WITH RECURSIVE reach(asset_id,hops) AS (
                    SELECT wa.asset_id,0
                    FROM work_item_assets wa
                    JOIN assets seed ON seed.id=wa.asset_id
                    JOIN projects seed_project ON seed_project.id=seed.project_id
                    WHERE wa.work_item_id=?1 AND seed.archived_at IS NULL
                      AND seed_project.archived_at IS NULL
                    UNION
                    SELECT CASE WHEN r.source_asset_id=reach.asset_id THEN r.target_asset_id ELSE r.source_asset_id END,reach.hops+1
                    FROM reach
                    JOIN relationships r ON r.source_asset_id=reach.asset_id OR r.target_asset_id=reach.asset_id
                    JOIN assets neighbor ON neighbor.id=CASE WHEN r.source_asset_id=reach.asset_id THEN r.target_asset_id ELSE r.source_asset_id END
                    JOIN projects neighbor_project ON neighbor_project.id=neighbor.project_id
                    WHERE reach.hops < ?2 AND r.archived_at IS NULL
                      AND neighbor.archived_at IS NULL AND neighbor_project.archived_at IS NULL
                 )
                 SELECT COUNT(DISTINCT reach.asset_id)
                 FROM reach
                 JOIN assets a ON a.id=reach.asset_id
                 JOIN projects p ON p.id=a.project_id
                 WHERE reach.hops>0 AND a.archived_at IS NULL AND p.archived_at IS NULL
                   AND NOT EXISTS (SELECT 1 FROM work_item_assets wa WHERE wa.work_item_id=?1 AND wa.asset_id=reach.asset_id)
                   AND NOT EXISTS (SELECT 1 FROM work_item_impact_reviews wr WHERE wr.work_item_id=?1 AND wr.asset_id=reach.asset_id AND wr.decision='excluded')",
                params![work_id, depth],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    fn excluded_impact_ids(&self, work_id: i64) -> Result<HashSet<i64>, StorageError> {
        self.connection
            .prepare(
                "SELECT asset_id FROM work_item_impact_reviews
                 WHERE work_item_id=?1 AND decision='excluded'",
            )?
            .query_map([work_id], |row| row.get(0))?
            .collect::<Result<HashSet<_>, _>>()
            .map_err(Into::into)
    }

    pub fn excluded_impact_count(&self, work_id: i64) -> Result<i64, StorageError> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM work_item_impact_reviews
                 WHERE work_item_id=?1 AND decision='excluded'",
                [work_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn exclude_work_impact(&self, work_id: i64, asset_id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO work_item_impact_reviews(work_item_id,asset_id,decision)
             VALUES(?1,?2,'excluded')
             ON CONFLICT(work_item_id,asset_id) DO UPDATE SET decision='excluded',created_at=CURRENT_TIMESTAMP",
            params![work_id, asset_id],
        )?;
        Ok(())
    }

    pub fn reset_excluded_work_impacts(&self, work_id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "DELETE FROM work_item_impact_reviews WHERE work_item_id=?1",
            [work_id],
        )?;
        Ok(())
    }

    pub fn work_health(&self, work_id: i64) -> Result<WorkHealth, StorageError> {
        let (status, owner, due_at, projects, servers, assets): (
            String,
            String,
            String,
            i64,
            i64,
            i64,
        ) = self.connection.query_row(
            "SELECT w.status, w.owner, w.due_at,
                        (SELECT COUNT(*) FROM work_item_projects WHERE work_item_id=w.id),
                        (SELECT COUNT(*) FROM work_item_servers WHERE work_item_id=w.id),
                        (SELECT COUNT(*) FROM work_item_assets WHERE work_item_id=w.id)
                 FROM work_items w WHERE w.id=?1 AND w.archived_at IS NULL",
            [work_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )?;
        let mut warnings = Vec::new();
        let mut action = None;
        if projects + servers + assets == 0 {
            warnings.push("Add a project or resource scope".into());
            action = Some(WorkHealthAction::ReviewScope);
        } else if projects > 0 && servers + assets == 0 {
            warnings.push("Projects selected without concrete resources".into());
            action = Some(WorkHealthAction::ReviewScope);
        }
        let outside_projects: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM work_item_assets wa JOIN assets a ON a.id=wa.asset_id
             WHERE wa.work_item_id=?1 AND NOT EXISTS (
                 SELECT 1 FROM work_item_projects wp
                 WHERE wp.work_item_id=wa.work_item_id AND wp.project_id=a.project_id
             )",
            [work_id],
            |row| row.get(0),
        )?;
        if projects > 0 && outside_projects > 0 {
            warnings.push(format!(
                "{outside_projects} resource(s) sit outside selected projects"
            ));
            action.get_or_insert(WorkHealthAction::ReviewScope);
        }
        let inferred = self.inferred_work_assets(work_id)?.len();
        if inferred > 0 {
            warnings.push(format!("{inferred} potential impact(s) need review"));
            action.get_or_insert(WorkHealthAction::ReviewImpact);
        }
        if status == WorkStatus::Ready.key() && owner.trim().is_empty() {
            warnings.push("Ready work has no owner".into());
            action.get_or_insert(WorkHealthAction::AddOwner);
        }
        if status == WorkStatus::Ready.key() && due_at.trim().is_empty() {
            warnings.push("Ready work has no due date".into());
            action.get_or_insert(WorkHealthAction::AddDueDate);
        }
        Ok(WorkHealth { warnings, action })
    }

    pub fn set_work_scope(
        &mut self,
        work_id: i64,
        project_ids: &[i64],
        server_ids: &[i64],
    ) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM work_item_projects WHERE work_item_id=?1",
            [work_id],
        )?;
        transaction.execute(
            "DELETE FROM work_item_servers WHERE work_item_id=?1",
            [work_id],
        )?;
        for id in project_ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
        {
            transaction.execute(
                "INSERT INTO work_item_projects(work_item_id,project_id) VALUES(?1,?2)",
                params![work_id, id],
            )?;
        }
        for id in server_ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
        {
            transaction.execute(
                "INSERT INTO work_item_servers(work_item_id,server_id) VALUES(?1,?2)",
                params![work_id, id],
            )?;
        }
        transaction.execute(
            "UPDATE work_items SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [work_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_work_asset_scope(
        &mut self,
        work_id: i64,
        asset_ids: &[i64],
    ) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        let existing_origins = transaction
            .prepare("SELECT asset_id,origin FROM work_item_assets WHERE work_item_id=?1")?
            .query_map([work_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;
        transaction.execute(
            "DELETE FROM work_item_assets WHERE work_item_id=?1",
            [work_id],
        )?;
        for id in asset_ids.iter().copied().collect::<HashSet<_>>() {
            transaction.execute(
                "INSERT INTO work_item_assets(work_item_id,asset_id,origin) VALUES(?1,?2,?3)",
                params![
                    work_id,
                    id,
                    existing_origins.get(&id).map_or("direct", String::as_str)
                ],
            )?;
            transaction.execute(
                "DELETE FROM work_item_impact_reviews WHERE work_item_id=?1 AND asset_id=?2",
                params![work_id, id],
            )?;
        }
        transaction.execute(
            "UPDATE work_items SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [work_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn add_work_asset_to_scope(
        &mut self,
        work_id: i64,
        asset_id: i64,
    ) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        let exists: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM assets WHERE id=?1 AND archived_at IS NULL",
            [asset_id],
            |row| row.get(0),
        )?;
        if exists != 1 {
            return Err(StorageError::Conflict);
        }
        transaction.execute(
            "INSERT OR IGNORE INTO work_item_assets(work_item_id,asset_id,origin) VALUES(?1,?2,'manual')",
            params![work_id, asset_id],
        )?;
        transaction.execute(
            "DELETE FROM work_item_impact_reviews WHERE work_item_id=?1 AND asset_id=?2",
            params![work_id, asset_id],
        )?;
        transaction.execute(
            "UPDATE work_items SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [work_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn add_all_inferred_work_assets(&mut self, work_id: i64) -> Result<usize, StorageError> {
        self.add_all_inferred_work_assets_with_depth(work_id, 1)
    }

    pub fn add_all_inferred_work_assets_with_depth(
        &mut self,
        work_id: i64,
        depth: i64,
    ) -> Result<usize, StorageError> {
        self.add_inferred_work_assets_with_depth_and_category(work_id, depth, None)
    }

    pub fn add_inferred_work_assets_with_depth_and_category(
        &mut self,
        work_id: i64,
        depth: i64,
        category: Option<&str>,
    ) -> Result<usize, StorageError> {
        let inferred = self.inferred_work_assets_with_depth(work_id, depth)?;
        let transaction = self.connection.transaction()?;
        let mut added = 0;
        for item in inferred {
            if category.is_some_and(|expected| item.category != expected) {
                continue;
            }
            added += transaction.execute(
                "INSERT OR IGNORE INTO work_item_assets(work_item_id,asset_id,origin) VALUES(?1,?2,'manual')",
                params![work_id, item.id],
            )?;
            transaction.execute(
                "DELETE FROM work_item_impact_reviews WHERE work_item_id=?1 AND asset_id=?2",
                params![work_id, item.id],
            )?;
        }
        if added > 0 {
            transaction.execute(
                "UPDATE work_items SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
                [work_id],
            )?;
        }
        transaction.commit()?;
        Ok(added)
    }

    pub fn priority_options(&self) -> Result<Vec<PriorityOption>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT label, color FROM priority_options ORDER BY position, id")?;
        let rows = statement.query_map([], |row| {
            Ok(PriorityOption {
                label: row.get(0)?,
                color: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn media_options(&self) -> Result<Vec<String>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT label FROM media_options ORDER BY position, id")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn resource_options(
        &self,
        kind: &str,
        include_hidden: bool,
    ) -> Result<Vec<ManagedOption>, StorageError> {
        validate_option_kind(kind)?;
        let sql = if include_hidden {
            "SELECT id,kind,label,semantic,color,is_default,hidden_at IS NOT NULL
             FROM resource_options WHERE kind=?1 ORDER BY hidden_at IS NOT NULL,position,id"
        } else {
            "SELECT id,kind,label,semantic,color,is_default,0
             FROM resource_options WHERE kind=?1 AND hidden_at IS NULL ORDER BY position,id"
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map([kind], |row| {
            Ok(ManagedOption {
                id: row.get(0)?,
                kind: row.get(1)?,
                label: row.get(2)?,
                semantic: row.get(3)?,
                color: row.get(4)?,
                is_default: row.get::<_, i64>(5)? != 0,
                hidden: row.get::<_, i64>(6)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_resource_option(
        &mut self,
        id: Option<i64>,
        kind: &str,
        label: &str,
        semantic: &str,
        color: &str,
        make_default: bool,
    ) -> Result<i64, StorageError> {
        validate_option_kind(kind)?;
        let label = validate_name(label, 64)?;
        validate_option_semantic(semantic)?;
        validate_color_token(color)?;
        let transaction = self.connection.transaction()?;
        let saved_id = if let Some(id) = id {
            let old_label: String = transaction.query_row(
                "SELECT label FROM resource_options WHERE id=?1 AND kind=?2",
                params![id, kind],
                |row| row.get(0),
            )?;
            transaction.execute(
                "UPDATE resource_options SET label=?1,semantic=?2,color=?3,hidden_at=NULL,updated_at=CURRENT_TIMESTAMP
                 WHERE id=?4 AND kind=?5",
                params![label, semantic, color, id, kind],
            )?;
            if !old_label.eq_ignore_ascii_case(&label) {
                let column = if kind == "environment" {
                    "environment"
                } else {
                    "status"
                };
                transaction.execute(
                    &format!(
                        "UPDATE server_records SET {column}=?1,updated_at=CURRENT_TIMESTAMP WHERE {column}=?2 COLLATE NOCASE"
                    ),
                    params![label, old_label],
                )?;
                transaction.execute(
                    &format!(
                        "UPDATE sites SET {column}=?1,updated_at=CURRENT_TIMESTAMP WHERE {column}=?2 COLLATE NOCASE"
                    ),
                    params![label, old_label],
                )?;
            }
            id
        } else {
            transaction.execute(
                "INSERT INTO resource_options(kind,label,semantic,color,position)
                 VALUES(?1,?2,?3,?4,(SELECT COALESCE(MAX(position),-1)+1 FROM resource_options WHERE kind=?1))",
                params![kind, label, semantic, color],
            )?;
            transaction.last_insert_rowid()
        };
        if make_default {
            transaction.execute(
                "UPDATE resource_options SET is_default=CASE WHEN id=?1 THEN 1 ELSE 0 END,
                 hidden_at=CASE WHEN id=?1 THEN NULL ELSE hidden_at END,updated_at=CURRENT_TIMESTAMP
                 WHERE kind=?2",
                params![saved_id, kind],
            )?;
        }
        transaction.commit()?;
        Ok(saved_id)
    }

    pub fn hide_resource_option(&mut self, id: i64) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        let (kind, label, is_default): (String, String, i64) = transaction.query_row(
            "SELECT kind,label,is_default FROM resource_options WHERE id=?1 AND hidden_at IS NULL",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let column = if kind == "environment" {
            "environment"
        } else {
            "status"
        };
        let server_count: i64 = transaction.query_row(
            &format!("SELECT COUNT(*) FROM server_records WHERE {column}=?1 COLLATE NOCASE"),
            [label.as_str()],
            |row| row.get(0),
        )?;
        let site_count: i64 = transaction.query_row(
            &format!("SELECT COUNT(*) FROM sites WHERE {column}=?1 COLLATE NOCASE"),
            [label.as_str()],
            |row| row.get(0),
        )?;
        if server_count + site_count > 0 {
            return Err(StorageError::InvalidImport(format!(
                "{label} is still used by {} records. Change those records before hiding it",
                server_count + site_count
            )));
        }
        if is_default != 0 {
            return Err(StorageError::InvalidImport(
                "Choose another default before hiding this option".into(),
            ));
        }
        transaction.execute(
            "UPDATE resource_options SET hidden_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn restore_resource_option(&self, id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE resource_options SET hidden_at=NULL,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [id],
        )?;
        Ok(())
    }

    pub fn managed_tags(&self, include_hidden: bool) -> Result<Vec<ManagedTag>, StorageError> {
        let sql = if include_hidden {
            "SELECT id,label,color,hidden_at IS NOT NULL FROM managed_tags
             ORDER BY hidden_at IS NOT NULL,position,id"
        } else {
            "SELECT id,label,color,0 FROM managed_tags WHERE hidden_at IS NULL ORDER BY position,id"
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map([], |row| {
            Ok(ManagedTag {
                id: row.get(0)?,
                label: row.get(1)?,
                color: row.get(2)?,
                hidden: row.get::<_, i64>(3)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_managed_tag(
        &mut self,
        id: Option<i64>,
        label: &str,
        color: &str,
    ) -> Result<i64, StorageError> {
        let label = validate_name(label, 64)?;
        validate_color_token(color)?;
        if let Some(id) = id {
            self.connection.execute(
                "UPDATE managed_tags SET label=?1,color=?2,hidden_at=NULL,updated_at=CURRENT_TIMESTAMP WHERE id=?3",
                params![label, color, id],
            )?;
            Ok(id)
        } else {
            self.connection.execute(
                "INSERT INTO managed_tags(label,color,position)
                 VALUES(?1,?2,(SELECT COALESCE(MAX(position),-1)+1 FROM managed_tags))",
                params![label, color],
            )?;
            Ok(self.connection.last_insert_rowid())
        }
    }

    pub fn set_managed_tag_hidden(&self, id: i64, hidden: bool) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE managed_tags SET hidden_at=CASE WHEN ?1 THEN CURRENT_TIMESTAMP ELSE NULL END,
             updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            params![hidden, id],
        )?;
        Ok(())
    }

    pub fn tag_suggestions(&self) -> Result<Vec<String>, StorageError> {
        let mut tags = self
            .managed_tags(false)?
            .into_iter()
            .map(|tag| tag.label)
            .collect::<Vec<_>>();
        for value in self
            .connection
            .prepare(
                "SELECT tags FROM server_records WHERE archived_at IS NULL
                 UNION ALL SELECT tags FROM sites WHERE archived_at IS NULL",
            )?
            .query_map([], |row| row.get::<_, String>(0))?
        {
            tags.extend(
                value?
                    .split(',')
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .map(str::to_owned),
            );
        }
        tags.sort_by_key(|tag| tag.to_lowercase());
        tags.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        Ok(tags)
    }

    pub fn add_priority_option(&mut self, label: &str, color: &str) -> Result<(), StorageError> {
        let label = validate_name(label, 48)?;
        self.connection
            .execute(
                "INSERT INTO priority_options (label, color, position)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position), -1) + 1 FROM priority_options))",
                params![label, color.trim()],
            )
            .map(|_| ())
            .map_err(map_write_error)
    }

    pub fn remove_priority_option(&mut self, label: &str) -> Result<(), StorageError> {
        self.connection
            .execute(
                "DELETE FROM priority_options WHERE label = ?1
                 AND (SELECT COUNT(*) FROM priority_options) > 1",
                [label],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn add_media_option(&mut self, label: &str) -> Result<(), StorageError> {
        let label = validate_name(label, 48)?;
        self.connection
            .execute(
                "INSERT INTO media_options (label, position)
                 VALUES (?1, (SELECT COALESCE(MAX(position), -1) + 1 FROM media_options))",
                [label],
            )
            .map(|_| ())
            .map_err(map_write_error)
    }

    pub fn remove_media_option(&mut self, label: &str) -> Result<(), StorageError> {
        self.connection
            .execute(
                "DELETE FROM media_options WHERE label = ?1
                 AND (SELECT COUNT(*) FROM media_options) > 1",
                [label],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn site_records(&self) -> Result<Vec<SiteRecord>, StorageError> {
        self.site_records_by_visibility(false)
    }

    pub fn hidden_site_records(&self) -> Result<Vec<SiteRecord>, StorageError> {
        self.site_records_by_visibility(true)
    }

    fn site_records_by_visibility(&self, hidden: bool) -> Result<Vec<SiteRecord>, StorageError> {
        let sql = if hidden {
            "SELECT id,name,url,environment,status,purpose,tags FROM sites
             WHERE archived_at IS NOT NULL ORDER BY position,id"
        } else {
            "SELECT id,name,url,environment,status,purpose,tags FROM sites
             WHERE archived_at IS NULL ORDER BY position,id"
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map([], |row| {
            Ok(SiteRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                environment: row.get(3)?,
                status: row.get(4)?,
                purpose: row.get(5)?,
                tags: row
                    .get::<_, String>(6)?
                    .split(',')
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .map(str::to_owned)
                    .collect(),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_site(&mut self, id: Option<i64>, draft: &SiteDraft) -> Result<i64, StorageError> {
        let draft = validate_site(draft).map_err(ValidationError::from)?;
        let tags = draft.tags.join(", ");
        if let Some(id) = id {
            self.connection.execute(
                "UPDATE sites SET name=?1,url=?2,environment=?3,status=?4,purpose=?5,tags=?6,updated_at=CURRENT_TIMESTAMP
                 WHERE id=?7 AND archived_at IS NULL",
                params![draft.name, draft.url, draft.environment, draft.status, draft.purpose, tags, id],
            )?;
            Ok(id)
        } else {
            self.connection.execute(
                "INSERT INTO sites(name,url,environment,status,purpose,tags,position)
                 VALUES(?1,?2,?3,?4,?5,?6,(SELECT COALESCE(MAX(position),-1)+1 FROM sites))",
                params![
                    draft.name,
                    draft.url,
                    draft.environment,
                    draft.status,
                    draft.purpose,
                    tags
                ],
            )?;
            Ok(self.connection.last_insert_rowid())
        }
    }

    pub fn update_site_cell(
        &mut self,
        site_id: i64,
        column_index: usize,
        value: &str,
    ) -> Result<(), StorageError> {
        let current = self.connection.query_row(
            "SELECT name,url,environment,status,purpose,tags FROM sites WHERE id=?1",
            [site_id],
            |row| {
                Ok(SiteDraft {
                    name: row.get(0)?,
                    url: row.get(1)?,
                    environment: row.get(2)?,
                    status: row.get(3)?,
                    purpose: row.get(4)?,
                    tags: row
                        .get::<_, String>(5)?
                        .split(',')
                        .map(str::to_owned)
                        .collect(),
                })
            },
        )?;
        let mut draft = current;
        match column_index {
            0 => draft.name = value.to_owned(),
            1 => draft.url = value.to_owned(),
            2 => draft.environment = value.to_owned(),
            3 => draft.status = value.to_owned(),
            4 => draft.purpose = value.to_owned(),
            5 => draft.tags = value.split(',').map(str::to_owned).collect(),
            _ => return Err(StorageError::InvalidImport("Unknown site column".into())),
        }
        self.save_site(Some(site_id), &draft).map(|_| ())
    }

    pub fn hide_site(&self, id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE sites SET archived_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP
             WHERE id=?1 AND archived_at IS NULL",
            [id],
        )?;
        Ok(())
    }

    pub fn restore_site(&self, id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE sites SET archived_at=NULL,updated_at=CURRENT_TIMESTAMP
             WHERE id=?1 AND archived_at IS NOT NULL",
            [id],
        )?;
        Ok(())
    }

    pub fn import_sites(
        &mut self,
        data: &TabularData,
        mappings: &[SiteImportTarget],
        defaults: &SiteImportDefaults,
    ) -> Result<usize, StorageError> {
        if mappings.len() != data.headers.len() {
            return Err(StorageError::InvalidImport(
                "Every source column needs a mapping".into(),
            ));
        }
        let name_count = mappings
            .iter()
            .filter(|target| **target == SiteImportTarget::Name)
            .count();
        let url_count = mappings
            .iter()
            .filter(|target| **target == SiteImportTarget::Url)
            .count();
        if name_count != 1 || url_count != 1 {
            return Err(StorageError::InvalidImport(
                "Map exactly one Name column and one URL column".into(),
            ));
        }
        for target in [
            SiteImportTarget::Environment,
            SiteImportTarget::Status,
            SiteImportTarget::Purpose,
            SiteImportTarget::Tags,
        ] {
            if mappings.iter().filter(|item| **item == target).count() > 1 {
                return Err(StorageError::InvalidImport(
                    "Each site field can only be mapped once".into(),
                ));
            }
        }
        let transaction = self.connection.transaction()?;
        let mut changed = 0;
        for (row_index, row) in data.rows.iter().enumerate() {
            let mut name = String::new();
            let mut url = String::new();
            let mut environment = String::new();
            let mut status = String::new();
            let mut purpose = String::new();
            let mut tags = String::new();
            for (index, target) in mappings.iter().enumerate() {
                let value = row.get(index).map_or("", String::as_str).trim();
                match target {
                    SiteImportTarget::Skip => {}
                    SiteImportTarget::Name => name = value.to_owned(),
                    SiteImportTarget::Url => url = value.to_owned(),
                    SiteImportTarget::Environment => environment = value.to_owned(),
                    SiteImportTarget::Status => status = value.to_owned(),
                    SiteImportTarget::Purpose => purpose = value.to_owned(),
                    SiteImportTarget::Tags => tags = value.to_owned(),
                }
            }
            if environment.is_empty() {
                environment = defaults.environment.clone();
            }
            if status.is_empty() {
                status = defaults.status.clone();
            }
            if purpose.is_empty() {
                purpose = defaults.purpose.clone();
            }
            if tags.is_empty() {
                tags = defaults.tags.clone();
            }
            let draft = validate_site(&SiteDraft {
                name,
                url,
                environment,
                status,
                purpose,
                tags: tags.split(',').map(str::to_owned).collect(),
            })
            .map_err(|error| {
                StorageError::InvalidImport(format!("Row {}: {error}", row_index + 2))
            })?;
            let existing_id = transaction
                .query_row(
                    "SELECT id FROM sites WHERE lower(trim(url))=lower(trim(?1)) AND archived_at IS NULL ORDER BY id LIMIT 1",
                    [draft.url.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            match (defaults.duplicate_policy, existing_id) {
                (ImportDuplicatePolicy::Skip, Some(_)) => continue,
                (ImportDuplicatePolicy::UpdateExisting, Some(id)) => {
                    transaction.execute(
                        "UPDATE sites SET name=?1,url=?2,environment=?3,status=?4,purpose=?5,tags=?6,updated_at=CURRENT_TIMESTAMP WHERE id=?7",
                        params![draft.name, draft.url, draft.environment, draft.status, draft.purpose, draft.tags.join(", "), id],
                    )?;
                }
                _ => {
                    transaction.execute(
                        "INSERT INTO sites(name,url,environment,status,purpose,tags,position)
                         VALUES(?1,?2,?3,?4,?5,?6,(SELECT COALESCE(MAX(position),-1)+1 FROM sites))",
                        params![draft.name, draft.url, draft.environment, draft.status, draft.purpose, draft.tags.join(", ")],
                    )?;
                }
            }
            changed += 1;
        }
        transaction.commit()?;
        Ok(changed)
    }

    pub fn server_records(&self) -> Result<Vec<ServerRecord>, StorageError> {
        self.server_records_by_visibility(false)
    }

    pub fn hidden_server_records(&self) -> Result<Vec<ServerRecord>, StorageError> {
        self.server_records_by_visibility(true)
    }

    fn server_records_by_visibility(
        &self,
        hidden: bool,
    ) -> Result<Vec<ServerRecord>, StorageError> {
        let columns = self.server_columns()?;
        let records_sql = if hidden {
            "SELECT id,name,tags,ip_address,ports,environment,status,purpose FROM server_records
             WHERE archived_at IS NOT NULL ORDER BY position, id"
        } else {
            "SELECT id,name,tags,ip_address,ports,environment,status,purpose FROM server_records
             WHERE archived_at IS NULL ORDER BY position, id"
        };
        let mut statement = self.connection.prepare(records_sql)?;
        let rows = statement.query_map([], |row| {
            Ok(ServerRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                tags: row
                    .get::<_, String>(2)?
                    .split(',')
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .map(str::to_owned)
                    .collect(),
                ip_address: row.get(3)?,
                ports: row.get(4)?,
                environment: row.get(5)?,
                status: row.get(6)?,
                purpose: row.get(7)?,
                custom_values: Vec::new(),
            })
        })?;
        let mut records = rows.collect::<Result<Vec<_>, _>>()?;
        let mut values = HashMap::new();
        let values_sql = if hidden {
            "SELECT v.server_id,v.column_id,v.value FROM server_custom_values v
             JOIN server_records s ON s.id=v.server_id WHERE s.archived_at IS NOT NULL"
        } else {
            "SELECT v.server_id,v.column_id,v.value FROM server_custom_values v
             JOIN server_records s ON s.id=v.server_id WHERE s.archived_at IS NULL"
        };
        let mut value_statement = self.connection.prepare(values_sql)?;
        let value_rows = value_statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for value in value_rows {
            let (server_id, column_id, value) = value?;
            values.insert((server_id, column_id), value);
        }
        for record in &mut records {
            record.custom_values = columns
                .iter()
                .map(|column| values.remove(&(record.id, column.id)).unwrap_or_default())
                .collect();
        }
        Ok(records)
    }

    pub fn server_columns(&self) -> Result<Vec<ServerColumn>, StorageError> {
        self.server_columns_by_visibility(false)
    }

    pub fn all_server_columns(&self) -> Result<Vec<ServerColumn>, StorageError> {
        self.server_columns_by_visibility(true)
    }

    fn server_columns_by_visibility(
        &self,
        include_hidden: bool,
    ) -> Result<Vec<ServerColumn>, StorageError> {
        let sql = if include_hidden {
            "SELECT id,name,value_type,default_value,hidden_at IS NOT NULL
             FROM server_custom_columns ORDER BY hidden_at IS NOT NULL,position,id"
        } else {
            "SELECT id,name,value_type,default_value,0
             FROM server_custom_columns WHERE hidden_at IS NULL ORDER BY position,id"
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map([], |row| {
            Ok(ServerColumn {
                id: row.get(0)?,
                name: row.get(1)?,
                value_type: row.get(2)?,
                default_value: row.get(3)?,
                hidden: row.get::<_, i64>(4)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_server_column(
        &mut self,
        id: Option<i64>,
        name: &str,
        value_type: &str,
        default_value: &str,
    ) -> Result<i64, StorageError> {
        let name = validate_name(name, 64)?;
        validate_custom_value(value_type, default_value)?;
        if let Some(id) = id {
            let invalid_count: i64 = self.connection.query_row(
                "SELECT COUNT(*) FROM server_custom_values WHERE column_id=?1 AND trim(value)<>''",
                [id],
                |row| row.get(0),
            )?;
            if invalid_count > 0 {
                let mut statement = self.connection.prepare(
                    "SELECT value FROM server_custom_values WHERE column_id=?1 AND trim(value)<>''",
                )?;
                let values = statement.query_map([id], |row| row.get::<_, String>(0))?;
                for value in values {
                    validate_custom_value(value_type, &value?)?;
                }
            }
            self.connection.execute(
                "UPDATE server_custom_columns SET name=?1,value_type=?2,default_value=?3,hidden_at=NULL WHERE id=?4",
                params![name, value_type, default_value.trim(), id],
            )?;
            Ok(id)
        } else {
            let count: i64 = self.connection.query_row(
                "SELECT COUNT(*) FROM server_custom_columns",
                [],
                |row| row.get(0),
            )?;
            if count as usize >= MAX_SERVER_CUSTOM_COLUMNS {
                return Err(StorageError::InvalidImport(format!(
                    "A server table can have at most {MAX_SERVER_CUSTOM_COLUMNS} custom columns"
                )));
            }
            self.connection.execute(
                "INSERT INTO server_custom_columns(name,value_type,default_value,position)
                 VALUES(?1,?2,?3,(SELECT COALESCE(MAX(position),-1)+1 FROM server_custom_columns))",
                params![name, value_type, default_value.trim()],
            )?;
            Ok(self.connection.last_insert_rowid())
        }
    }

    pub fn set_server_column_hidden(&self, id: i64, hidden: bool) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE server_custom_columns SET hidden_at=CASE WHEN ?1 THEN CURRENT_TIMESTAMP ELSE NULL END WHERE id=?2",
            params![hidden, id],
        )?;
        Ok(())
    }

    pub fn move_server_column(&mut self, id: i64, direction: i64) -> Result<(), StorageError> {
        let columns = self.all_server_columns()?;
        let Some(index) = columns.iter().position(|column| column.id == id) else {
            return Ok(());
        };
        let target = if direction < 0 {
            index.checked_sub(1)
        } else if index + 1 < columns.len() {
            Some(index + 1)
        } else {
            None
        };
        let Some(target) = target else { return Ok(()) };
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE server_custom_columns SET position=?1 WHERE id=?2",
            params![target as i64, columns[index].id],
        )?;
        transaction.execute(
            "UPDATE server_custom_columns SET position=?1 WHERE id=?2",
            params![index as i64, columns[target].id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn import_servers(
        &mut self,
        data: &TabularData,
        mappings: &[ServerImportTarget],
    ) -> Result<usize, StorageError> {
        self.import_servers_with_defaults(data, mappings, &ServerImportDefaults::default())
    }

    pub fn import_servers_with_defaults(
        &mut self,
        data: &TabularData,
        mappings: &[ServerImportTarget],
        defaults: &ServerImportDefaults,
    ) -> Result<usize, StorageError> {
        if mappings.len() != data.headers.len() {
            return Err(StorageError::InvalidImport(
                "Every source column needs a mapping".into(),
            ));
        }
        validate_server_mappings(mappings, defaults)?;
        let transaction = self.connection.transaction()?;
        let resolved = resolve_server_import_targets(&transaction, mappings)?;

        let mut changed = 0;
        for (row_index, row) in data.rows.iter().enumerate() {
            let mut name = String::new();
            let mut tags = Vec::new();
            let mut ip_address = String::new();
            let mut ports = String::new();
            let mut environment = String::new();
            let mut status = String::new();
            let mut purpose = String::new();
            let mut custom_values = Vec::new();
            for (source_index, target) in resolved.iter().enumerate() {
                let value = row.get(source_index).map_or("", String::as_str).trim();
                match target {
                    ResolvedServerImportTarget::Skip => {}
                    ResolvedServerImportTarget::Name => name = value.to_owned(),
                    ResolvedServerImportTarget::Tags => {
                        tags.extend(value.split(',').map(str::to_owned));
                    }
                    ResolvedServerImportTarget::IpAddress => ip_address = value.to_owned(),
                    ResolvedServerImportTarget::Ports => ports = value.to_owned(),
                    ResolvedServerImportTarget::Environment => environment = value.to_owned(),
                    ResolvedServerImportTarget::Status => status = value.to_owned(),
                    ResolvedServerImportTarget::Purpose => purpose = value.to_owned(),
                    ResolvedServerImportTarget::Custom(column_id) => {
                        if value.chars().count() > 2_048 {
                            return Err(StorageError::InvalidImport(format!(
                                "Row {}: custom values must be 2,048 characters or fewer",
                                row_index + 2
                            )));
                        }
                        custom_values.push((*column_id, value.to_owned()));
                    }
                }
            }
            if tags.iter().all(|tag| tag.trim().is_empty()) {
                tags.extend(defaults.tags.split(',').map(str::to_owned));
            }
            if tags.iter().all(|tag| tag.trim().is_empty()) {
                tags.push("imported".into());
            }
            if ip_address.trim().is_empty() {
                ip_address = defaults.ip_address.trim().to_owned();
            }
            if ports.trim().is_empty() {
                ports = defaults.ports.trim().to_owned();
            }
            if name.trim().is_empty() {
                name = defaults.name.trim().to_owned();
            }
            if name.trim().is_empty() {
                name = ip_address.clone();
            }
            if environment.trim().is_empty() {
                environment = defaults.environment.trim().to_owned();
            }
            if status.trim().is_empty() {
                status = defaults.status.trim().to_owned();
            }
            if purpose.trim().is_empty() {
                purpose = defaults.purpose.trim().to_owned();
            }
            let draft = validate_server(&ServerDraft {
                name,
                tags,
                ip_address,
                ports,
                environment,
                status,
                purpose,
            })
            .map_err(|error| {
                StorageError::InvalidImport(format!("Row {}: {error}", row_index + 2))
            })?;
            let tag_text = draft.tags.join(", ");
            let existing_id = transaction
                .query_row(
                    "SELECT id FROM server_records WHERE lower(trim(ip_address))=lower(trim(?1)) AND archived_at IS NULL ORDER BY id LIMIT 1",
                    [draft.ip_address.as_str()],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            if defaults.duplicate_policy == ImportDuplicatePolicy::Skip && existing_id.is_some() {
                continue;
            }
            let server_id = if defaults.duplicate_policy == ImportDuplicatePolicy::UpdateExisting {
                if let Some(id) = existing_id {
                    transaction.execute(
                        "UPDATE server_records SET name=?1,purpose=?2,tags=?3,ip_address=?4,ports=?5,environment=?6,status=?7,updated_at=CURRENT_TIMESTAMP WHERE id=?8",
                        params![draft.name, draft.purpose, tag_text, draft.ip_address, draft.ports, draft.environment, draft.status, id],
                    )?;
                    id
                } else {
                    transaction.execute(
                        "INSERT INTO server_records (name,purpose,tags,ip_address,ports,environment,status,position)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,(SELECT COALESCE(MAX(position),-1)+1 FROM server_records))",
                        params![draft.name, draft.purpose, tag_text, draft.ip_address, draft.ports, draft.environment, draft.status],
                    )?;
                    let id = transaction.last_insert_rowid();
                    insert_server_column_defaults(&transaction, id)?;
                    crate::operational::seed_server_services(&transaction, id, true)?;
                    id
                }
            } else {
                transaction.execute(
                    "INSERT INTO server_records (name,purpose,tags,ip_address,ports,environment,status,position)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,(SELECT COALESCE(MAX(position),-1)+1 FROM server_records))",
                    params![draft.name, draft.purpose, tag_text, draft.ip_address, draft.ports, draft.environment, draft.status],
                )?;
                let id = transaction.last_insert_rowid();
                insert_server_column_defaults(&transaction, id)?;
                crate::operational::seed_server_services(&transaction, id, true)?;
                id
            };
            for (column_id, value) in custom_values {
                if !value.is_empty() {
                    let value_type: String = transaction.query_row(
                        "SELECT value_type FROM server_custom_columns WHERE id=?1 AND hidden_at IS NULL",
                        [column_id],
                        |row| row.get(0),
                    )?;
                    validate_custom_value(&value_type, &value)?;
                    transaction.execute(
                        "INSERT INTO server_custom_values (server_id,column_id,value) VALUES (?1,?2,?3)
                         ON CONFLICT(server_id,column_id) DO UPDATE SET value=excluded.value",
                        params![server_id, column_id, value],
                    )?;
                }
            }
            transaction.execute("INSERT INTO resource_history(resource_type,resource_id,action,summary) VALUES('server',?1,'updated','Server imported from a table')", [server_id])?;
            changed += 1;
        }
        transaction.commit()?;
        Ok(changed)
    }

    pub fn save_server(
        &mut self,
        id: Option<i64>,
        draft: &ServerDraft,
    ) -> Result<i64, StorageError> {
        self.save_server_with_context(id, draft, None)
    }

    pub fn save_server_with_context(
        &mut self,
        id: Option<i64>,
        draft: &ServerDraft,
        context: Option<&crate::AssetContextForm>,
    ) -> Result<i64, StorageError> {
        let draft = validate_server(draft).map_err(ValidationError::from)?;
        let tags = draft.tags.join(", ");
        let transaction = self.connection.transaction()?;
        let result = if let Some(id) = id {
            let updated = transaction.execute(
                "UPDATE server_records SET name=?1,purpose=?2,tags=?3,ip_address=?4,ports=?5,environment=?6,status=?7,updated_at=CURRENT_TIMESTAMP
                 WHERE id=?8 AND archived_at IS NULL",
                params![draft.name, draft.purpose, tags, draft.ip_address, draft.ports, draft.environment, draft.status, id],
            )?;
            if updated != 1 {
                return Err(StorageError::InvalidImport(
                    "Server is hidden or no longer exists".into(),
                ));
            }
            id
        } else {
            transaction.execute(
                "INSERT INTO server_records (name,purpose,tags,ip_address,ports,environment,status,position)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,(SELECT COALESCE(MAX(position),-1)+1 FROM server_records))",
                params![draft.name, draft.purpose, tags, draft.ip_address, draft.ports, draft.environment, draft.status],
            )?;
            let id = transaction.last_insert_rowid();
            insert_server_column_defaults(&transaction, id)?;
            crate::operational::seed_server_services(&transaction, id, true)?;
            id
        };
        crate::asset_context::save_context(&transaction, "server", result, context)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn update_server_cell(
        &mut self,
        server_id: i64,
        column_index: usize,
        value: &str,
    ) -> Result<(), StorageError> {
        if value.chars().count() > 2_048 {
            return Err(StorageError::InvalidImport(
                "Cell values must be 2,048 characters or fewer".into(),
            ));
        }
        let value = value.trim();
        match column_index {
            0..=6 => {
                let (name, ip_address, ports, environment, status, purpose, tags): (String, String, String, String, String, String, String) =
                    self.connection.query_row(
                        "SELECT name,ip_address,ports,environment,status,purpose,tags FROM server_records WHERE id=?1",
                        [server_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
                    )?;
                let draft = ServerDraft {
                    name: if column_index == 0 { value } else { &name }.to_owned(),
                    ip_address: if column_index == 1 {
                        value
                    } else {
                        &ip_address
                    }
                    .to_owned(),
                    ports: if column_index == 2 { value } else { &ports }.to_owned(),
                    environment: if column_index == 3 {
                        value
                    } else {
                        &environment
                    }
                    .to_owned(),
                    status: if column_index == 4 { value } else { &status }.to_owned(),
                    purpose: if column_index == 5 { value } else { &purpose }.to_owned(),
                    tags: if column_index == 6 {
                        value.split(',').map(str::to_owned).collect()
                    } else {
                        tags.split(',').map(str::to_owned).collect()
                    },
                };
                self.save_server(Some(server_id), &draft).map(|_| ())
            }
            custom_index => {
                let (column_id, value_type): (i64, String) = self.connection.query_row(
                    "SELECT id,value_type FROM server_custom_columns WHERE hidden_at IS NULL
                     ORDER BY position,id LIMIT 1 OFFSET ?1",
                    [i64::try_from(custom_index - 7).unwrap_or(i64::MAX)],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                validate_custom_value(&value_type, value)?;
                self.connection.execute(
                    "INSERT INTO server_custom_values(server_id,column_id,value) VALUES(?1,?2,?3)
                     ON CONFLICT(server_id,column_id) DO UPDATE SET value=excluded.value",
                    params![server_id, column_id, value],
                )?;
                Ok(())
            }
        }
    }

    pub fn hide_server(&self, id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE server_records SET archived_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP
             WHERE id=?1 AND archived_at IS NULL",
            [id],
        )?;
        Ok(())
    }

    pub fn restore_server(&self, id: i64) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE server_records SET archived_at=NULL,updated_at=CURRENT_TIMESTAMP
             WHERE id=?1 AND archived_at IS NOT NULL",
            [id],
        )?;
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

    pub fn backup_directory(&self) -> Result<PathBuf, StorageError> {
        let database_path = self.database_path.as_ref().ok_or_else(|| {
            StorageError::InvalidImport("Demo workspaces do not create disk backups".into())
        })?;
        Ok(database_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups"))
    }

    pub fn create_managed_backup(&self, retention: usize) -> Result<PathBuf, StorageError> {
        let directory = self.backup_directory()?;
        fs::create_dir_all(&directory).map_err(|source| StorageError::CreateDataDirectory {
            path: directory.clone(),
            source,
        })?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| StorageError::InvalidImport(error.to_string()))?
            .as_secs();
        let path = directory.join(format!("itgla-backup-{timestamp}.db"));
        self.create_backup(&path)?;
        let mut backups = fs::read_dir(&directory)
            .map_err(|source| StorageError::CreateDataDirectory {
                path: directory.clone(),
                source,
            })?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("itgla-backup-") && name.ends_with(".db"))
            })
            .collect::<Vec<_>>();
        backups.sort();
        let keep = retention.clamp(1, 30);
        let remove_count = backups.len().saturating_sub(keep);
        for old in backups.into_iter().take(remove_count) {
            fs::remove_file(&old).map_err(|source| StorageError::FileOperation {
                operation: "remove old backup",
                path: old,
                source,
            })?;
        }
        Ok(path)
    }

    pub fn reset_demo_fixture(&mut self) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        transaction.execute_batch(
            "DELETE FROM operational_inventory;
             DELETE FROM server_custom_values;
             DELETE FROM server_custom_columns;
             DELETE FROM server_records;
             DELETE FROM sites;",
        )?;
        for (position, name) in ["Provider", "Region", "Project", "Owner"]
            .into_iter()
            .enumerate()
        {
            transaction.execute(
                "INSERT INTO server_custom_columns(name,value_type,default_value,position) VALUES(?1,'text','',?2)",
                params![name, position as i64],
            )?;
        }
        let servers = [
            (
                "customer-edge-01",
                "203.0.113.10",
                "22, 80, 443",
                "Production",
                "Active",
                "Customer traffic entry point",
                "customer-facing, production",
                ["DigitalOcean", "Singapore", "Customer portal", "Alex"],
            ),
            (
                "api-prod-01",
                "203.0.113.20",
                "22, 443, 8080",
                "Production",
                "Active",
                "Public API",
                "api, production",
                ["AWS", "Singapore", "Platform API", "Alex"],
            ),
            (
                "worker-prod-01",
                "203.0.113.21",
                "22, 9000",
                "Production",
                "Maintenance",
                "Background jobs",
                "worker, production",
                ["AWS", "Singapore", "Platform API", "Mina"],
            ),
            (
                "postgres-primary",
                "10.20.0.10",
                "5432",
                "Production",
                "Attention",
                "Primary database",
                "database, production",
                ["AWS", "Singapore", "Platform API", "Mina"],
            ),
            (
                "backup-01",
                "10.20.0.30",
                "22",
                "Production",
                "Active",
                "Encrypted backups",
                "backup, internal",
                ["Hetzner", "Helsinki", "Continuity", "Alex"],
            ),
            (
                "staging-api-01",
                "198.51.100.40",
                "22, 443",
                "Staging",
                "Active",
                "Pre-release API",
                "api, staging",
                ["AWS", "Singapore", "Platform API", "Mina"],
            ),
            (
                "dev-sandbox",
                "192.0.2.55",
                "22, 3000",
                "Development",
                "Active",
                "Developer experiments",
                "development, internal",
                ["Local", "Office", "Lab", "Alex"],
            ),
            (
                "legacy-reports",
                "198.51.100.77",
                "22, 8080",
                "Production",
                "Offline",
                "Legacy reporting",
                "legacy, internal",
                ["Linode", "Tokyo", "Reports", "Mina"],
            ),
        ];
        for (position, (name, ip, ports, environment, status, purpose, tags, values)) in
            servers.into_iter().enumerate()
        {
            transaction.execute(
                "INSERT INTO server_records(name,purpose,tags,ip_address,ports,environment,status,position) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![name, purpose, tags, ip, ports, environment, status, position as i64],
            )?;
            let server_id = transaction.last_insert_rowid();
            for (column_position, value) in values.into_iter().enumerate() {
                transaction.execute(
                    "INSERT INTO server_custom_values(server_id,column_id,value) SELECT ?1,id,?2 FROM server_custom_columns WHERE position=?3",
                    params![server_id, value, column_position as i64],
                )?;
            }
        }
        let sites = [
            (
                "Customer portal",
                "https://portal.example.test",
                "Production",
                "Active",
                "Customer workspace",
                "customer-facing, production",
            ),
            (
                "Public website",
                "https://www.example.test",
                "Production",
                "Active",
                "Product website",
                "customer-facing, marketing",
            ),
            (
                "Status page",
                "https://status.example.test",
                "Production",
                "Active",
                "Incident communication",
                "customer-facing, status",
            ),
            (
                "Internal admin",
                "https://admin.example.test",
                "Production",
                "Attention",
                "Operations console",
                "internal, admin",
            ),
            (
                "Staging portal",
                "https://staging.example.test",
                "Staging",
                "Maintenance",
                "Release verification",
                "staging, internal",
            ),
        ];
        for (position, (name, url, environment, status, purpose, tags)) in
            sites.into_iter().enumerate()
        {
            transaction.execute(
                "INSERT INTO sites(name,url,environment,status,purpose,tags,position) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![name, url, environment, status, purpose, tags, position as i64],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn restore_backup(&mut self, path: &Path) -> Result<(), StorageError> {
        self.ensure_external_path(path)?;
        if self.automatic_backup_path("pre-restore.db").as_deref() == Some(path) {
            return Err(StorageError::InvalidImport(
                "The restore source cannot be the automatic recovery point".into(),
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
                "Choose a valid file path outside the current database".into(),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResolvedServerImportTarget {
    Skip,
    Name,
    Tags,
    IpAddress,
    Ports,
    Environment,
    Status,
    Purpose,
    Custom(i64),
}

fn validate_server_mappings(
    mappings: &[ServerImportTarget],
    defaults: &ServerImportDefaults,
) -> Result<(), StorageError> {
    let count =
        |needle: ServerImportTarget| mappings.iter().filter(|target| **target == needle).count();
    let ip_count = count(ServerImportTarget::IpAddress);
    let port_count = count(ServerImportTarget::Ports);
    if [
        ServerImportTarget::Name,
        ServerImportTarget::Tags,
        ServerImportTarget::IpAddress,
        ServerImportTarget::Ports,
        ServerImportTarget::Environment,
        ServerImportTarget::Status,
        ServerImportTarget::Purpose,
    ]
    .into_iter()
    .any(|target| count(target) > 1)
    {
        return Err(StorageError::InvalidImport(
            "Each standard server field can only be mapped once".into(),
        ));
    }
    if ip_count == 0 && defaults.ip_address.trim().is_empty() {
        return Err(StorageError::InvalidImport(
            "Map an IP Address column or set a default IP address".into(),
        ));
    }
    if port_count == 0 && defaults.ports.trim().is_empty() {
        return Err(StorageError::InvalidImport(
            "Map a Ports column or set default ports".into(),
        ));
    }

    let mut destinations = HashSet::new();
    for target in mappings {
        let key = match target {
            ServerImportTarget::ExistingCustom(id) => Some(format!("id:{id}")),
            ServerImportTarget::NewCustom(name) => {
                let name = name.trim();
                if name.is_empty() || name.chars().count() > 64 {
                    return Err(StorageError::InvalidImport(
                        "Custom column names must contain 1 to 64 characters".into(),
                    ));
                }
                Some(format!("name:{}", name.to_lowercase()))
            }
            _ => None,
        };
        if key.is_some_and(|key| !destinations.insert(key)) {
            return Err(StorageError::InvalidImport(
                "Each destination column can only be mapped once".into(),
            ));
        }
    }
    Ok(())
}

fn validate_option_kind(kind: &str) -> Result<(), StorageError> {
    if matches!(kind, "environment" | "status") {
        Ok(())
    } else {
        Err(StorageError::InvalidImport("Unknown option kind".into()))
    }
}

trait SqlExecutor {
    fn execute_sql<P: rusqlite::Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize>;
}

impl SqlExecutor for Connection {
    fn execute_sql<P: rusqlite::Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize> {
        self.execute(sql, params)
    }
}

impl SqlExecutor for Transaction<'_> {
    fn execute_sql<P: rusqlite::Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize> {
        self.execute(sql, params)
    }
}

fn insert_server_column_defaults(
    executor: &impl SqlExecutor,
    server_id: i64,
) -> Result<(), StorageError> {
    executor.execute_sql(
        "INSERT INTO server_custom_values(server_id,column_id,value)
         SELECT ?1,id,default_value FROM server_custom_columns
         WHERE hidden_at IS NULL AND trim(default_value)<>''
         ON CONFLICT(server_id,column_id) DO NOTHING",
        [server_id],
    )?;
    Ok(())
}

fn validate_custom_value(value_type: &str, value: &str) -> Result<(), StorageError> {
    let value = value.trim();
    if value.chars().count() > 2_048 {
        return Err(StorageError::InvalidImport(
            "Custom values must be 2,048 characters or fewer".into(),
        ));
    }
    if value.is_empty() {
        return Ok(());
    }
    let valid = match value_type {
        "text" => true,
        "number" => value.parse::<f64>().is_ok_and(f64::is_finite),
        "date" => {
            let bytes = value.as_bytes();
            bytes.len() == 10
                && bytes[4] == b'-'
                && bytes[7] == b'-'
                && bytes
                    .iter()
                    .enumerate()
                    .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
        }
        "url" => value.starts_with("https://") || value.starts_with("http://"),
        "boolean" => matches!(value.to_ascii_lowercase().as_str(), "true" | "false"),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidImport(format!(
            "{value} is not a valid {value_type} value"
        )))
    }
}

fn validate_option_semantic(semantic: &str) -> Result<(), StorageError> {
    if matches!(
        semantic,
        "healthy" | "attention" | "inactive" | "planned" | "neutral"
    ) {
        Ok(())
    } else {
        Err(StorageError::InvalidImport(
            "Unknown option semantic type".into(),
        ))
    }
}

fn validate_color_token(color: &str) -> Result<(), StorageError> {
    if matches!(
        color,
        "green" | "amber" | "red" | "blue" | "purple" | "cyan" | "slate"
    ) {
        Ok(())
    } else {
        Err(StorageError::InvalidImport("Unknown color token".into()))
    }
}

fn resolve_server_import_targets(
    transaction: &Transaction<'_>,
    mappings: &[ServerImportTarget],
) -> Result<Vec<ResolvedServerImportTarget>, StorageError> {
    let mut existing_ids = HashSet::new();
    {
        let mut statement = transaction.prepare("SELECT id FROM server_custom_columns")?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
        for id in rows {
            existing_ids.insert(id?);
        }
    }

    let mut resolved = Vec::with_capacity(mappings.len());
    for target in mappings {
        let target = match target {
            ServerImportTarget::Skip => ResolvedServerImportTarget::Skip,
            ServerImportTarget::Name => ResolvedServerImportTarget::Name,
            ServerImportTarget::Tags => ResolvedServerImportTarget::Tags,
            ServerImportTarget::IpAddress => ResolvedServerImportTarget::IpAddress,
            ServerImportTarget::Ports => ResolvedServerImportTarget::Ports,
            ServerImportTarget::Environment => ResolvedServerImportTarget::Environment,
            ServerImportTarget::Status => ResolvedServerImportTarget::Status,
            ServerImportTarget::Purpose => ResolvedServerImportTarget::Purpose,
            ServerImportTarget::ExistingCustom(id) => {
                if !existing_ids.contains(id) {
                    return Err(StorageError::InvalidImport(
                        "A mapped custom column no longer exists".into(),
                    ));
                }
                ResolvedServerImportTarget::Custom(*id)
            }
            ServerImportTarget::NewCustom(name) => {
                transaction.execute(
                    "INSERT INTO server_custom_columns (name,position)
                     VALUES (?1,(SELECT COALESCE(MAX(position),-1)+1 FROM server_custom_columns))
                     ON CONFLICT(name) DO NOTHING",
                    [name.trim()],
                )?;
                let id = transaction.query_row(
                    "SELECT id FROM server_custom_columns WHERE name=?1 COLLATE NOCASE",
                    [name.trim()],
                    |row| row.get::<_, i64>(0),
                )?;
                existing_ids.insert(id);
                ResolvedServerImportTarget::Custom(id)
            }
        };
        resolved.push(target);
    }
    let column_count: i64 =
        transaction.query_row("SELECT COUNT(*) FROM server_custom_columns", [], |row| {
            row.get(0)
        })?;
    if column_count as usize > MAX_SERVER_CUSTOM_COLUMNS {
        return Err(StorageError::InvalidImport(format!(
            "A server table can have at most {MAX_SERVER_CUSTOM_COLUMNS} custom columns"
        )));
    }
    let mut custom_destinations = HashSet::new();
    if resolved.iter().any(|target| {
        matches!(target, ResolvedServerImportTarget::Custom(id) if !custom_destinations.insert(*id))
    }) {
        return Err(StorageError::InvalidImport(
            "Each destination column can only be mapped once".into(),
        ));
    }
    Ok(resolved)
}

fn create_parent(path: &Path) -> Result<(), StorageError> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|source| StorageError::FileOperation {
        operation: "create directory",
        path: parent.to_owned(),
        source,
    })
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), StorageError> {
    if path.as_os_str().is_empty() {
        return Err(StorageError::InvalidImport(
            "Export path is required".into(),
        ));
    }
    create_parent(path)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("json");
    let temporary = path.with_extension(format!("{extension}.tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = File::create(&temporary).map_err(|source| StorageError::FileOperation {
            operation: "create temporary export file",
            path: temporary.clone(),
            source,
        })?;
        file.write_all(contents)
            .and_then(|()| file.sync_all())
            .map_err(|source| StorageError::FileOperation {
                operation: "write export file",
                path: temporary.clone(),
                source,
            })?;
        fs::rename(&temporary, path).map_err(|source| StorageError::FileOperation {
            operation: "commit export file",
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
        operation: "read import file",
        path: path.to_owned(),
        source,
    })?;
    if metadata.len() > MAX_IMPORT_BYTES {
        return Err(StorageError::ImportTooLarge {
            maximum_mb: MAX_IMPORT_BYTES / 1024 / 1024,
        });
    }
    let file = File::open(path).map_err(|source| StorageError::FileOperation {
        operation: "open import file",
        path: path.to_owned(),
        source,
    })?;
    let mut contents = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_IMPORT_BYTES + 1)
        .read_to_end(&mut contents)
        .map_err(|source| StorageError::FileOperation {
            operation: "read import file",
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
            "Project count must be between 1 and {MAX_PROJECTS}"
        )));
    }
    if workspace.assets.len() > MAX_ASSETS || workspace.relationships.len() > MAX_RELATIONSHIPS {
        return Err(StorageError::InvalidImport(format!(
            "At most {MAX_ASSETS} resources and {MAX_RELATIONSHIPS} relationships are allowed"
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
            return Err(StorageError::InvalidImport(
                "Project IDs or names are duplicated".into(),
            ));
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
                "A resource ID, project, type, or status is invalid".into(),
            ));
        }
        if !asset_names.insert((
            asset.project_id,
            asset.kind.clone(),
            asset.name.trim().to_lowercase(),
        )) {
            return Err(StorageError::InvalidImport(
                "Resources are duplicated within a project".into(),
            ));
        }
        let mut tags = HashSet::new();
        if asset.tags.len() > 20
            || asset.tags.iter().any(|tag| {
                tag.trim().is_empty()
                    || tag.chars().count() > 48
                    || !tags.insert(tag.trim().to_lowercase())
            })
        {
            return Err(StorageError::InvalidImport(
                "Resource tags are invalid".into(),
            ));
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
            return Err(StorageError::InvalidImport(
                "Relationships are invalid or duplicated".into(),
            ));
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
            "Backup database integrity check failed".into(),
        ));
    }
    let active_projects: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE archived_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|_| StorageError::InvalidBackup("Required project table is missing".into()))?;
    if active_projects == 0 {
        return Err(StorageError::InvalidBackup(
            "The backup has no active project to restore".into(),
        ));
    }
    connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM assets) + (SELECT COUNT(*) FROM tags)
                  + (SELECT COUNT(*) FROM asset_tags) + (SELECT COUNT(*) FROM relationships)",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| {
            StorageError::InvalidBackup("Required resource or relationship table is missing".into())
        })?;
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
            "The backup contains foreign-key violations or cross-project relationships".into(),
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

fn display_resource_kind(value: &str) -> String {
    match value {
        "website" => "Website",
        "domain" => "Domain",
        "certificate" => "Certificate",
        "server" => "Server",
        "service" => "Service",
        _ => "Resource",
    }
    .into()
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
    fn server_services_are_pending_linked_and_created_atomically() -> Result<(), StorageError> {
        let mut repo = Repository::in_memory()?;
        let draft = ServerDraft {
            name: "test-host".into(),
            ip_address: "203.0.113.10".into(),
            ports: "22".into(),
            environment: "production".into(),
            status: "active".into(),
            purpose: "API".into(),
            tags: vec!["production".into()],
        };
        let id = repo.save_server(None, &draft)?;
        let entries = repo.inventory_entries()?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hosts, vec![id]);
        assert_eq!(entries[0].details.ports, "22");
        assert_eq!(entries[0].details.protocol, "SSH");
        assert_eq!(entries[0].details.notes, "ssh");
        assert!(entries[0].details.pending_confirmation);
        repo.save_server(Some(id), &draft)?;
        assert_eq!(repo.inventory_entries()?.len(), 1);
        repo.mark_inventory_verified(entries[0].id)?;
        assert!(!repo.inventory_entries()?[0].details.pending_confirmation);
        let mut details = crate::operational::Details {
            name: "Database".into(),
            ports: "5432".into(),
            protocol: "PostgreSQL".into(),
            ..Default::default()
        };
        let service = repo.save_inventory(0, 0, details.clone(), &[id], None)?;
        details.ports = "3890".into();
        repo.save_inventory(0, 0, details.clone(), &[id], None)?;
        details.ports = "3060".into();
        repo.save_inventory(0, 0, details, &[id], None)?;
        assert_eq!(repo.inventory_entries()?.len(), 4);
        assert!(
            repo.inventory_entries()?
                .iter()
                .all(|entry| entry.hosts == [id])
        );
        assert!(
            repo.inventory_entries()?
                .iter()
                .any(|entry| entry.id == service && entry.details.protocol == "POSTGRESQL")
        );
        repo.connection.execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<4996) INSERT INTO operational_inventory(kind,data) SELECT 0,(SELECT data FROM operational_inventory LIMIT 1) FROM n;")?;
        let before = repo.server_records()?.len();
        assert!(repo.save_server(None, &draft).is_err());
        assert_eq!(
            repo.server_records()?.len(),
            before,
            "failed service creation must roll back the server"
        );
        Ok(())
    }

    #[test]
    fn services_migration_preserves_metadata_and_is_idempotent() -> Result<(), StorageError> {
        let directory = temporary_path("services-migration", "data");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("source.db");
        let mut repo = Repository::open(path.clone())?;
        repo.reset_demo_fixture()?;
        let server = repo.server_records()?[0].id;
        repo.save_endpoint(server, 0, "3890", "TCP", "LDAP", "Directory access")?;
        let details = crate::operational::Details {
            name: "Existing LDAP".into(),
            ports: "3890".into(),
            protocol: "LDAP".into(),
            transport: "TCP".into(),
            notes: "keep this note".into(),
            ..Default::default()
        };
        let existing = repo.save_inventory(0, 0, details, &[server], None)?;
        repo.connection.pragma_update(None, "user_version", 18)?;
        drop(repo);
        let repo = Repository::open(path.clone())?;
        assert!(directory.join("backups/pre-schema-19.db").exists());
        let entries = repo.inventory_entries()?;
        assert_eq!(
            entries
                .iter()
                .filter(|e| e.hosts.contains(&server) && e.details.ports == "3890")
                .count(),
            1
        );
        assert_eq!(
            entries
                .iter()
                .find(|e| e.id == existing)
                .unwrap()
                .details
                .notes,
            "keep this note"
        );
        assert!(
            entries
                .iter()
                .any(|e| e.details.pending_confirmation && e.details.ports == "22")
        );
        let count = entries.len();
        drop(repo);
        let repo = Repository::open(path)?;
        assert_eq!(repo.inventory_entries()?.len(), count);
        assert_eq!(repo.schema_version()?, SCHEMA_VERSION);
        repo.connection.execute("INSERT INTO server_endpoints(server_id,port,transport,protocol,purpose) VALUES(?1,6543,'TCP','PostgreSQL','pooler')", [server])?;
        repo.connection.execute_batch("CREATE TRIGGER reject_migration_service BEFORE INSERT ON operational_inventory BEGIN SELECT RAISE(ABORT,'test migration failure'); END; PRAGMA user_version=18;")?;
        drop(repo);
        let failed = Repository::open(directory.join("source.db"));
        assert!(failed.is_err());
        let connection = Connection::open(directory.join("source.db"))?;
        assert_eq!(
            connection.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
            18
        );
        assert_eq!(
            connection.query_row("SELECT count(*) FROM operational_inventory", [], |r| r
                .get::<_, usize>(0))?,
            count
        );
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
        Ok(())
    }

    #[test]
    fn endpoints_migrate_with_backup_and_roll_back_on_failure() -> Result<(), StorageError> {
        let directory = temporary_path("endpoint-migration", "data");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("source.db");
        let mut repo = Repository::open(path.clone())?;
        repo.reset_demo_fixture()?;
        let count = repo.server_records()?.len();
        repo.connection.execute_batch("DROP TRIGGER server_endpoint_insert; DROP TRIGGER server_endpoint_sync; DROP TABLE server_endpoints; PRAGMA user_version=17;")?;
        drop(repo);
        let mut repo = Repository::open(path)?;
        assert_eq!(repo.schema_version()?, SCHEMA_VERSION);
        assert!(directory.join("backups/pre-schema-18.db").exists());
        assert_eq!(repo.server_records()?.len(), count);
        for server in repo.server_records()? {
            assert_eq!(
                repo.endpoints(server.id)?.len(),
                server.ports.split(',').count()
            );
        }
        repo.connection.execute_batch("DROP TRIGGER server_endpoint_insert; DROP TRIGGER server_endpoint_sync; DROP TABLE server_endpoints; CREATE TABLE server_endpoints(collision INTEGER); PRAGMA user_version=17;")?;
        assert!(repo.migrate_remaining().is_err());
        assert_eq!(repo.schema_version()?, 17);
        assert_eq!(repo.server_records()?.len(), count);
        assert_eq!(
            repo.connection
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))?,
            "ok"
        );
        drop(repo);
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }

    #[test]
    fn font_defaults_to_large_and_preserves_explicit_selection() -> Result<(), StorageError> {
        let repository = Repository::in_memory()?;
        assert_eq!(repository.font_size_step()?, 2);
        repository.set_font_size_step(1)?;
        assert_eq!(repository.font_size_step()?, 1);
        for step in 1..=3 {
            repository.set_font_size_step(step)?;
            assert_eq!(repository.font_size_step()?, step);
            assert_eq!(repository.workspace_preferences().font_size_step, step);
        }
        repository.set_font_size_step(99)?;
        assert_eq!(repository.font_size_step()?, 3);
        let mut preferences = repository.workspace_preferences();
        preferences.theme_index = 4;
        preferences.font_size_step = 4;
        repository.set_workspace_preferences(&preferences)?;
        assert_eq!(repository.workspace_preferences().theme_index, 4);
        assert_eq!(repository.workspace_preferences().font_size_step, 3);
        repository.connection.execute(
            "UPDATE app_settings SET value='invalid' WHERE key='font_size_step'",
            [],
        )?;
        assert_eq!(repository.font_size_step()?, 2);
        Ok(())
    }

    fn connection_at_schema(version: i64) -> Result<Connection, StorageError> {
        assert!((1..=13).contains(&version));
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(include_str!("../migrations/001_initial.sql"))?;
        connection.execute_batch(include_str!("../migrations/seed.sql"))?;
        let migrations = [
            include_str!("../migrations/002_relationship_lifecycle.sql"),
            include_str!("../migrations/003_server_records.sql"),
            include_str!("../migrations/004_server_tags.sql"),
            include_str!("../migrations/005_server_custom_columns.sql"),
            include_str!("../migrations/006_planning.sql"),
            include_str!("../migrations/007_app_settings.sql"),
            include_str!("../migrations/008_work_items.sql"),
            include_str!("../migrations/009_work_scope.sql"),
            include_str!("../migrations/010_work_assets.sql"),
            include_str!("../migrations/011_impact_reviews.sql"),
            include_str!("../migrations/012_work_asset_origin.sql"),
            include_str!("../migrations/013_sites.sql"),
        ];
        for (index, migration) in migrations.iter().take((version - 1) as usize).enumerate() {
            connection.execute_batch(migration)?;
            connection.pragma_update(None, "user_version", (index + 2) as i64)?;
        }
        assert_eq!(
            connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?,
            version
        );
        Ok(connection)
    }

    fn test_work_draft(title: &str) -> WorkDraft {
        WorkDraft {
            title: title.into(),
            kind: WorkKind::Change,
            status: WorkStatus::Planned,
            priority: "P1".into(),
            owner: String::new(),
            start_at: String::new(),
            due_at: String::new(),
            notes: String::new(),
        }
    }

    fn test_asset_draft(project_id: i64, kind: ResourceKind, name: &str) -> AssetDraft {
        AssetDraft {
            project_id,
            kind,
            name: name.into(),
            detail: String::new(),
            status_detail: String::new(),
            environment: "test".into(),
            health: Health::Healthy,
            tags: Vec::new(),
        }
    }

    #[test]
    fn migrates_and_seeds_an_empty_database() -> Result<(), StorageError> {
        let repository = Repository::in_memory()?;
        assert_eq!(repository.schema_version()?, SCHEMA_VERSION);
        assert!(repository.server_records()?.is_empty());
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
            repository.set_column_widths(&[136.0, 200.0, 120.0])?;
            repository.set_font_size_step(1)?;
        }
        let repository = Repository::open(path.clone())?;
        assert_eq!(repository.projects()?.len(), 3);
        assert_eq!(
            repository.column_widths()?,
            [136.0, 200.0, 120.0, 130.0, 110.0, 220.0, 170.0]
        );
        assert_eq!(repository.font_size_step()?, 1);
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
                supported: SCHEMA_VERSION
            })
        ));
        Ok(())
    }

    #[test]
    fn migrates_schema_ten_through_impact_reviews_and_origins() -> Result<(), StorageError> {
        let connection = connection_at_schema(10)?;
        connection.execute(
            "INSERT INTO work_items(title,kind,status,priority) VALUES('Legacy work','change','planned','P1')",
            [],
        )?;
        let work_id = connection.last_insert_rowid();
        connection.execute(
            "INSERT INTO work_item_assets(work_item_id,asset_id) VALUES(?1,1)",
            [work_id],
        )?;

        let repository = Repository::from_connection(connection, None)?;
        assert_eq!(repository.schema_version()?, SCHEMA_VERSION);
        let origin: String = repository.connection.query_row(
            "SELECT origin FROM work_item_assets WHERE work_item_id=?1 AND asset_id=1",
            [work_id],
            |row| row.get(0),
        )?;
        assert_eq!(origin, "direct");
        repository.exclude_work_impact(work_id, 2)?;
        assert_eq!(repository.excluded_impact_count(work_id)?, 1);
        Ok(())
    }

    #[test]
    fn migrates_schema_eleven_without_losing_impact_reviews() -> Result<(), StorageError> {
        let connection = connection_at_schema(11)?;
        connection.execute(
            "INSERT INTO work_items(title,kind,status,priority) VALUES('Reviewed work','change','planned','P1')",
            [],
        )?;
        let work_id = connection.last_insert_rowid();
        connection.execute(
            "INSERT INTO work_item_assets(work_item_id,asset_id) VALUES(?1,1)",
            [work_id],
        )?;
        connection.execute(
            "INSERT INTO work_item_impact_reviews(work_item_id,asset_id,decision) VALUES(?1,2,'excluded')",
            [work_id],
        )?;

        let repository = Repository::from_connection(connection, None)?;
        assert_eq!(repository.schema_version()?, SCHEMA_VERSION);
        assert_eq!(repository.excluded_impact_count(work_id)?, 1);
        assert!(
            repository
                .work_asset_scope(work_id)?
                .iter()
                .any(|asset| asset.id == 1 && asset.selected && asset.origin == "direct")
        );
        Ok(())
    }

    #[test]
    fn migrates_schema_twelve_and_preserves_existing_data() -> Result<(), StorageError> {
        let connection = connection_at_schema(12)?;
        connection.execute(
            "INSERT INTO server_records(purpose,tags,ip_address,ports) VALUES('API','api','203.0.113.8','443')",
            [],
        )?;
        let repository = Repository::from_connection(connection, None)?;
        assert_eq!(repository.schema_version()?, SCHEMA_VERSION);
        assert_eq!(repository.server_records()?.len(), 1);
        assert!(repository.site_records()?.is_empty());
        Ok(())
    }

    #[test]
    fn migrates_server_custom_core_fields_into_schema_fourteen() -> Result<(), StorageError> {
        let connection = connection_at_schema(13)?;
        connection.execute(
            "INSERT INTO app_settings(key,value) VALUES('column_widths','[120,150,100,210]')",
            [],
        )?;
        connection.execute(
            "INSERT INTO server_records(purpose,tags,ip_address,ports) VALUES('legacy-api','api','203.0.113.18','443')",
            [],
        )?;
        let server_id = connection.last_insert_rowid();
        for (position, (name, value)) in [
            ("Name", "api-primary"),
            ("Environment", "staging"),
            ("Status", "maintenance"),
            ("Purpose", "Customer API"),
            ("Owner", "Platform"),
        ]
        .into_iter()
        .enumerate()
        {
            connection.execute(
                "INSERT INTO server_custom_columns(name,position) VALUES(?1,?2)",
                params![name, position as i64],
            )?;
            let column_id = connection.last_insert_rowid();
            connection.execute(
                "INSERT INTO server_custom_values(server_id,column_id,value) VALUES(?1,?2,?3)",
                params![server_id, column_id, value],
            )?;
        }

        let repository = Repository::from_connection(connection, None)?;
        let record = &repository.server_records()?[0];
        assert_eq!(record.name, "api-primary");
        assert_eq!(record.environment, "staging");
        assert_eq!(record.status, "maintenance");
        assert_eq!(record.purpose, "Customer API");
        assert_eq!(repository.server_columns()?[0].name, "Owner");
        assert_eq!(record.custom_values, ["Platform"]);
        assert_eq!(
            repository.column_widths()?,
            [160.0, 150.0, 100.0, 130.0, 110.0, 220.0, 120.0, 210.0]
        );
        Ok(())
    }

    #[test]
    fn creates_updates_hides_restores_and_imports_sites() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let id = repository.save_site(
            None,
            &SiteDraft {
                name: "Customer portal".into(),
                url: "https://app.example.com".into(),
                environment: "production".into(),
                status: "active".into(),
                purpose: "Customer application".into(),
                tags: vec!["customer-facing".into()],
            },
        )?;
        repository.update_site_cell(id, 3, "maintenance")?;
        assert_eq!(repository.site_records()?[0].status, "maintenance");
        repository.hide_site(id)?;
        assert!(repository.site_records()?.is_empty());
        assert_eq!(repository.hidden_site_records()?.len(), 1);
        repository.restore_site(id)?;

        let data = TabularData {
            headers: vec!["Name".into(), "URL".into(), "Purpose".into()],
            rows: vec![vec![
                "Status page".into(),
                "https://status.example.com".into(),
                "Public status".into(),
            ]],
        };
        let mappings = [
            SiteImportTarget::Name,
            SiteImportTarget::Url,
            SiteImportTarget::Purpose,
        ];
        assert_eq!(
            repository.import_sites(&data, &mappings, &SiteImportDefaults::default())?,
            1
        );
        let sites = repository.site_records()?;
        assert_eq!(sites.len(), 2);
        assert!(
            sites
                .iter()
                .any(|site| site.name == "Status page" && site.environment == "production")
        );
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
        assert_eq!(repository.schema_version()?, SCHEMA_VERSION);
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
    fn creates_updates_hides_and_restores_server_records() -> Result<(), StorageError> {
        let path = temporary_path("servers", "db");
        let id = {
            let mut repository = Repository::open(path.clone())?;
            repository.save_server(
                None,
                &ServerDraft {
                    name: "api-01".into(),
                    tags: vec!["production".into(), "api".into()],
                    ip_address: "203.0.113.10".into(),
                    ports: "443, 22, 443".into(),
                    environment: "production".into(),
                    status: "active".into(),
                    purpose: "Public API".into(),
                },
            )?
        };
        let mut repository = Repository::open(path.clone())?;
        assert_eq!(repository.projects()?.len(), 3);
        assert_eq!(repository.server_records()?[0].ports, "443, 22");
        repository.save_server(
            Some(id),
            &ServerDraft {
                name: "api-primary".into(),
                tags: vec!["primary".into(), "api".into()],
                ip_address: "2001:db8::10".into(),
                ports: "443".into(),
                environment: "production".into(),
                status: "active".into(),
                purpose: "Public API".into(),
            },
        )?;
        assert_eq!(repository.server_records()?[0].tags, vec!["primary", "api"]);
        repository.hide_server(id)?;
        assert!(repository.server_records()?.is_empty());
        assert_eq!(repository.hidden_server_records()?.len(), 1);
        repository.restore_server(id)?;
        assert_eq!(repository.server_records()?.len(), 1);
        assert!(repository.hidden_server_records()?.is_empty());
        drop(repository);
        let _ = fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn imports_mapped_rows_and_custom_columns_atomically() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let data = TabularData {
            headers: vec![
                "Labels".into(),
                "Host".into(),
                "Open ports".into(),
                "Owner".into(),
            ],
            rows: vec![
                vec![
                    "production, api".into(),
                    "203.0.113.10".into(),
                    "443, 22".into(),
                    "Platform".into(),
                ],
                vec![
                    "database".into(),
                    "2001:db8::20".into(),
                    "5432".into(),
                    "Data".into(),
                ],
            ],
        };
        let mappings = [
            ServerImportTarget::Tags,
            ServerImportTarget::IpAddress,
            ServerImportTarget::Ports,
            ServerImportTarget::NewCustom("Owner".into()),
        ];
        assert_eq!(repository.import_servers(&data, &mappings)?, 2);
        assert_eq!(repository.server_columns()?[0].name, "Owner");
        let records = repository.server_records()?;
        assert_eq!(records[0].tags, ["production", "api"]);
        assert_eq!(records[0].ports, "443, 22");
        assert_eq!(records[0].custom_values, ["Platform"]);

        let invalid = TabularData {
            headers: vec!["IP".into(), "Ports".into(), "Region".into()],
            rows: vec![vec!["not-an-ip".into(), "443".into(), "west".into()]],
        };
        let invalid_mappings = [
            ServerImportTarget::IpAddress,
            ServerImportTarget::Ports,
            ServerImportTarget::NewCustom("Region".into()),
        ];
        assert!(matches!(
            repository.import_servers(&invalid, &invalid_mappings),
            Err(StorageError::InvalidImport(message)) if message.starts_with("Row 2:")
        ));
        assert_eq!(repository.server_records()?.len(), 2);
        assert_eq!(repository.server_columns()?.len(), 1);
        Ok(())
    }

    #[test]
    fn imports_rows_with_defaults_for_missing_required_values() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let data = TabularData {
            headers: vec!["Host".into(), "Owner".into()],
            rows: vec![
                vec!["203.0.113.30".into(), "Platform".into()],
                vec!["203.0.113.31".into(), "Data".into()],
            ],
        };
        let mappings = [
            ServerImportTarget::IpAddress,
            ServerImportTarget::NewCustom("Owner".into()),
        ];
        let defaults = ServerImportDefaults {
            tags: "imported, prod".into(),
            ip_address: String::new(),
            ports: "443".into(),
            ..ServerImportDefaults::default()
        };
        assert_eq!(
            repository.import_servers_with_defaults(&data, &mappings, &defaults)?,
            2
        );
        let records = repository.server_records()?;
        assert_eq!(records[0].tags, ["imported", "prod"]);
        assert_eq!(records[0].ports, "443");
        Ok(())
    }

    #[test]
    fn updates_standard_and_custom_server_cells() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let id = repository.save_server(
            None,
            &ServerDraft {
                name: "edge-01".into(),
                tags: vec!["prod".into()],
                ip_address: "203.0.113.40".into(),
                ports: "22".into(),
                environment: "production".into(),
                status: "active".into(),
                purpose: "Edge server".into(),
            },
        )?;
        let data = TabularData {
            headers: vec!["IP".into(), "Ports".into(), "Region".into()],
            rows: vec![vec!["203.0.113.40".into(), "22".into(), "Asia".into()]],
        };
        repository.import_servers(
            &data,
            &[
                ServerImportTarget::IpAddress,
                ServerImportTarget::Ports,
                ServerImportTarget::NewCustom("Region".into()),
            ],
        )?;
        repository.update_server_cell(id, 6, "prod, edge")?;
        repository.update_server_cell(id, 2, "443, 8443")?;
        repository.update_server_cell(id, 7, "Singapore")?;
        let record = repository
            .server_records()?
            .into_iter()
            .find(|record| record.id == id)
            .expect("saved server should exist");
        assert_eq!(record.tags, ["prod", "edge"]);
        assert_eq!(record.ports, "443, 8443");
        assert_eq!(record.custom_values, ["Singapore"]);
        Ok(())
    }

    #[test]
    fn creates_updates_and_archives_work_with_checklist() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let id = repository.save_work_item(
            None,
            &WorkDraft {
                title: "Renew production API certificate".into(),
                kind: WorkKind::Renewal,
                status: WorkStatus::Planned,
                priority: "P1".into(),
                owner: "Alex".into(),
                start_at: "2027-03-01".into(),
                due_at: "2027-03-10".into(),
                notes: "Protect every production domain before the old certificate expires.".into(),
            },
        )?;
        let item = repository
            .work_items()?
            .into_iter()
            .find(|item| item.id == id)
            .expect("work item should exist");
        assert_eq!(item.kind, WorkKind::Renewal);
        assert_eq!(item.status, WorkStatus::Planned);
        assert_eq!(item.checklist_total, 3);
        assert_eq!(item.checklist_completed, 0);

        repository.save_work_item(
            Some(id),
            &WorkDraft {
                title: "Renew production API certificate".into(),
                kind: WorkKind::Renewal,
                status: WorkStatus::InProgress,
                priority: "P0".into(),
                owner: "Sam".into(),
                start_at: "2027-03-01".into(),
                due_at: "2027-03-08".into(),
                notes: "Scope confirmed.".into(),
            },
        )?;
        let updated = repository
            .work_items()?
            .into_iter()
            .find(|item| item.id == id)
            .expect("updated work item should exist");
        assert_eq!(updated.status, WorkStatus::InProgress);
        assert_eq!(updated.priority, "P0");
        assert_eq!(updated.owner, "Sam");

        repository.set_work_status(id, WorkStatus::Blocked)?;
        assert_eq!(
            repository
                .work_items()?
                .into_iter()
                .find(|item| item.id == id)
                .expect("quick-updated work item should exist")
                .status,
            WorkStatus::Blocked
        );

        repository.set_work_scope(id, &[1], &[])?;
        let assets = repository.work_asset_scope(id)?;
        assert!(!assets.is_empty());
        let first_asset = assets[0].id;
        repository.set_work_asset_scope(id, &[first_asset])?;
        let inferred = repository.inferred_work_assets(id)?;
        assert!(
            inferred
                .iter()
                .any(|item| item.label.contains("cloudnote.io"))
        );
        let inferred_id = inferred[0].id;
        repository.add_work_asset_to_scope(id, inferred_id)?;
        let scoped_assets = repository.work_asset_scope(id)?;
        assert!(
            scoped_assets
                .iter()
                .any(|item| item.id == first_asset && item.origin == "direct")
        );
        assert!(
            scoped_assets
                .iter()
                .any(|item| item.id == inferred_id && item.origin == "manual")
        );
        repository.set_work_asset_scope(id, &[first_asset, inferred_id])?;
        assert!(
            repository
                .work_asset_scope(id)?
                .iter()
                .any(|item| item.id == inferred_id && item.origin == "manual")
        );
        assert_eq!(
            repository
                .work_asset_scope(id)?
                .iter()
                .filter(|item| item.selected)
                .count(),
            2
        );
        assert_eq!(repository.add_all_inferred_work_assets(id)?, 1);
        assert_eq!(
            repository
                .work_asset_scope(id)?
                .iter()
                .filter(|item| item.selected)
                .count(),
            3
        );
        repository.set_work_asset_scope(id, &[6])?;
        let one_hop = repository.inferred_work_assets_with_depth(id, 1)?;
        let two_hop = repository.inferred_work_assets_with_depth(id, 2)?;
        assert!(one_hop.iter().any(|item| item.label.contains("notes-api")));
        assert!(two_hop.iter().any(|item| item.label.contains("cn-prod-01")));
        let ignored_id = one_hop[0].id;
        repository.exclude_work_impact(id, ignored_id)?;
        assert_eq!(repository.excluded_impact_count(id)?, 1);
        assert!(
            repository
                .inferred_work_assets_with_depth(id, 1)?
                .iter()
                .all(|item| item.id != ignored_id)
        );
        repository.reset_excluded_work_impacts(id)?;
        assert!(
            repository
                .inferred_work_assets_with_depth(id, 1)?
                .iter()
                .any(|item| item.id == ignored_id)
        );
        let (projects, servers) = repository.work_scope(id)?;
        assert!(
            projects
                .iter()
                .any(|project| project.id == 1 && project.selected)
        );
        assert!(servers.iter().all(|server| !server.selected));
        assert_eq!(
            repository
                .work_items()?
                .into_iter()
                .find(|item| item.id == id)
                .expect("scoped work item should exist")
                .project_count,
            1
        );
        assert_eq!(
            repository
                .work_items()?
                .into_iter()
                .find(|work| work.id == id)
                .expect("scoped work item should exist")
                .asset_count,
            1
        );

        repository.archive_work_item(id)?;
        assert!(repository.work_items()?.iter().all(|item| item.id != id));
        Ok(())
    }

    #[test]
    fn reports_actionable_work_scope_health() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let id = repository.save_work_item(
            None,
            &WorkDraft {
                title: "Review deployment scope".into(),
                kind: WorkKind::Change,
                status: WorkStatus::Planned,
                priority: "P1".into(),
                owner: String::new(),
                start_at: String::new(),
                due_at: String::new(),
                notes: String::new(),
            },
        )?;
        let empty_health = repository.work_health(id)?;
        assert!(
            empty_health
                .warnings
                .iter()
                .any(|warning| warning.contains("Add a project"))
        );
        assert_eq!(empty_health.action, Some(WorkHealthAction::ReviewScope));

        repository.set_work_scope(id, &[2], &[])?;
        repository.set_work_asset_scope(id, &[1])?;
        let health = repository.work_health(id)?;
        assert!(
            health
                .warnings
                .iter()
                .any(|warning| warning.contains("outside selected projects"))
        );
        assert_eq!(health.action, Some(WorkHealthAction::ReviewScope));

        repository.save_work_item(
            Some(id),
            &WorkDraft {
                title: "Review deployment scope".into(),
                kind: WorkKind::Change,
                status: WorkStatus::Ready,
                priority: "P1".into(),
                owner: String::new(),
                start_at: String::new(),
                due_at: String::new(),
                notes: String::new(),
            },
        )?;
        let health = repository.work_health(id)?;
        assert!(
            health
                .warnings
                .iter()
                .any(|warning| warning == "Ready work has no owner")
        );
        assert!(
            health
                .warnings
                .iter()
                .any(|warning| warning == "Ready work has no due date")
        );
        Ok(())
    }

    #[test]
    fn classifies_standard_and_flexible_work_due_dates() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let dates = [
            (
                "Overdue",
                "date('now','localtime','-1 day')",
                DueState::Overdue,
            ),
            ("Today", "date('now','localtime')", DueState::Today),
            ("Soon", "date('now','localtime','+3 days')", DueState::Soon),
            (
                "Scheduled",
                "date('now','localtime','+30 days')",
                DueState::Scheduled,
            ),
        ];
        for (title, expression, expected) in dates {
            let due_at: String =
                repository
                    .connection
                    .query_row(&format!("SELECT {expression}"), [], |row| row.get(0))?;
            repository.save_work_item(
                None,
                &WorkDraft {
                    title: title.into(),
                    kind: WorkKind::Task,
                    status: WorkStatus::Planned,
                    priority: "P2".into(),
                    owner: String::new(),
                    start_at: String::new(),
                    due_at,
                    notes: String::new(),
                },
            )?;
            assert_eq!(
                repository
                    .work_items()?
                    .into_iter()
                    .find(|item| item.title == title)
                    .expect("dated work should exist")
                    .due_state,
                expected
            );
        }
        let flexible_id = repository.save_work_item(
            None,
            &WorkDraft {
                title: "Flexible".into(),
                kind: WorkKind::Task,
                status: WorkStatus::Planned,
                priority: "P2".into(),
                owner: String::new(),
                start_at: String::new(),
                due_at: "After the maintenance window".into(),
                notes: String::new(),
            },
        )?;
        assert_eq!(
            repository
                .work_items()?
                .into_iter()
                .find(|item| item.id == flexible_id)
                .expect("flexible work should exist")
                .due_state,
            DueState::Flexible
        );
        Ok(())
    }

    #[test]
    fn adds_completes_and_deletes_checklist_items() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let work_id = repository.save_work_item(
            None,
            &WorkDraft {
                title: "Deploy service".into(),
                kind: WorkKind::Change,
                status: WorkStatus::Planned,
                priority: "P1".into(),
                owner: String::new(),
                start_at: String::new(),
                due_at: String::new(),
                notes: String::new(),
            },
        )?;
        let item_id = repository.add_checklist_item(work_id, "Verify health endpoint")?;
        let before = repository.checklist_items(work_id)?;
        let before_index = before
            .iter()
            .position(|item| item.id == item_id)
            .expect("added checklist item should exist");
        repository.move_checklist_item(item_id, -1)?;
        let after = repository.checklist_items(work_id)?;
        let after_index = after
            .iter()
            .position(|item| item.id == item_id)
            .expect("moved checklist item should exist");
        assert_eq!(after_index + 1, before_index);
        repository.set_checklist_item_completed(item_id, true)?;
        assert!(
            repository
                .checklist_items(work_id)?
                .iter()
                .any(|item| item.id == item_id && item.completed)
        );
        assert!(matches!(
            repository.set_work_status(work_id, WorkStatus::Done),
            Err(StorageError::InvalidImport(_))
        ));
        for item in repository.checklist_items(work_id)? {
            repository.set_checklist_item_completed(item.id, true)?;
        }
        repository.set_work_status(work_id, WorkStatus::Done)?;
        repository.delete_checklist_item(item_id)?;
        assert!(
            repository
                .checklist_items(work_id)?
                .iter()
                .all(|item| item.id != item_id)
        );
        Ok(())
    }

    #[test]
    fn caps_impact_rows_but_reports_the_exact_total() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let work_id = repository.save_work_item(
            None,
            &WorkDraft {
                title: "Large impact fixture".into(),
                kind: WorkKind::Change,
                status: WorkStatus::Planned,
                priority: "P1".into(),
                owner: String::new(),
                start_at: String::new(),
                due_at: String::new(),
                notes: String::new(),
            },
        )?;
        repository.set_work_asset_scope(work_id, &[1])?;
        for index in 0..60 {
            repository.connection.execute(
                "INSERT INTO assets(project_id,kind,name,position) VALUES(1,'service',?1,?2)",
                params![format!("fixture-service-{index:02}"), 100 + index],
            )?;
            let asset_id = repository.connection.last_insert_rowid();
            repository.connection.execute(
                "INSERT INTO relationships(source_asset_id,target_asset_id,kind) VALUES(1,?1,'depends_on')",
                [asset_id],
            )?;
        }
        let preview = repository.inferred_work_assets_with_depth(work_id, 1)?;
        let total = repository.inferred_work_asset_total_with_depth(work_id, 1)?;
        assert_eq!(preview.len(), MAX_INFERRED_IMPACTS);
        assert!(total > MAX_INFERRED_IMPACTS as i64);
        Ok(())
    }

    #[test]
    fn impact_fixture_is_bounded_deduplicated_stable_and_reviewable() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let project_id = repository.save_project(None, "Impact fixture")?;
        let certificate = repository.save_asset(
            None,
            &test_asset_draft(project_id, ResourceKind::Certificate, "fixture-certificate"),
        )?;
        let website_a = repository.save_asset(
            None,
            &test_asset_draft(project_id, ResourceKind::Website, "a.fixture.example"),
        )?;
        let website_b = repository.save_asset(
            None,
            &test_asset_draft(project_id, ResourceKind::Website, "b.fixture.example"),
        )?;
        let domain = repository.save_asset(
            None,
            &test_asset_draft(project_id, ResourceKind::Domain, "fixture.example"),
        )?;
        let service = repository.save_asset(
            None,
            &test_asset_draft(project_id, ResourceKind::Service, "fixture-service"),
        )?;
        let server = repository.save_asset(
            None,
            &test_asset_draft(project_id, ResourceKind::Server, "fixture-server"),
        )?;
        repository.save_relationship(
            project_id,
            website_a,
            certificate,
            RelationshipKind::ProtectedBy,
        )?;
        repository.save_relationship(
            project_id,
            website_b,
            certificate,
            RelationshipKind::ProtectedBy,
        )?;
        repository.save_relationship(
            project_id,
            website_a,
            domain,
            RelationshipKind::UsesDomain,
        )?;
        repository.save_relationship(
            project_id,
            website_b,
            domain,
            RelationshipKind::UsesDomain,
        )?;
        repository.save_relationship(project_id, service, server, RelationshipKind::DeploysTo)?;

        let certificate_work = repository.save_work_item(None, &test_work_draft("Renew cert"))?;
        repository.set_work_asset_scope(certificate_work, &[certificate])?;
        let one_hop = repository.inferred_work_assets_with_depth(certificate_work, 1)?;
        assert_eq!(
            one_hop.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![website_a, website_b]
        );
        let two_hop = repository.inferred_work_assets_with_depth(certificate_work, 2)?;
        assert_eq!(two_hop.iter().filter(|item| item.id == domain).count(), 1);
        assert_eq!(
            two_hop.iter().map(|item| item.id).collect::<Vec<_>>(),
            repository
                .inferred_work_assets_with_depth(certificate_work, 2)?
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>()
        );
        repository.exclude_work_impact(certificate_work, domain)?;
        assert!(
            repository
                .inferred_work_assets_with_depth(certificate_work, 2)?
                .iter()
                .all(|item| item.id != domain)
        );
        repository.reset_excluded_work_impacts(certificate_work)?;
        repository.archive_asset(website_a)?;
        repository.archive_asset(website_b)?;
        assert!(
            repository
                .inferred_work_assets_with_depth(certificate_work, 2)?
                .is_empty()
        );
        assert_eq!(
            repository.inferred_work_asset_total_with_depth(certificate_work, 2)?,
            0
        );

        let service_work = repository.save_work_item(None, &test_work_draft("Deploy service"))?;
        repository.set_work_asset_scope(service_work, &[service])?;
        assert_eq!(
            repository
                .inferred_work_assets_with_depth(service_work, 1)?
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![server]
        );
        Ok(())
    }

    #[test]
    fn impact_reviews_and_scope_origins_survive_reopen() -> Result<(), StorageError> {
        let path = temporary_path("work-impact-reopen", "db");
        let work_id;
        let direct_id = 5;
        let manual_id = 4;
        let excluded_id = 6;
        {
            let mut repository = Repository::open(path.clone())?;
            work_id = repository.save_work_item(None, &test_work_draft("Persistent impact"))?;
            repository.set_work_asset_scope(work_id, &[direct_id])?;
            repository.add_work_asset_to_scope(work_id, manual_id)?;
            repository.exclude_work_impact(work_id, excluded_id)?;
        }

        let repository = Repository::open(path.clone())?;
        let scope = repository.work_asset_scope(work_id)?;
        assert!(
            scope
                .iter()
                .any(|asset| asset.id == direct_id && asset.selected && asset.origin == "direct")
        );
        assert!(
            scope
                .iter()
                .any(|asset| asset.id == manual_id && asset.selected && asset.origin == "manual")
        );
        assert_eq!(repository.excluded_impact_count(work_id)?, 1);
        assert!(
            repository
                .inferred_work_assets_with_depth(work_id, 1)?
                .iter()
                .all(|asset| asset.id != excluded_id)
        );
        drop(repository);
        let _ = fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn migrates_schema_three_purpose_into_tags() -> Result<(), StorageError> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(include_str!("../migrations/001_initial.sql"))?;
        connection.execute_batch(include_str!("../migrations/002_relationship_lifecycle.sql"))?;
        connection.execute_batch(include_str!("../migrations/003_server_records.sql"))?;
        connection.execute(
            "INSERT INTO server_records (purpose, ip_address, ports) VALUES (?1, ?2, ?3)",
            params!["Legacy API", "203.0.113.20", "443"],
        )?;
        connection.pragma_update(None, "user_version", 3)?;

        let repository = Repository::from_connection(connection, None)?;
        assert_eq!(repository.schema_version()?, SCHEMA_VERSION);
        assert_eq!(repository.server_records()?[0].tags, vec!["Legacy API"]);
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
            operation: "write test file",
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
            operation: "create test directory",
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

    #[test]
    fn table_fit_preferences_are_independent_and_preserve_legacy_choices()
    -> Result<(), StorageError> {
        let repository = Repository::in_memory()?;
        repository.connection.execute(
            "INSERT OR REPLACE INTO app_settings(key,value) VALUES ('fit_columns_on_open','1')",
            [],
        )?;
        let mut preferences = repository.workspace_preferences();
        assert!(preferences.fit_columns_on_open);
        assert!(preferences.site_fit_columns_on_open);
        preferences.site_fit_columns_on_open = false;
        repository.set_workspace_preferences(&preferences)?;
        let saved = repository.workspace_preferences();
        assert!(saved.fit_columns_on_open);
        assert!(!saved.site_fit_columns_on_open);
        Ok(())
    }

    #[test]
    fn schema_fifteen_preferences_options_and_demo_are_isolated() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        assert_eq!(repository.schema_version()?, SCHEMA_VERSION);
        assert!(
            repository
                .resource_options("environment", false)?
                .iter()
                .any(|option| option.label == "Production" && option.is_default)
        );

        let mut preferences = repository.workspace_preferences();
        preferences.theme_index = 2;
        preferences.table_density = 0;
        preferences.import_duplicate_policy = ImportDuplicatePolicy::UpdateExisting;
        preferences.backup_retention = 14;
        repository.set_workspace_preferences(&preferences)?;
        assert_eq!(repository.workspace_preferences(), preferences);

        repository.reset_demo_fixture()?;
        assert_eq!(repository.server_records()?.len(), 8);
        assert_eq!(repository.site_records()?.len(), 5);
        assert_eq!(repository.server_columns()?.len(), 4);
        repository.reset_demo_fixture()?;
        assert_eq!(repository.server_records()?.len(), 8);
        Ok(())
    }

    #[test]
    fn managed_option_rename_updates_records_and_used_options_cannot_hide()
    -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let id = repository.save_server(
            None,
            &ServerDraft {
                name: "edge".into(),
                ip_address: "203.0.113.90".into(),
                ports: "443".into(),
                environment: "Production".into(),
                status: "Active".into(),
                purpose: String::new(),
                tags: vec!["edge".into()],
            },
        )?;
        let production = repository
            .resource_options("environment", false)?
            .into_iter()
            .find(|option| option.label == "Production")
            .expect("seeded Production option");
        repository.save_resource_option(
            Some(production.id),
            "environment",
            "Live",
            "healthy",
            "green",
            true,
        )?;
        assert_eq!(
            repository
                .server_records()?
                .into_iter()
                .find(|server| server.id == id)
                .map(|server| server.environment),
            Some("Live".into())
        );
        assert!(repository.hide_resource_option(production.id).is_err());
        Ok(())
    }

    #[test]
    fn import_duplicate_policy_skips_and_updates_by_stable_keys() -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        let data = TabularData {
            headers: vec!["Name".into(), "IP Address".into(), "Ports".into()],
            rows: vec![vec!["api".into(), "203.0.113.44".into(), "443".into()]],
        };
        let mappings = [
            ServerImportTarget::Name,
            ServerImportTarget::IpAddress,
            ServerImportTarget::Ports,
        ];
        let mut defaults = ServerImportDefaults::default();
        assert_eq!(
            repository.import_servers_with_defaults(&data, &mappings, &defaults)?,
            1
        );
        defaults.duplicate_policy = ImportDuplicatePolicy::Skip;
        assert_eq!(
            repository.import_servers_with_defaults(&data, &mappings, &defaults)?,
            0
        );
        let updated = TabularData {
            headers: data.headers.clone(),
            rows: vec![vec![
                "api-renamed".into(),
                "203.0.113.44".into(),
                "8443".into(),
            ]],
        };
        defaults.duplicate_policy = ImportDuplicatePolicy::UpdateExisting;
        assert_eq!(
            repository.import_servers_with_defaults(&updated, &mappings, &defaults)?,
            1
        );
        let matches = repository
            .server_records()?
            .into_iter()
            .filter(|server| server.ip_address == "203.0.113.44")
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1);
        assert_eq!(
            (matches[0].name.as_str(), matches[0].ports.as_str()),
            ("api-renamed", "8443")
        );
        Ok(())
    }

    #[test]
    fn inventory_migration_backup_restore_and_reopen_preserve_links() -> Result<(), StorageError> {
        let directory = temporary_path("inventory-migration", "data");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("source.db");
        let backup = directory.join("inventory.db");
        let mut repository = Repository::open(path.clone())?;
        repository.reset_demo_fixture()?;
        let servers = repository.server_records()?;
        // Model an existing v15 database, including its real server identifiers.
        repository.connection.execute_batch("DROP TRIGGER server_endpoint_insert; DROP TRIGGER server_endpoint_sync; DROP TABLE server_endpoints; DROP TABLE operational_relations; DROP TABLE operational_hosts; DROP TABLE operational_inventory; PRAGMA user_version=15;")?;
        drop(repository);
        let mut repository = Repository::open(path.clone())?;
        assert_eq!(repository.schema_version()?, SCHEMA_VERSION);
        assert_eq!(repository.server_records()?.len(), servers.len());
        let migrated_count = repository.inventory_entries()?.len();
        assert!(migrated_count > 0);
        let before = Connection::open(directory.join("backups/pre-schema-16.db"))?;
        assert_eq!(
            before.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?,
            15
        );
        drop(before);
        assert!(directory.join("backups/pre-schema-17.db").exists());
        let details = |name: &str| crate::operational::Details {
            name: name.into(),
            tags: "production, orders".into(),
            ..Default::default()
        };
        let primary =
            repository.save_inventory(0, 0, details("database primary"), &[servers[0].id], None)?;
        let replica =
            repository.save_inventory(0, 0, details("database replica"), &[servers[1].id], None)?;
        let mut domain = details("api.example.com");
        domain.parent = "example.com".into();
        domain.cdn = "Cloudflare".into();
        repository.save_inventory(0, 1, domain, &[servers[0].id, servers[1].id], None)?;
        repository.add_inventory_relation(primary, replica, 1, "synthetic replication")?;
        repository.create_backup(&backup)?;
        repository.set_inventory_hidden(primary, true)?;
        repository.restore_backup(&backup)?;
        drop(repository);
        let repository = Repository::open(path)?;
        let entries = repository.inventory_entries()?;
        assert_eq!(entries.len(), migrated_count + 3);
        assert!(
            entries
                .iter()
                .filter(|e| !e.details.pending_confirmation)
                .all(|e| !e.hidden)
        );
        assert_eq!(entries.iter().find(|e| e.kind == 1).unwrap().hosts.len(), 2);
        assert_eq!(
            repository.connection.query_row(
                "SELECT COUNT(*) FROM operational_relations WHERE source_id=?1 AND target_id=?2",
                params![primary, replica],
                |r| r.get::<_, i64>(0)
            )?,
            1
        );
        assert_eq!(
            repository
                .connection
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))?,
            "ok"
        );
        drop(repository);
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }

    #[test]
    fn verification_migration_preserves_existing_inventory() -> Result<(), StorageError> {
        let directory = temporary_path("verification-migration", "data");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("source.db");
        let mut repository = Repository::open(path.clone())?;
        repository.reset_demo_fixture()?;
        let host = repository.server_records()?[0].id;
        let id = repository.save_inventory(
            0,
            0,
            crate::operational::Details {
                name: "Existing service".into(),
                ..Default::default()
            },
            &[host],
            None,
        )?;
        repository.connection.execute_batch(
            "DROP TRIGGER server_endpoint_insert; DROP TRIGGER server_endpoint_sync; DROP TABLE server_endpoints; ALTER TABLE operational_inventory DROP COLUMN verified_at; PRAGMA user_version=16;",
        )?;
        drop(repository);
        let repository = Repository::open(path)?;
        let entries = repository.inventory_entries()?;
        let preserved = entries.iter().find(|entry| entry.id == id).unwrap();
        assert_eq!(preserved.hosts, vec![host]);
        assert!(preserved.verified_at.is_none());
        assert!(preserved.verification_due);
        assert!(
            entries
                .iter()
                .any(|entry| entry.details.pending_confirmation)
        );
        assert!(directory.join("backups/pre-schema-17.db").exists());
        drop(repository);
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }

    #[test]
    fn failed_inventory_migration_rolls_back_schema_and_preserves_servers()
    -> Result<(), StorageError> {
        let mut repository = Repository::in_memory()?;
        repository.reset_demo_fixture()?;
        let count = repository.server_records()?.len();
        repository.connection.execute_batch("DROP TRIGGER server_endpoint_insert; DROP TRIGGER server_endpoint_sync; DROP TABLE server_endpoints; DROP TABLE operational_relations; DROP TABLE operational_hosts; DROP TABLE operational_inventory; CREATE TABLE operational_hosts(collision INTEGER); PRAGMA user_version=15;")?;
        assert!(repository.migrate_remaining().is_err());
        assert_eq!(repository.schema_version()?, 15);
        assert_eq!(repository.server_records()?.len(), count);
        assert_eq!(
            repository.connection.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='operational_inventory'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            0
        );
        Ok(())
    }

    #[test]
    fn managed_backup_prunes_old_files_to_retention() -> Result<(), StorageError> {
        let directory = temporary_path("managed-backup", "data");
        fs::create_dir_all(&directory).map_err(|source| StorageError::FileOperation {
            operation: "create test directory",
            path: directory.clone(),
            source,
        })?;
        let repository = Repository::open(directory.join("source.db"))?;
        let backups = repository.backup_directory()?;
        fs::create_dir_all(&backups).map_err(|source| StorageError::FileOperation {
            operation: "create backup directory",
            path: backups.clone(),
            source,
        })?;
        for timestamp in 1..=4 {
            fs::write(backups.join(format!("itgla-backup-{timestamp}.db")), b"old").map_err(
                |source| StorageError::FileOperation {
                    operation: "write old backup fixture",
                    path: backups.clone(),
                    source,
                },
            )?;
        }
        repository.create_managed_backup(2)?;
        let remaining = fs::read_dir(&backups)
            .map_err(|source| StorageError::FileOperation {
                operation: "read backup directory",
                path: backups.clone(),
                source,
            })?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("itgla-backup-")
            })
            .count();
        assert_eq!(remaining, 2);
        assert!(backups.join("pre-schema-16.db").exists());
        drop(repository);
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }
}

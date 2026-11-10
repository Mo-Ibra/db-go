use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Table not found: {0}")]
    TableNotFound(String),
    #[error("only read-only queries are allowed")]
    ReadOnly,
    #[error("query returned no columns")]
    NoColumns,
    #[error("unknown column: {0}")]
    UnknownColumn(String),
    #[error("a value is required for the {0:?} filter")]
    MissingFilterValue(FilterOp),
    #[error("database already exists: {0}")]
    DatabaseExists(String),
    #[error("project already exists: {0}")]
    ProjectExists(String),
    #[error("not a DBGo project: {0}")]
    NotAProject(String),
    #[error("invalid table design: {0}")]
    InvalidDesign(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

#[derive(Debug, Clone, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub not_null: bool,
    pub primary_key: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CellValue {
    Null,
    Integer { value: i64 },
    Real { value: f64 },
    Text { value: String },
    Blob { value: String, size: usize },
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub rowid: Option<i64>,
    pub cells: Vec<CellValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Row>,
    pub next_cursor: Option<i64>,
    pub prev_cursor: Option<i64>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Row>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryPlanStep {
    pub id: i64,
    pub parent: i64,
    pub detail: String,
    pub full_scan: bool,
    pub uses_index: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryPlan {
    pub sql: String,
    pub steps: Vec<QueryPlanStep>,
    pub full_scans: usize,
    pub indexes_used: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexStats {
    pub name: String,
    pub unique: bool,
    pub columns: Vec<String>,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableStats {
    pub name: String,
    pub row_count: i64,
    pub row_count_capped: bool,
    pub row_count_estimated: bool,
    pub size_bytes: Option<i64>,
    pub index_bytes: Option<i64>,
    pub indexes: Vec<IndexStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatabaseStats {
    pub path: String,
    pub page_size: i64,
    pub page_count: i64,
    pub free_bytes: i64,
    pub total_bytes: i64,
    pub dbstat: bool,
    pub tables: Vec<TableStats>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImportFormat {
    Csv,
    Json,
    Jsonl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportColumnMapping {
    pub source: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    pub table: Option<String>,
    pub create_table: bool,
    pub format: ImportFormat,
    pub has_headers: bool,
    pub delimiter: Option<String>,
    #[serde(default)]
    pub mapping: Vec<ImportColumnMapping>,
    pub upsert_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportColumn {
    pub source: String,
    pub suggested_name: String,
    pub inferred_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportPreview {
    pub format: ImportFormat,
    pub columns: Vec<ImportColumn>,
    pub rows: Vec<JsonObject>,
    pub total_rows: usize,
    pub suggested_table: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub table: String,
    pub rows: usize,
    pub session_id: String,
    pub created_table: bool,
    pub ignored_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct D1TableExport {
    pub name: String,
    pub rows: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct D1ExportResult {
    pub output_dir: String,
    pub schema_file: String,
    pub data_files: Vec<String>,
    pub manifest_file: String,
    pub wrangler_file: String,
    pub readme_file: String,
    pub tables: Vec<D1TableExport>,
    pub rows: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecResult {
    pub sql: String,
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Row>,
    pub truncated: bool,
    pub affected: usize,
    pub session_id: Option<String>,
    pub transactional: bool,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TriggerInfo {
    pub name: String,
    pub table: String,
    pub sql: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MaintenanceResult {
    pub action: String,
    pub session_id: Option<String>,
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountResult {
    pub count: i64,
    pub capped: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    pub column: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Contains,
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    IsNull,
    NotNull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterSpec {
    pub column: String,
    pub op: FilterOp,
    #[serde(default, deserialize_with = "deserialize_filter_value")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilterExpr {
    Condition {
        column: String,
        op: FilterOp,
        #[serde(default, deserialize_with = "deserialize_filter_value")]
        value: Option<String>,
    },
    And {
        filters: Vec<FilterExpr>,
    },
    Or {
        filters: Vec<FilterExpr>,
    },
    Not {
        filter: Box<FilterExpr>,
    },
}

impl From<FilterSpec> for FilterExpr {
    fn from(filter: FilterSpec) -> Self {
        FilterExpr::Condition {
            column: filter.column,
            op: filter.op,
            value: filter.value,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FilterInput {
    Spec(FilterSpec),
    Expr(FilterExpr),
}

impl FilterInput {
    pub fn into_expr(self) -> FilterExpr {
        match self {
            FilterInput::Spec(spec) => spec.into(),
            FilterInput::Expr(expr) => expr,
        }
    }
}

fn deserialize_filter_value<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;

    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value)),
        Some(serde_json::Value::Number(value)) => Ok(Some(value.to_string())),
        Some(serde_json::Value::Bool(value)) => Ok(Some(if value { "1" } else { "0" }.to_string())),
        Some(other) => Err(D::Error::custom(format!(
            "filter value must be a string, number, or boolean, got {other}"
        ))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyDesign {
    pub table: String,
    pub column: String,
    pub on_delete: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDesign {
    pub name: String,
    pub type_name: String,
    pub primary_key: bool,
    pub not_null: bool,
    pub unique: bool,
    pub default_value: Option<String>,
    pub references: Option<ForeignKeyDesign>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDesign {
    pub name: String,
    pub columns: Vec<ColumnDesign>,
    pub index_foreign_keys: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum AlterTableOp {
    AddColumn { column: ColumnDesign },
    RenameColumn { from: String, to: String },
    DropColumn { name: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaChange {
    pub session_id: String,
    pub backup_path: Option<String>,
    pub migration_id: i64,
    pub migration_name: String,
    pub migration_path: Option<String>,
    pub statements: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub table: String,
    pub unique: bool,
    pub origin: String,
    pub partial: bool,
    pub columns: Vec<String>,
    pub sql: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForeignKeyInfo {
    pub column: String,
    pub referenced_table: String,
    pub referenced_column: String,
    pub on_update: String,
    pub on_delete: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableDetails {
    pub name: String,
    pub kind: String,
    pub ddl: String,
    pub columns: Vec<ColumnInfo>,
    pub indexes: Vec<IndexInfo>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewInfo {
    pub name: String,
    pub sql: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationInfo {
    pub name: String,
    pub applied: bool,
    pub applied_at: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppliedMigration {
    pub name: String,
    pub session_id: String,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrateResult {
    pub applied: Vec<AppliedMigration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoreResult {
    pub restored_from: String,
    pub safety_backup: Option<String>,
    pub session_id: String,
    pub tables: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentFilesInfo {
    pub directory: String,
    pub database: String,
    pub agents_path: String,
    pub schema_path: String,
    pub config_path: String,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DbInfo {
    pub path: String,
    pub tables: Vec<TableInfo>,
    pub backup_path: Option<String>,
    pub project_path: Option<String>,
    pub agent_files: Option<AgentFilesInfo>,
    pub agent_files_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub database: String,
    pub version: u32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteResult {
    pub session_id: String,
    pub affected: usize,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub actions: i64,
    pub started_at: String,
}

pub type JsonObject = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowUpdate {
    pub rowid: i64,
    pub values: JsonObject,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaColumn {
    pub name: String,
    pub type_name: String,
    pub not_null: bool,
    pub primary_key: bool,
    pub foreign_key: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaTable {
    pub name: String,
    pub columns: Vec<SchemaColumn>,
    pub primary_key: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Relationship {
    pub id: String,
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
    pub on_delete: String,
    pub on_update: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaGraph {
    pub tables: Vec<SchemaTable>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Next,
    Prev,
}

pub struct Database {
    path: PathBuf,
    conn: Connection,
    project_dir: Option<PathBuf>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let project_dir = detect_project_dir(&path);
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-64000;
             PRAGMA mmap_size=268435456;
             PRAGMA busy_timeout=5000;
             PRAGMA foreign_keys=ON;",
        )?;
        Ok(Self {
            path,
            conn,
            project_dir,
        })
    }

    pub fn open_project(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(DbError::NotAProject(format!(
                "path not found: {}",
                path.display()
            )));
        }
        if path.is_dir() {
            let config = read_project_config(path)?;
            let database_path = path.join(&config.database);
            if !database_path.exists() {
                return Err(DbError::NotAProject(format!(
                    "database file missing: {}",
                    database_path.display()
                )));
            }
            let mut database = Self::open(&database_path)?;
            database.project_dir = Some(path.to_path_buf());
            Ok(database)
        } else {
            Self::open(path)
        }
    }

    pub fn project_dir(&self) -> Option<&Path> {
        self.project_dir.as_deref()
    }

    pub fn backup(&self) -> Result<PathBuf> {
        if let Some(project_dir) = &self.project_dir {
            let backups = project_dir.join("backups");
            std::fs::create_dir_all(&backups)?;
            let file_name = self
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("db.sqlite");
            let backup = backups.join(format!("{file_name}.bak-{}", now_millis()));
            backup_database_to(&self.path, &backup)?;
            Ok(backup)
        } else {
            backup_database(&self.path)
        }
    }

    fn backup_directory(&self) -> Option<PathBuf> {
        if let Some(project_dir) = &self.project_dir {
            return Some(project_dir.join("backups"));
        }
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        Some(if parent.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            parent.to_path_buf()
        })
    }

    fn backup_prefix(&self) -> String {
        format!(
            "{}.bak-",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("db.sqlite")
        )
    }

    pub fn backups(&self) -> Result<Vec<BackupInfo>> {
        let Some(directory) = self.backup_directory() else {
            return Ok(Vec::new());
        };
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let prefix = self.backup_prefix();
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.starts_with(&prefix) {
                continue;
            }
            let name = name.to_string();
            let metadata = entry.metadata()?;
            let modified_ms = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as i64)
                .unwrap_or(0);
            let canonical = std::fs::canonicalize(&path).unwrap_or(path);
            backups.push(BackupInfo {
                name,
                path: canonical.display().to_string(),
                size_bytes: metadata.len(),
                modified_ms,
            });
        }
        backups.sort_by(|left, right| {
            right
                .modified_ms
                .cmp(&left.modified_ms)
                .then_with(|| right.name.cmp(&left.name))
        });
        Ok(backups)
    }

    fn resolve_backup(&self, backup: &str) -> Result<PathBuf> {
        let directory = self.backup_directory().ok_or_else(|| {
            DbError::InvalidDesign("database has no backup directory".to_string())
        })?;
        let raw = PathBuf::from(backup);
        let candidate = if raw.is_absolute() {
            raw
        } else {
            let mut candidates = vec![directory.join(&raw)];
            if let Ok(current) = std::env::current_dir() {
                candidates.push(current.join(&raw));
            }
            if let Some(project_dir) = &self.project_dir {
                candidates.push(project_dir.join(&raw));
            }
            candidates
                .into_iter()
                .find(|candidate| candidate.exists())
                .ok_or_else(|| {
                    DbError::InvalidDesign(format!("backup not found: {}", raw.display()))
                })?
        };

        let candidate = candidate.canonicalize().map_err(|_| {
            DbError::InvalidDesign(format!("backup not found: {}", candidate.display()))
        })?;
        let allowed = directory.canonicalize().map_err(|_| {
            DbError::InvalidDesign(format!(
                "backup directory not found: {}",
                directory.display()
            ))
        })?;
        if !candidate.starts_with(&allowed) {
            return Err(DbError::InvalidDesign(format!(
                "backup must be inside {}",
                allowed.display()
            )));
        }
        if let Ok(live) = self.path.canonicalize() {
            if live == candidate {
                return Err(DbError::InvalidDesign(
                    "cannot restore the live database over itself".to_string(),
                ));
            }
        }
        let name = candidate
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !name.starts_with(&self.backup_prefix()) {
            return Err(DbError::InvalidDesign(format!(
                "not a DBGo backup for this database: {name}"
            )));
        }
        Ok(candidate)
    }

    pub fn restore_backup(&mut self, backup: &str) -> Result<RestoreResult> {
        let source = self.resolve_backup(backup)?;
        {
            let check = Connection::open(&source)?;
            let integrity: String = check.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
            if integrity != "ok" {
                return Err(DbError::InvalidDesign(format!(
                    "backup failed quick_check: {integrity}"
                )));
            }
        }

        self.conn.restore(
            rusqlite::MAIN_DB,
            &source,
            None::<fn(rusqlite::backup::Progress)>,
        )?;
        self.conn
            .execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let before = serde_json::json!({ "database": self.path.display().to_string() });
        let after = serde_json::json!({
            "restored_from": source.display().to_string(),
        });
        record_audit(
            &self.conn,
            &session_id,
            "backups",
            None,
            "restore",
            Some(&before),
            Some(&after),
        )?;

        if let Err(error) = self.write_schema_md() {
            eprintln!("[dbgo] warning: could not update schema.md: {error}");
        }
        let tables = self.tables()?.len();

        Ok(RestoreResult {
            restored_from: source.display().to_string(),
            safety_backup: None,
            session_id,
            tables,
        })
    }

    fn schema_document(&self) -> Result<String> {
        let graph = self.schema_graph()?;
        let mut markdown = schema_markdown(&graph);

        let mut index_lines = String::new();
        for table in &graph.tables {
            for index in self.indexes(Some(&table.name))? {
                if index.origin == "pk" {
                    continue;
                }
                index_lines.push_str(&format!(
                    "- `{}`: `{}`{} on {}\n",
                    md_escape(&table.name),
                    md_escape(&index.name),
                    if index.unique { " (unique)" } else { "" },
                    index
                        .columns
                        .iter()
                        .map(|column| format!("`{}`", md_escape(column)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        if !index_lines.is_empty() {
            markdown.push_str("## Indexes\n\n");
            markdown.push_str(&index_lines);
            markdown.push('\n');
        }
        Ok(markdown)
    }

    fn write_schema_md(&self) -> Result<()> {
        let Some(project_dir) = &self.project_dir else {
            return Ok(());
        };
        let document = self.schema_document()?;
        let path = project_dir.join("schema.md");
        let existing = std::fs::read_to_string(&path).ok();
        let merged = merge_marked_document(
            existing.as_deref(),
            &document,
            "dbgo:schema",
            looks_like_dbgo_schema,
        );
        std::fs::write(path, merged)?;
        Ok(())
    }

    pub fn ensure_agent_files(&mut self) -> Result<AgentFilesInfo> {
        let (project_dir, config, created) = match &self.project_dir {
            Some(project_dir) => {
                let config = read_project_config(project_dir)?;
                (project_dir.clone(), config, false)
            }
            None => {
                let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
                let project_dir = if parent.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    parent.to_path_buf()
                };
                let config_path = project_dir.join("dbgo.json");
                if config_path.exists() {
                    let config = read_project_config(&project_dir)?;
                    let config_database = project_dir.join(&config.database);
                    let matches = config_database == self.path
                        || config_database.canonicalize().ok() == self.path.canonicalize().ok();
                    if !matches {
                        return Err(DbError::InvalidDesign(format!(
                            "{} already points to another database: {}",
                            config_path.display(),
                            config.database
                        )));
                    }
                    (project_dir, config, false)
                } else {
                    let database = self
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("db.sqlite")
                        .to_string();
                    let name = project_dir
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| !name.is_empty())
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            self.path
                                .file_stem()
                                .and_then(|stem| stem.to_str())
                                .map(ToOwned::to_owned)
                        })
                        .unwrap_or_else(|| "database".to_string());
                    (
                        project_dir,
                        ProjectConfig {
                            name,
                            database,
                            version: 1,
                            created_at: now_secs(),
                        },
                        true,
                    )
                }
            }
        };

        std::fs::create_dir_all(project_dir.join("migrations"))?;
        std::fs::create_dir_all(project_dir.join("backups"))?;
        if self.table_exists("_dbgo_migrations")? {
            self.materialize_migrations(&project_dir)?;
        } else {
            self.ensure_migrations_table()?;
        }

        let agents_path = project_dir.join("AGENTS.md");
        let existing_agents = std::fs::read_to_string(&agents_path).ok();
        let agents = merge_marked_document(
            existing_agents.as_deref(),
            &agents_markdown(&config),
            "dbgo:agent-guide",
            looks_like_dbgo_agents,
        );
        std::fs::write(&agents_path, agents)?;

        let schema_path = project_dir.join("schema.md");
        let existing_schema = std::fs::read_to_string(&schema_path).ok();
        let schema = merge_marked_document(
            existing_schema.as_deref(),
            &self.schema_document()?,
            "dbgo:schema",
            looks_like_dbgo_schema,
        );
        std::fs::write(&schema_path, schema)?;

        let config_path = project_dir.join("dbgo.json");
        if created {
            write_project_config(&project_dir, &config)?;
        }
        self.project_dir = Some(project_dir.clone());

        Ok(AgentFilesInfo {
            directory: project_dir.display().to_string(),
            database: config.database,
            agents_path: agents_path.display().to_string(),
            schema_path: schema_path.display().to_string(),
            config_path: config_path.display().to_string(),
            created,
        })
    }

    pub fn create_project(
        path: impl AsRef<Path>,
        overwrite: bool,
    ) -> Result<(Self, Option<PathBuf>)> {
        let project_dir = path.as_ref().to_path_buf();
        if project_dir.exists() && !project_dir.is_dir() {
            return Err(DbError::InvalidDesign(format!(
                "project path must be a directory: {}",
                project_dir.display()
            )));
        }

        let config_path = project_dir.join("dbgo.json");
        let database_path = project_dir.join("db.sqlite");
        let exists = config_path.exists() || database_path.exists();
        if exists && !overwrite {
            return Err(DbError::ProjectExists(project_dir.display().to_string()));
        }

        std::fs::create_dir_all(&project_dir)?;
        let backups_dir = project_dir.join("backups");
        let migrations_dir = project_dir.join("migrations");
        std::fs::create_dir_all(&backups_dir)?;
        std::fs::create_dir_all(&migrations_dir)?;

        let mut backup = None;
        if database_path.exists() {
            let backup_path = backups_dir.join(format!("db.sqlite.bak-{}", now_millis()));
            backup_database_to(&database_path, &backup_path)?;
            std::fs::remove_file(&database_path)?;
            remove_sidecar(&database_path, "-wal");
            remove_sidecar(&database_path, "-shm");
            backup = Some(backup_path);
        }

        let mut database = Self::open(&database_path)?;
        database.ensure_migrations_table()?;
        database.project_dir = Some(project_dir.clone());

        let config = ProjectConfig {
            name: project_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("database")
                .to_string(),
            database: "db.sqlite".to_string(),
            version: 1,
            created_at: now_secs(),
        };
        write_project_files(&project_dir, &config)?;

        Ok((database, backup))
    }

    pub fn create(path: impl AsRef<Path>, overwrite: bool) -> Result<(Self, Option<PathBuf>)> {
        let path = path.as_ref().to_path_buf();
        let backup = if path.exists() {
            if !overwrite {
                return Err(DbError::DatabaseExists(path.display().to_string()));
            }
            Some(backup_database(&path)?)
        } else {
            None
        };

        if backup.is_some() {
            std::fs::remove_file(&path)?;
            remove_sidecar(&path, "-wal");
            remove_sidecar(&path, "-shm");
        }

        let database = Self::open(&path)?;
        database.ensure_migrations_table()?;
        Ok((database, backup))
    }

    fn ensure_migrations_table(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _dbgo_migrations (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                sql TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn tables(&self) -> Result<Vec<TableInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, type
             FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND substr(name, 1, 6) <> '_dbgo_'
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TableInfo {
                name: row.get(0)?,
                kind: row.get(1)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn schema_map(&self) -> Result<BTreeMap<String, Vec<String>>> {
        let mut map = BTreeMap::new();
        for table in self.tables()? {
            let columns = self.columns(&table.name)?;
            map.insert(
                table.name,
                columns.into_iter().map(|column| column.name).collect(),
            );
        }
        Ok(map)
    }

    pub fn schema_graph(&self) -> Result<SchemaGraph> {
        let tables = self.tables()?;
        let mut nodes = Vec::with_capacity(tables.len());
        let mut relationships = Vec::new();

        for table in &tables {
            let columns = self.columns(&table.name)?;
            let mut foreign_key_columns = HashSet::new();
            let pragma = format!("PRAGMA foreign_key_list({})", quote_ident(&table.name));
            let mut statement = self.conn.prepare(&pragma)?;
            let mut rows = statement.query([])?;

            while let Some(row) = rows.next()? {
                let to_table: String = row.get(2)?;
                let from_column: String = row.get(3)?;
                let to_column: Option<String> = row.get(4)?;
                let on_update: String = row.get(5)?;
                let on_delete: String = row.get(6)?;
                let to_column = match to_column {
                    Some(column) if !column.is_empty() => column,
                    _ => self
                        .primary_key_column(&to_table)
                        .ok()
                        .flatten()
                        .unwrap_or_default(),
                };
                if to_column.is_empty() {
                    continue;
                }

                foreign_key_columns.insert(from_column.clone());
                relationships.push(Relationship {
                    id: format!("{}:{}->{}:{}", table.name, from_column, to_table, to_column),
                    from_table: table.name.clone(),
                    from_column,
                    to_table,
                    to_column,
                    on_delete,
                    on_update,
                });
            }

            let primary_key = columns
                .iter()
                .filter(|column| column.primary_key)
                .map(|column| column.name.clone())
                .collect();

            let columns = columns
                .into_iter()
                .map(|column| SchemaColumn {
                    name: column.name.clone(),
                    type_name: column.type_name,
                    not_null: column.not_null,
                    primary_key: column.primary_key,
                    foreign_key: foreign_key_columns.contains(&column.name),
                })
                .collect();

            nodes.push(SchemaTable {
                name: table.name.clone(),
                columns,
                primary_key,
            });
        }

        Ok(SchemaGraph {
            tables: nodes,
            relationships,
        })
    }

    fn primary_key_column(&self, table: &str) -> Result<Option<String>> {
        let columns = self.columns(table)?;
        Ok(columns
            .into_iter()
            .find(|column| column.primary_key)
            .map(|column| column.name))
    }

    pub fn columns(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        self.ensure_table(table)?;
        let sql = format!("PRAGMA table_info({})", quote_ident(table));
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(ColumnInfo {
                name: row.get(1)?,
                type_name: row.get(2)?,
                not_null: row.get::<_, i64>(3)? != 0,
                primary_key: row.get::<_, i64>(5)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn page(
        &self,
        table: &str,
        cursor: Option<i64>,
        direction: Direction,
        limit: u32,
    ) -> Result<Page> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        let limit = limit.clamp(1, 5_000) as i64;
        let quoted = quote_ident(table);

        let (sql, params) = match (direction, cursor) {
            (Direction::Next, Some(c)) => (
                format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} \
                     WHERE rowid > ?1 ORDER BY rowid ASC LIMIT ?2"
                ),
                vec![c, limit],
            ),
            (Direction::Next, None) => (
                format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} \
                     ORDER BY rowid ASC LIMIT ?1"
                ),
                vec![limit],
            ),
            (Direction::Prev, Some(c)) => (
                format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} \
                     WHERE rowid < ?1 ORDER BY rowid DESC LIMIT ?2"
                ),
                vec![c, limit],
            ),
            (Direction::Prev, None) => (
                format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} \
                     ORDER BY rowid ASC LIMIT ?1"
                ),
                vec![limit],
            ),
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
        let mut out = Vec::new();

        while let Some(row) = rows.next()? {
            let rowid = row.get::<_, i64>(0).ok();
            let mut cells = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                let value = row.get_ref(index + 1)?;
                cells.push(cell_from_value_ref(value));
            }
            out.push(Row { rowid, cells });
        }

        if direction == Direction::Prev {
            out.reverse();
        }

        let prev_cursor = out.first().and_then(|row| row.rowid);
        let next_cursor = out.last().and_then(|row| row.rowid);

        let has_more = match next_cursor {
            Some(last) => {
                let probe = format!("SELECT 1 FROM {quoted} WHERE rowid > ?1 LIMIT 1");
                let mut stmt = self.conn.prepare(&probe)?;
                let mut rows = stmt.query([last])?;
                rows.next()?.is_some()
            }
            None => false,
        };

        Ok(Page {
            columns,
            rows: out,
            next_cursor,
            prev_cursor,
            has_more,
        })
    }

    pub fn table_page(
        &self,
        table: &str,
        filter: Option<&FilterSpec>,
        sort: Option<&SortSpec>,
        offset: i64,
        limit: u32,
    ) -> Result<Page> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        let quoted = quote_ident(table);

        let (where_sql, mut params) = match filter {
            Some(filter) => filter_clause(filter, &columns)?,
            None => (String::new(), Vec::new()),
        };

        let order_sql = match sort {
            Some(sort) => {
                if !columns.iter().any(|column| column.name == sort.column) {
                    return Err(DbError::UnknownColumn(sort.column.clone()));
                }
                let column = quote_ident(&sort.column);
                let direction = match sort.direction {
                    SortDirection::Asc => "ASC",
                    SortDirection::Desc => "DESC",
                };
                format!(" ORDER BY {column} {direction}, rowid {direction}")
            }
            None => " ORDER BY rowid ASC".to_string(),
        };

        let limit = limit.clamp(1, 5_000) as i64;
        let offset = offset.max(0);
        let sql = format!(
            "SELECT rowid AS __dbgo_rowid, * FROM {quoted}{where_sql}{order_sql} LIMIT ? OFFSET ?"
        );

        params.push(rusqlite::types::Value::Integer(limit + 1));
        params.push(rusqlite::types::Value::Integer(offset));

        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
        let mut out = Vec::new();
        let mut has_more = false;

        while let Some(row) = rows.next()? {
            if out.len() == limit as usize {
                has_more = true;
                break;
            }
            let rowid = row.get::<_, i64>(0).ok();
            let mut cells = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                cells.push(cell_from_value_ref(row.get_ref(index + 1)?));
            }
            out.push(Row { rowid, cells });
        }

        let prev_cursor = out.first().and_then(|row| row.rowid);
        let next_cursor = out.last().and_then(|row| row.rowid);

        Ok(Page {
            columns,
            rows: out,
            next_cursor,
            prev_cursor,
            has_more,
        })
    }

    pub fn data_version(&self) -> Result<i64> {
        self.conn
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .map_err(DbError::from)
    }

    pub fn refresh_agent_guide(&mut self) -> Result<AgentFilesInfo> {
        self.ensure_agent_files()
    }

    fn materialize_migrations(&self, project_dir: &Path) -> Result<()> {
        let migrations_dir = project_dir.join("migrations");
        std::fs::create_dir_all(&migrations_dir)?;
        let mut statement = self
            .conn
            .prepare("SELECT name, sql FROM _dbgo_migrations ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (name, sql) = row?;
            let path = migrations_dir.join(format!("{}.sql", sanitize_slug(&name)));
            if !path.exists() {
                std::fs::write(path, format!("{sql}\n"))?;
            }
        }
        Ok(())
    }

    pub fn sessions(&self) -> Result<Vec<SessionSummary>> {
        self.ensure_audit_table()?;
        let mut statement = self.conn.prepare(
            "SELECT session_id, COUNT(*), MIN(created_at)
             FROM _dbgo_audit
             GROUP BY session_id
             ORDER BY MIN(created_at) DESC
             LIMIT 100",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(SessionSummary {
                session_id: row.get(0)?,
                actions: row.get(1)?,
                started_at: row.get(2)?,
            })
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    fn ensure_audit_table(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _dbgo_audit (
                id INTEGER PRIMARY KEY,
                session_id TEXT NOT NULL,
                table_name TEXT NOT NULL,
                rowid INTEGER,
                action TEXT NOT NULL,
                before_json TEXT,
                after_json TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_dbgo_audit_session ON _dbgo_audit(session_id);",
        )?;
        Ok(())
    }

    pub fn insert_rows(&self, table: &str, rows: &[JsonObject]) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        if rows.is_empty() {
            return Ok(WriteResult {
                session_id: new_session_id(),
                affected: 0,
                backup_path: None,
            });
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let quoted = quote_ident(table);
        let transaction = self.conn.unchecked_transaction()?;
        let mut affected = 0;

        for row in rows {
            validate_json_columns(row, &columns)?;
            let keys: Vec<&String> = row.keys().collect();
            if keys.is_empty() {
                return Err(DbError::InvalidDesign(
                    "insert row has no columns".to_string(),
                ));
            }

            let column_sql = keys
                .iter()
                .map(|key| quote_ident(key.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = vec!["?"; keys.len()].join(", ");
            let sql = format!("INSERT INTO {quoted} ({column_sql}) VALUES ({placeholders})");
            let params = keys
                .iter()
                .map(|key| json_to_sql(row.get(key.as_str()).expect("row key exists")))
                .collect::<Vec<_>>();

            transaction.execute(&sql, rusqlite::params_from_iter(params))?;
            let rowid = transaction.last_insert_rowid();
            let after = fetch_row_json(&transaction, table, &columns, rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(rowid),
                "insert",
                None,
                after.as_ref(),
            )?;
            affected += 1;
        }

        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    pub fn upsert_rows(&self, table: &str, key: &str, rows: &[JsonObject]) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        if !columns.iter().any(|column| column.name == key) {
            return Err(DbError::UnknownColumn(key.to_string()));
        }
        if rows.is_empty() {
            return Ok(WriteResult {
                session_id: new_session_id(),
                affected: 0,
                backup_path: None,
            });
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let quoted = quote_ident(table);
        let key_ident = quote_ident(key);
        let transaction = self.conn.unchecked_transaction()?;
        let mut affected = 0;

        for row in rows {
            validate_json_columns(row, &columns)?;
            if !row.contains_key(key) {
                return Err(DbError::InvalidDesign(format!(
                    "upsert row is missing key column: {key}"
                )));
            }

            let before = {
                let sql = format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} WHERE {key_ident} = ?1 LIMIT 1"
                );
                let mut statement = transaction.prepare(&sql)?;
                let mut found =
                    statement.query([json_to_sql(row.get(key).expect("key exists"))])?;
                if let Some(found_row) = found.next()? {
                    let rowid = found_row.get::<_, i64>(0)?;
                    Some((rowid, row_to_json(&columns, found_row)?))
                } else {
                    None
                }
            };

            let keys: Vec<&String> = row.keys().collect();
            let column_sql = keys
                .iter()
                .map(|column| quote_ident(column.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = vec!["?"; keys.len()].join(", ");

            let mut update_columns: Vec<&String> = Vec::new();
            for column in &keys {
                if column.as_str() != key {
                    update_columns.push(*column);
                }
            }

            let conflict = if update_columns.is_empty() {
                format!("ON CONFLICT({key_ident}) DO NOTHING")
            } else {
                let updates = update_columns
                    .iter()
                    .map(|column| {
                        format!(
                            "{} = excluded.{}",
                            quote_ident(column.as_str()),
                            quote_ident(column.as_str())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("ON CONFLICT({key_ident}) DO UPDATE SET {updates}")
            };

            let sql =
                format!("INSERT INTO {quoted} ({column_sql}) VALUES ({placeholders}) {conflict}");
            let params = keys
                .iter()
                .map(|column| json_to_sql(row.get(column.as_str()).expect("row key exists")))
                .collect::<Vec<_>>();
            transaction.execute(&sql, rusqlite::params_from_iter(params))?;

            let (rowid, action) = if let Some((rowid, _)) = &before {
                (*rowid, "update")
            } else {
                (transaction.last_insert_rowid(), "insert")
            };
            let after = fetch_row_json(&transaction, table, &columns, rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(rowid),
                action,
                before.as_ref().map(|(_, value)| value),
                after.as_ref(),
            )?;
            affected += 1;
        }

        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    pub fn update_rows(
        &self,
        table: &str,
        filter: &FilterSpec,
        values: &JsonObject,
    ) -> Result<WriteResult> {
        self.update_rows_expr(table, &FilterExpr::from(filter.clone()), values)
    }

    pub fn update_rows_expr(
        &self,
        table: &str,
        filter: &FilterExpr,
        values: &JsonObject,
    ) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        if values.is_empty() {
            return Err(DbError::InvalidDesign("no values to update".to_string()));
        }

        let mut set_parts = Vec::new();
        let mut set_params = Vec::new();
        for (name, value) in values {
            validate_json_column(name, &columns)?;
            if columns
                .iter()
                .any(|column| column.name == *name && column.primary_key)
            {
                return Err(DbError::InvalidDesign(
                    "updating primary key columns is not supported".to_string(),
                ));
            }
            set_parts.push(format!("{} = ?", quote_ident(name)));
            set_params.push(json_to_sql(value));
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let (condition, where_params) = filter_expr_clause(filter, &columns)?;
        let where_sql = format!(" WHERE {condition}");
        let quoted = quote_ident(table);
        let transaction = self.conn.unchecked_transaction()?;
        let matching = select_rows_json(&transaction, &quoted, &where_sql, where_params, &columns)?;

        let set_sql = set_parts.join(", ");
        for (rowid, before) in &matching {
            let mut params = set_params.clone();
            params.push(rusqlite::types::Value::Integer(*rowid));
            transaction.execute(
                &format!("UPDATE {quoted} SET {set_sql} WHERE rowid = ?"),
                rusqlite::params_from_iter(params),
            )?;
            let after = fetch_row_json(&transaction, table, &columns, *rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(*rowid),
                "update",
                Some(before),
                after.as_ref(),
            )?;
        }

        let affected = matching.len();
        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    pub fn delete_rows(&self, table: &str, filter: &FilterSpec) -> Result<WriteResult> {
        self.delete_rows_expr(table, &FilterExpr::from(filter.clone()))
    }

    pub fn delete_rows_expr(&self, table: &str, filter: &FilterExpr) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let (condition, where_params) = filter_expr_clause(filter, &columns)?;
        let where_sql = format!(" WHERE {condition}");
        let quoted = quote_ident(table);
        let transaction = self.conn.unchecked_transaction()?;
        let matching = select_rows_json(&transaction, &quoted, &where_sql, where_params, &columns)?;

        for (rowid, before) in &matching {
            transaction.execute(&format!("DELETE FROM {quoted} WHERE rowid = ?1"), [*rowid])?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(*rowid),
                "delete",
                Some(before),
                None,
            )?;
        }

        let affected = matching.len();
        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    pub fn save_changes(
        &self,
        table: &str,
        updates: &[RowUpdate],
        inserts: &[JsonObject],
        deletes: &[i64],
    ) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        if updates.is_empty() && inserts.is_empty() && deletes.is_empty() {
            return Ok(WriteResult {
                session_id: new_session_id(),
                affected: 0,
                backup_path: None,
            });
        }

        for update in updates {
            validate_update_values(&update.values, &columns)?;
        }
        for insert in inserts {
            validate_json_columns(insert, &columns)?;
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let quoted = quote_ident(table);
        let transaction = self.conn.unchecked_transaction()?;
        let mut affected = 0;

        // Deletes run first so a replacement row can reuse a key freed in the
        // same save. Updates come next to free unique values before inserts.
        for rowid in deletes {
            let before = fetch_row_json(&transaction, table, &columns, *rowid)?
                .ok_or_else(|| DbError::InvalidDesign(format!("row not found: {rowid}")))?;
            transaction.execute(&format!("DELETE FROM {quoted} WHERE rowid = ?1"), [*rowid])?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(*rowid),
                "delete",
                Some(&before),
                None,
            )?;
            affected += 1;
        }

        for update in updates {
            let before =
                fetch_row_json(&transaction, table, &columns, update.rowid)?.ok_or_else(|| {
                    DbError::InvalidDesign(format!("row not found: {}", update.rowid))
                })?;
            if update.values.is_empty() {
                continue;
            }

            let mut set_parts = Vec::new();
            let mut params = Vec::new();
            for (name, value) in &update.values {
                set_parts.push(format!("{} = ?", quote_ident(name)));
                params.push(json_to_sql(value));
            }
            params.push(rusqlite::types::Value::Integer(update.rowid));

            transaction.execute(
                &format!(
                    "UPDATE {quoted} SET {} WHERE rowid = ?",
                    set_parts.join(", ")
                ),
                rusqlite::params_from_iter(params),
            )?;
            let after = fetch_row_json(&transaction, table, &columns, update.rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(update.rowid),
                "update",
                Some(&before),
                after.as_ref(),
            )?;
            affected += 1;
        }

        for row in inserts {
            let keys: Vec<&String> = row.keys().collect();
            if keys.is_empty() {
                return Err(DbError::InvalidDesign(
                    "insert row has no columns".to_string(),
                ));
            }
            let column_sql = keys
                .iter()
                .map(|key| quote_ident(key.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = vec!["?"; keys.len()].join(", ");
            let sql = format!("INSERT INTO {quoted} ({column_sql}) VALUES ({placeholders})");
            let params = keys
                .iter()
                .map(|key| json_to_sql(row.get(key.as_str()).expect("row key exists")))
                .collect::<Vec<_>>();

            transaction.execute(&sql, rusqlite::params_from_iter(params))?;
            let rowid = transaction.last_insert_rowid();
            let after = fetch_row_json(&transaction, table, &columns, rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(rowid),
                "insert",
                None,
                after.as_ref(),
            )?;
            affected += 1;
        }

        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    pub fn query(&self, sql: &str, limit: u32) -> Result<QueryResult> {
        let limit = limit.clamp(1, 10_000) as usize;
        let mut stmt = self.conn.prepare(sql)?;

        if !stmt.readonly() {
            return Err(DbError::ReadOnly);
        }

        let columns: Vec<ColumnInfo> = stmt
            .columns()
            .map(|column| ColumnInfo {
                name: column.name().to_string(),
                type_name: column.decl_type().unwrap_or("").to_string(),
                not_null: false,
                primary_key: false,
            })
            .collect();

        if columns.is_empty() {
            return Err(DbError::NoColumns);
        }

        let column_count = columns.len();
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        let mut truncated = false;

        while let Some(row) = rows.next()? {
            if out.len() == limit {
                truncated = true;
                break;
            }
            let mut cells = Vec::with_capacity(column_count);
            for index in 0..column_count {
                cells.push(cell_from_value_ref(row.get_ref(index)?));
            }
            out.push(Row { rowid: None, cells });
        }

        Ok(QueryResult {
            columns,
            rows: out,
            truncated,
        })
    }

    pub fn explain_query(&self, sql: &str) -> Result<QueryPlan> {
        let sql = sql.trim().trim_end_matches(';').trim();
        if sql.is_empty() {
            return Err(DbError::InvalidDesign(
                "a query is required for EXPLAIN".to_string(),
            ));
        }

        let explain = format!("EXPLAIN QUERY PLAN {sql}");
        let mut statement = self.conn.prepare(&explain)?;
        let mut rows = statement.query([])?;
        let mut steps = Vec::new();
        while let Some(row) = rows.next()? {
            let detail: String = row.get(3)?;
            let upper = detail.to_ascii_uppercase();
            let full_scan = upper.starts_with("SCAN ")
                && !upper.contains(" USING ")
                && !upper.contains("SUBQUERY")
                && !upper.contains("CONSTANT ROW");
            let uses_index = upper.contains("USING INDEX")
                || upper.contains("USING COVERING INDEX")
                || upper.contains("USING INTEGER PRIMARY KEY");
            steps.push(QueryPlanStep {
                id: row.get(0)?,
                parent: row.get(1)?,
                detail,
                full_scan,
                uses_index,
            });
        }

        let full_scans = steps.iter().filter(|step| step.full_scan).count();
        let indexes_used = steps.iter().filter(|step| step.uses_index).count();
        Ok(QueryPlan {
            sql: sql.to_string(),
            steps,
            full_scans,
            indexes_used,
        })
    }

    pub fn database_stats(&self) -> Result<DatabaseStats> {
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let free_pages: i64 = self
            .conn
            .query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
        let dbstat = self.dbstat_sizes()?;
        let estimates = self.row_count_estimates()?;
        let mut tables = Vec::new();
        for table in self.tables()? {
            tables.push(self.stats_for_table(&table.name, dbstat.as_ref(), &estimates)?);
        }
        Ok(DatabaseStats {
            path: self.path.display().to_string(),
            page_size,
            page_count,
            free_bytes: free_pages * page_size,
            total_bytes: page_count * page_size,
            dbstat: dbstat.is_some(),
            tables,
        })
    }

    pub fn table_stats(&self, table: &str) -> Result<TableStats> {
        self.ensure_table(table)?;
        let indexes = self.indexes(Some(table))?;
        let mut names = vec![table.to_string()];
        names.extend(indexes.into_iter().map(|index| index.name));
        let dbstat = self.dbstat_sizes_for(&names)?;
        let estimates = self.row_count_estimates()?;
        self.stats_for_table(table, dbstat.as_ref(), &estimates)
    }

    fn stats_for_table(
        &self,
        table: &str,
        dbstat: Option<&HashMap<String, i64>>,
        estimates: &HashMap<String, i64>,
    ) -> Result<TableStats> {
        let (row_count, row_count_capped, row_count_estimated) =
            if let Some(estimate) = estimates.get(table) {
                (*estimate, false, true)
            } else {
                let count = self.count_rows(table, 1_000_000)?;
                (count.count, count.capped, false)
            };

        let indexes = self
            .indexes(Some(table))?
            .into_iter()
            .map(|index| IndexStats {
                size_bytes: dbstat.map(|sizes| *sizes.get(&index.name).unwrap_or(&0)),
                name: index.name,
                unique: index.unique,
                columns: index.columns,
            })
            .collect::<Vec<_>>();
        let size_bytes = dbstat.map(|sizes| *sizes.get(table).unwrap_or(&0));
        let index_bytes = if size_bytes.is_some() {
            Some(
                indexes
                    .iter()
                    .filter_map(|index| index.size_bytes)
                    .sum::<i64>(),
            )
        } else {
            None
        };

        Ok(TableStats {
            name: table.to_string(),
            row_count,
            row_count_capped,
            row_count_estimated,
            size_bytes,
            index_bytes,
            indexes,
        })
    }

    fn row_count_estimates(&self) -> Result<HashMap<String, i64>> {
        let exists: i64 = self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'sqlite_stat1'
             )",
            [],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Ok(HashMap::new());
        }

        let mut statement = self.conn.prepare("SELECT tbl, stat FROM sqlite_stat1")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut estimates = HashMap::new();
        for row in rows {
            let (table, stat) = row?;
            if let Some(estimate) = stat
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<i64>().ok())
            {
                estimates.entry(table).or_insert(estimate);
            }
        }
        Ok(estimates)
    }

    fn dbstat_sizes(&self) -> Result<Option<HashMap<String, i64>>> {
        let mut statement = match self
            .conn
            .prepare("SELECT name, SUM(pgsize) FROM dbstat GROUP BY name")
        {
            Ok(statement) => statement,
            Err(_) => return Ok(None),
        };
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut sizes = HashMap::new();
        for row in rows {
            let (name, size) = row?;
            sizes.insert(name, size);
        }
        Ok(Some(sizes))
    }

    fn dbstat_sizes_for(&self, names: &[String]) -> Result<Option<HashMap<String, i64>>> {
        if names.is_empty() {
            return Ok(None);
        }
        let placeholders = vec!["?"; names.len()].join(", ");
        let sql = format!(
            "SELECT name, SUM(pgsize) FROM dbstat WHERE name IN ({placeholders}) GROUP BY name"
        );
        let mut statement = match self.conn.prepare(&sql) {
            Ok(statement) => statement,
            Err(_) => return Ok(None),
        };
        let rows = statement.query_map(rusqlite::params_from_iter(names.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut sizes = HashMap::new();
        for row in rows {
            let (name, size) = row?;
            sizes.insert(name, size);
        }
        Ok(Some(sizes))
    }

    pub fn export_query(
        &self,
        sql: &str,
        format: ExportFormat,
        writer: &mut dyn Write,
    ) -> Result<usize> {
        let sql = sql.trim().trim_end_matches(';');
        if sql.is_empty() {
            return Err(DbError::InvalidDesign(
                "a query is required for export".to_string(),
            ));
        }
        let mut statement = self.conn.prepare(sql)?;
        if !statement.readonly() {
            return Err(DbError::ReadOnly);
        }
        self.export_statement(&mut statement, &[], format, writer)
    }

    pub fn export_table(
        &self,
        table: &str,
        filter: Option<&FilterSpec>,
        sort: Option<&SortSpec>,
        format: ExportFormat,
        writer: &mut dyn Write,
    ) -> Result<usize> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        let (where_sql, params) = match filter {
            Some(filter) => filter_clause(filter, &columns)?,
            None => (String::new(), Vec::new()),
        };
        let order_sql = if let Some(sort) = sort {
            if !columns.iter().any(|column| column.name == sort.column) {
                return Err(DbError::UnknownColumn(sort.column.clone()));
            }
            let direction = match sort.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            format!(" ORDER BY {} {direction}", quote_ident(&sort.column))
        } else {
            String::new()
        };
        let sql = format!("SELECT * FROM {}{where_sql}{order_sql}", quote_ident(table));
        let mut statement = self.conn.prepare(&sql)?;
        self.export_statement(&mut statement, &params, format, writer)
    }

    fn export_statement(
        &self,
        statement: &mut rusqlite::Statement<'_>,
        params: &[rusqlite::types::Value],
        format: ExportFormat,
        writer: &mut dyn Write,
    ) -> Result<usize> {
        let columns = statement
            .column_names()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let mut rows = statement.query(rusqlite::params_from_iter(params.iter()))?;

        match format {
            ExportFormat::Csv => {
                writeln!(
                    writer,
                    "{}",
                    columns
                        .iter()
                        .map(|column| csv_escape(column))
                        .collect::<Vec<_>>()
                        .join(",")
                )?;
                let mut count = 0;
                while let Some(row) = rows.next()? {
                    let mut fields = Vec::with_capacity(columns.len());
                    for index in 0..columns.len() {
                        fields.push(csv_value(row.get_ref(index)?));
                    }
                    writeln!(
                        writer,
                        "{}",
                        fields
                            .iter()
                            .map(|field| csv_escape(field))
                            .collect::<Vec<_>>()
                            .join(",")
                    )?;
                    count += 1;
                }
                Ok(count)
            }
            ExportFormat::Json => {
                write!(writer, "[")?;
                let mut first = true;
                let mut count = 0;
                while let Some(row) = rows.next()? {
                    if !first {
                        write!(writer, ",")?;
                    }
                    first = false;
                    let mut object = serde_json::Map::new();
                    for (index, name) in columns.iter().enumerate() {
                        object.insert(name.clone(), export_json(row.get_ref(index)?));
                    }
                    serde_json::to_writer(&mut *writer, &object)?;
                    count += 1;
                }
                writeln!(writer, "]")?;
                Ok(count)
            }
        }
    }

    pub fn import_file(&self, path: &Path, request: &ImportRequest) -> Result<ImportResult> {
        let (source_columns, source_rows) = read_import_source(
            path,
            request.format,
            request.has_headers,
            request.delimiter.as_deref(),
        )?;
        if source_rows.is_empty() {
            return Err(DbError::InvalidDesign(
                "the import file has no data rows".to_string(),
            ));
        }
        let inferred = infer_import_columns(&source_columns, &source_rows, request.format);

        let (table_name, mapping, created_table) = if request.create_table {
            if request.upsert_key.is_some() {
                return Err(DbError::InvalidDesign(
                    "create-table imports cannot use an upsert key".to_string(),
                ));
            }
            let raw_table = request
                .table
                .clone()
                .unwrap_or_else(|| suggest_table_name(path));
            let table_name = sanitize_import_identifier(&raw_table, "imported_table");
            if self.table_exists(&table_name)? {
                return Err(DbError::InvalidDesign(format!(
                    "table already exists: {table_name}"
                )));
            }

            let requested = request
                .mapping
                .iter()
                .map(|mapping| (mapping.source.as_str(), mapping.target.as_deref()))
                .collect::<HashMap<_, _>>();
            let mut used = HashSet::new();
            let mut pairs = Vec::new();
            let mut design_columns = Vec::new();
            for (index, source) in source_columns.iter().enumerate() {
                let target = match requested.get(source.as_str()) {
                    Some(None) => continue,
                    Some(Some(target)) => *target,
                    None => inferred[index].suggested_name.as_str(),
                };
                let target = unique_import_identifier(target, &mut used, index + 1);
                design_columns.push(ColumnDesign {
                    name: target.clone(),
                    type_name: inferred[index].inferred_type.clone(),
                    primary_key: false,
                    not_null: false,
                    unique: false,
                    default_value: None,
                    references: None,
                });
                pairs.push((source.clone(), target));
            }
            if pairs.is_empty() {
                return Err(DbError::InvalidDesign(
                    "no source columns are mapped".to_string(),
                ));
            }

            let design = TableDesign {
                name: table_name.clone(),
                columns: design_columns,
                index_foreign_keys: false,
            };
            self.create_table(&design)?;
            (table_name, pairs, true)
        } else {
            let table_name = request
                .table
                .clone()
                .ok_or_else(|| DbError::InvalidDesign("a target table is required".to_string()))?;
            self.ensure_table(&table_name)?;
            let target_columns = self.columns(&table_name)?;
            let mapping =
                resolve_import_mapping(&source_columns, &target_columns, &request.mapping)?;
            (table_name, mapping, false)
        };

        let target_columns = self.columns(&table_name)?;
        let target_index = target_columns
            .iter()
            .map(|column| (column.name.to_ascii_lowercase(), column.clone()))
            .collect::<HashMap<_, _>>();
        let target_names = mapping
            .iter()
            .map(|(_, target)| target.clone())
            .collect::<Vec<_>>();

        let upsert_key = match request.upsert_key.as_deref() {
            Some(key) => target_names
                .iter()
                .find(|target| target.eq_ignore_ascii_case(key))
                .cloned()
                .ok_or_else(|| DbError::UnknownColumn(key.to_string()))
                .map(Some)?,
            None => None,
        };

        let mut converted_rows = Vec::with_capacity(source_rows.len());
        for (row_index, source_row) in source_rows.iter().enumerate() {
            let mut target_row = JsonObject::new();
            for (source, target) in &mapping {
                let column = target_index
                    .get(&target.to_ascii_lowercase())
                    .ok_or_else(|| DbError::UnknownColumn(target.clone()))?;
                let value = source_row
                    .get(source)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let value = if request.format == ImportFormat::Csv {
                    match value {
                        serde_json::Value::String(raw) => {
                            convert_csv_import_value(&raw, column, row_index + 1)?
                        }
                        serde_json::Value::Null => serde_json::Value::Null,
                        other => other,
                    }
                } else {
                    value
                };
                target_row.insert(target.clone(), value);
            }
            converted_rows.push(target_row);
        }

        if let Some(key) = &upsert_key {
            for row in &converted_rows {
                if row.get(key).is_none() || row.get(key) == Some(&serde_json::Value::Null) {
                    return Err(DbError::InvalidDesign(format!(
                        "import row is missing a non-null upsert key: {key}"
                    )));
                }
            }
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let quoted = quote_ident(&table_name);
        let column_sql = target_names
            .iter()
            .map(|name| quote_ident(name))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = vec!["?"; target_names.len()].join(", ");
        let insert_prefix = format!("INSERT INTO {quoted} ({column_sql}) VALUES ({placeholders})");
        let sql = if let Some(key) = &upsert_key {
            let updates = target_names
                .iter()
                .filter(|name| !name.eq_ignore_ascii_case(key))
                .map(|name| format!("{} = excluded.{}", quote_ident(name), quote_ident(name)))
                .collect::<Vec<_>>()
                .join(", ");
            if updates.is_empty() {
                format!(
                    "{insert_prefix} ON CONFLICT({}) DO NOTHING",
                    quote_ident(key)
                )
            } else {
                format!(
                    "{insert_prefix} ON CONFLICT({}) DO UPDATE SET {updates}",
                    quote_ident(key)
                )
            }
        } else {
            insert_prefix
        };

        let transaction = self.conn.unchecked_transaction()?;
        let mut affected = 0;
        for row in &converted_rows {
            let params = target_names
                .iter()
                .map(|name| {
                    json_to_sql(
                        row.get(name)
                            .expect("every mapped target column is present"),
                    )
                })
                .collect::<Vec<_>>();

            let (rowid, before) = if let Some(key) = &upsert_key {
                let key_value = row.get(key).expect("upsert key is present");
                let where_sql = format!(" WHERE {} = ?", quote_ident(key));
                let before = select_rows_json(
                    &transaction,
                    &quoted,
                    &where_sql,
                    vec![json_to_sql(key_value)],
                    &target_columns,
                )?
                .into_iter()
                .next();
                transaction.execute(&sql, rusqlite::params_from_iter(params))?;
                let rowid = match &before {
                    Some((rowid, _)) => *rowid,
                    None => transaction.last_insert_rowid(),
                };
                (rowid, before.map(|(_, value)| value))
            } else {
                transaction.execute(&sql, rusqlite::params_from_iter(params))?;
                (transaction.last_insert_rowid(), None)
            };

            let after = fetch_row_json(&transaction, &table_name, &target_columns, rowid)?;
            record_audit(
                &transaction,
                &session_id,
                &table_name,
                Some(rowid),
                if before.is_some() { "update" } else { "insert" },
                before.as_ref(),
                after.as_ref(),
            )?;
            affected += 1;
        }
        transaction.commit()?;

        let mapped_sources = mapping
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<HashSet<_>>();
        let ignored_columns = source_columns
            .into_iter()
            .filter(|source| !mapped_sources.contains(source))
            .collect();

        Ok(ImportResult {
            table: table_name,
            rows: affected,
            session_id,
            created_table,
            ignored_columns,
        })
    }

    pub fn export_d1(
        &self,
        output_dir: &Path,
        database_name: Option<&str>,
        force: bool,
    ) -> Result<D1ExportResult> {
        if output_dir.exists() {
            if !output_dir.is_dir() {
                return Err(DbError::InvalidDesign(format!(
                    "export path must be a directory: {}",
                    output_dir.display()
                )));
            }
            if !force && std::fs::read_dir(output_dir)?.next().is_some() {
                return Err(DbError::InvalidDesign(format!(
                    "output directory is not empty: {} (use --force to overwrite)",
                    output_dir.display()
                )));
            }
        } else {
            std::fs::create_dir_all(output_dir)?;
        }

        let generated_at: String = self
            .conn
            .query_row("SELECT datetime('now')", [], |row| row.get(0))?;
        let name = d1_database_name(self, database_name);
        let mut warnings = Vec::new();

        let objects = self.d1_schema_objects()?;
        let virtual_tables = objects
            .iter()
            .filter(|object| {
                object.kind == "table"
                    && object
                        .sql
                        .trim_start()
                        .to_ascii_uppercase()
                        .starts_with("CREATE VIRTUAL TABLE")
            })
            .map(|object| object.name.clone())
            .collect::<Vec<_>>();
        if !virtual_tables.is_empty() {
            warnings.push(format!(
                "skipped virtual table(s): {}",
                virtual_tables.join(", ")
            ));
        }

        let virtual_lower = virtual_tables
            .iter()
            .map(|table| table.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        let mut skipped = virtual_lower.clone();
        for table in self.tables()? {
            let lower = table.name.to_ascii_lowercase();
            if virtual_lower
                .iter()
                .any(|virtual_name| lower.starts_with(&format!("{virtual_name}_")))
            {
                skipped.insert(lower);
                warnings.push(format!("skipped shadow table: {}", table.name));
            }
        }

        let mut table_names = self
            .tables()?
            .into_iter()
            .map(|table| table.name)
            .filter(|table| !skipped.contains(&table.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        table_names = order_d1_tables(table_names, self, &skipped, &mut warnings)?;

        let mut schema = format!(
            "-- D1 export generated by DBGo\n-- source: {}\n-- generated: {generated_at}\n\n",
            self.path.display()
        );
        let mut written_objects = 0;
        for table in &table_names {
            if let Some(object) = objects
                .iter()
                .find(|object| object.kind == "table" && object.name.eq_ignore_ascii_case(table))
            {
                schema.push_str(object.sql.trim_end_matches(';'));
                schema.push_str(";\n\n");
                written_objects += 1;
            }
        }
        for kind in ["index", "view", "trigger"] {
            for object in objects.iter().filter(|object| object.kind == kind) {
                if skipped.contains(&object.table.to_ascii_lowercase()) {
                    warnings.push(format!(
                        "skipped {} {} on skipped table {}",
                        object.kind, object.name, object.table
                    ));
                    continue;
                }
                schema.push_str(object.sql.trim_end_matches(';'));
                schema.push_str(";\n\n");
                written_objects += 1;
            }
        }
        if written_objects == 0 {
            warnings.push("no exportable schema objects found".to_string());
        }

        let schema_path = output_dir.join("schema.sql");
        std::fs::write(&schema_path, schema)?;

        let mut data_writer = D1DataWriter::new(output_dir.to_path_buf(), generated_at.clone());
        let mut tables = Vec::new();
        let mut total_rows = 0;
        for table in &table_names {
            let columns = self.d1_export_columns(table)?;
            if columns.is_empty() {
                warnings.push(format!("skipped table without exportable columns: {table}"));
                continue;
            }
            let column_sql = columns
                .iter()
                .map(|column| quote_ident(column))
                .collect::<Vec<_>>()
                .join(", ");
            let select_sql = format!("SELECT {column_sql} FROM {}", quote_ident(table));
            let mut statement = self.conn.prepare(&select_sql)?;
            let mut rows = statement.query([])?;
            let mut table_rows = 0;
            while let Some(row) = rows.next()? {
                let mut values = Vec::with_capacity(columns.len());
                for index in 0..columns.len() {
                    values.push(d1_sql_literal(row.get_ref(index)?));
                }
                let statement = format!(
                    "INSERT INTO {} ({column_sql}) VALUES ({});",
                    quote_ident(table),
                    values.join(", ")
                );
                data_writer.write(&statement)?;
                table_rows += 1;
                total_rows += 1;
            }
            tables.push(D1TableExport {
                name: table.clone(),
                rows: table_rows,
            });
        }
        data_writer.finish()?;
        let data_files = data_writer.files.clone();

        let wrangler_path = output_dir.join("wrangler.toml");
        std::fs::write(&wrangler_path, d1_wrangler_toml(&name))?;

        let readme_path = output_dir.join("README.md");
        std::fs::write(
            &readme_path,
            d1_readme(
                &name,
                &self.path,
                &generated_at,
                &data_files,
                &tables,
                &warnings,
            ),
        )?;

        let manifest_path = output_dir.join("manifest.json");
        let manifest = serde_json::json!({
            "generated_by": "DBGo",
            "generated_at": generated_at,
            "source": self.path.display().to_string(),
            "database_name": name,
            "schema_file": "schema.sql",
            "data_files": &data_files,
            "tables": tables.iter().map(|table| serde_json::json!({
                "name": table.name,
                "rows": table.rows,
            })).collect::<Vec<_>>(),
            "total_rows": total_rows,
            "warnings": &warnings,
        });
        std::fs::write(
            &manifest_path,
            format!("{}\n", serde_json::to_string_pretty(&manifest)?),
        )?;

        Ok(D1ExportResult {
            output_dir: output_dir.display().to_string(),
            schema_file: schema_path.display().to_string(),
            data_files,
            manifest_file: manifest_path.display().to_string(),
            wrangler_file: wrangler_path.display().to_string(),
            readme_file: readme_path.display().to_string(),
            tables,
            rows: total_rows,
            warnings,
        })
    }

    fn d1_schema_objects(&self) -> Result<Vec<D1SqlObject>> {
        let mut statement = self.conn.prepare(
            "SELECT type, name, tbl_name, sql
             FROM sqlite_master
             WHERE sql IS NOT NULL
               AND name NOT LIKE 'sqlite_%'
               AND substr(name, 1, 6) <> '_dbgo_'
               AND substr(tbl_name, 1, 6) <> '_dbgo_'
             ORDER BY CASE type
                        WHEN 'table' THEN 0
                        WHEN 'index' THEN 1
                        WHEN 'view' THEN 2
                        WHEN 'trigger' THEN 3
                        ELSE 4
                      END,
                      name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(D1SqlObject {
                kind: row.get(0)?,
                name: row.get(1)?,
                table: row.get(2)?,
                sql: row.get(3)?,
            })
        })?;
        let mut objects = Vec::new();
        for row in rows {
            objects.push(row?);
        }
        Ok(objects)
    }

    fn d1_export_columns(&self, table: &str) -> Result<Vec<String>> {
        let pragma = format!("PRAGMA table_xinfo({})", quote_ident(table));
        let mut statement = self.conn.prepare(&pragma)?;
        let mut rows = statement.query([])?;
        let mut columns = Vec::new();
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            let hidden: i64 = row.get(6)?;
            if hidden == 0 {
                columns.push(name);
            }
        }
        Ok(columns)
    }

    pub fn exec(
        &self,
        sql: &str,
        allow_write: bool,
        transactional: bool,
        returning: bool,
        limit: u32,
    ) -> Result<ExecResult> {
        let sql = sql.trim();
        if sql.is_empty() {
            return Err(DbError::InvalidDesign(
                "a SQL statement is required".to_string(),
            ));
        }
        let limit = limit.clamp(1, 10_000) as usize;

        if !allow_write {
            let result = self.query(sql, limit as u32)?;
            return Ok(ExecResult {
                sql: sql.to_string(),
                columns: result.columns,
                rows: result.rows,
                truncated: result.truncated,
                affected: 0,
                session_id: None,
                transactional: false,
                read_only: true,
            });
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let after = serde_json::json!({
            "sql": sql,
            "returning": returning,
        });

        if returning {
            let (columns, rows, truncated, affected) = if transactional {
                let transaction = self.conn.unchecked_transaction()?;
                let result = exec_returning(&transaction, sql, limit)?;
                record_audit(
                    &transaction,
                    &session_id,
                    "sql",
                    None,
                    "exec",
                    None,
                    Some(&after),
                )?;
                transaction.commit()?;
                result
            } else {
                let result = exec_returning(&self.conn, sql, limit)?;
                record_audit(
                    &self.conn,
                    &session_id,
                    "sql",
                    None,
                    "exec",
                    None,
                    Some(&after),
                )?;
                result
            };
            return Ok(ExecResult {
                sql: sql.to_string(),
                columns,
                rows,
                truncated,
                affected,
                session_id: Some(session_id),
                transactional,
                read_only: false,
            });
        }

        let affected = if transactional {
            let transaction = self.conn.unchecked_transaction()?;
            let before = self.conn.total_changes();
            transaction.execute_batch(sql)?;
            let affected = self.conn.total_changes().saturating_sub(before) as usize;
            record_audit(
                &transaction,
                &session_id,
                "sql",
                None,
                "exec",
                None,
                Some(&after),
            )?;
            transaction.commit()?;
            affected
        } else {
            let before = self.conn.total_changes();
            self.conn.execute_batch(sql)?;
            let affected = self.conn.total_changes().saturating_sub(before) as usize;
            record_audit(
                &self.conn,
                &session_id,
                "sql",
                None,
                "exec",
                None,
                Some(&after),
            )?;
            affected
        };

        Ok(ExecResult {
            sql: sql.to_string(),
            columns: Vec::new(),
            rows: Vec::new(),
            truncated: false,
            affected,
            session_id: Some(session_id),
            transactional,
            read_only: false,
        })
    }

    pub fn drop_table(&self, table: &str) -> Result<SchemaChange> {
        self.ensure_table(table)?;
        if table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "internal tables cannot be dropped".to_string(),
            ));
        }
        let before = self.table_ddl(table)?;
        self.run_schema_change(
            &format!("drop_table_{table}"),
            vec![format!("DROP TABLE {}", quote_ident(table))],
            "drop_table",
            table,
            Some(serde_json::json!({ "ddl": before })),
            None,
        )
    }

    pub fn rename_table(&self, from: &str, to: &str) -> Result<SchemaChange> {
        self.ensure_table(from)?;
        validate_ident(to)?;
        if to.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "names starting with _dbgo_ are reserved".to_string(),
            ));
        }
        if self.any_object_exists(to)? {
            return Err(DbError::InvalidDesign(format!(
                "object already exists: {to}"
            )));
        }
        let before = self.table_ddl(from)?;
        self.run_schema_change(
            &format!("rename_table_{from}_to_{to}"),
            vec![format!(
                "ALTER TABLE {} RENAME TO {}",
                quote_ident(from),
                quote_ident(to)
            )],
            "rename_table",
            from,
            Some(serde_json::json!({ "ddl": before })),
            Some(serde_json::json!({ "new_name": to })),
        )
    }

    pub fn triggers(&self, table: Option<&str>) -> Result<Vec<TriggerInfo>> {
        let mut sql = "SELECT name, tbl_name, sql
                       FROM sqlite_master
                       WHERE type = 'trigger'
                         AND sql IS NOT NULL
                         AND name NOT LIKE 'sqlite_%'
                         AND substr(name, 1, 6) <> '_dbgo_'"
            .to_string();
        if table.is_some() {
            sql.push_str(" AND tbl_name = ?1 COLLATE NOCASE");
        }
        sql.push_str(" ORDER BY name");
        let mut statement = self.conn.prepare(&sql)?;
        let rows = if let Some(table) = table {
            statement
                .query_map([table], |row| {
                    Ok(TriggerInfo {
                        name: row.get(0)?,
                        table: row.get(1)?,
                        sql: row.get(2)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            statement
                .query_map([], |row| {
                    Ok(TriggerInfo {
                        name: row.get(0)?,
                        table: row.get(1)?,
                        sql: row.get(2)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    pub fn create_trigger(&self, name: &str, sql: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(format!(
                "reserved trigger name: {name}"
            )));
        }
        let exists: i64 = self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'trigger' AND name = ?1 COLLATE NOCASE
             )",
            [name],
            |row| row.get(0),
        )?;
        if exists != 0 {
            return Err(DbError::InvalidDesign(format!(
                "trigger already exists: {name}"
            )));
        }
        let sql = sql.trim().trim_end_matches(';').trim();
        if !sql.to_ascii_uppercase().starts_with("CREATE TRIGGER") {
            return Err(DbError::InvalidDesign(
                "trigger SQL must start with CREATE TRIGGER".to_string(),
            ));
        }
        self.run_schema_change(
            &format!("trigger_{name}"),
            vec![sql.to_string()],
            "create_trigger",
            name,
            None,
            Some(serde_json::json!({ "ddl": sql })),
        )
    }

    pub fn drop_trigger(&self, name: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(format!(
                "reserved trigger name: {name}"
            )));
        }
        let ddl = self
            .conn
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'trigger' AND name = ?1 COLLATE NOCASE",
                [name],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .ok_or_else(|| DbError::InvalidDesign(format!("trigger not found: {name}")))?;
        self.run_schema_change(
            &format!("drop_trigger_{name}"),
            vec![format!("DROP TRIGGER {}", quote_ident(name))],
            "drop_trigger",
            name,
            Some(serde_json::json!({ "ddl": ddl })),
            None,
        )
    }

    pub fn vacuum(&self) -> Result<MaintenanceResult> {
        self.conn.execute_batch("VACUUM;")?;
        self.record_maintenance("vacuum", serde_json::json!({}), Vec::new(), Vec::new())
    }

    pub fn analyze(&self) -> Result<MaintenanceResult> {
        self.conn.execute_batch("ANALYZE;")?;
        self.record_maintenance("analyze", serde_json::json!({}), Vec::new(), Vec::new())
    }

    pub fn pragma(&self, pragma: &str, write: bool) -> Result<MaintenanceResult> {
        let body = pragma.trim().trim_end_matches(';').trim();
        let body = if body.to_ascii_lowercase().starts_with("pragma ") {
            body[7..].trim()
        } else {
            body
        };
        if body.is_empty() {
            return Err(DbError::InvalidDesign(
                "a PRAGMA name is required".to_string(),
            ));
        }
        let sql = format!("PRAGMA {body}");
        let mut statement = self.conn.prepare(&sql)?;
        let columns = statement
            .columns()
            .map(|column| ColumnInfo {
                name: column.name().to_string(),
                type_name: column.decl_type().unwrap_or("").to_string(),
                not_null: false,
                primary_key: false,
            })
            .collect::<Vec<_>>();
        let mut rows = statement.query([])?;
        let mut output = Vec::new();
        while let Some(row) = rows.next()? {
            let mut cells = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                cells.push(cell_from_value_ref(row.get_ref(index)?));
            }
            output.push(Row { rowid: None, cells });
        }
        if write {
            self.record_maintenance(
                "pragma",
                serde_json::json!({ "pragma": body }),
                columns,
                output,
            )
        } else {
            Ok(MaintenanceResult {
                action: "pragma".to_string(),
                session_id: None,
                columns,
                rows: output,
            })
        }
    }

    fn record_maintenance(
        &self,
        action: &str,
        details: serde_json::Value,
        columns: Vec<ColumnInfo>,
        rows: Vec<Row>,
    ) -> Result<MaintenanceResult> {
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        record_audit(
            &self.conn,
            &session_id,
            "maintenance",
            None,
            action,
            None,
            Some(&details),
        )?;
        Ok(MaintenanceResult {
            action: action.to_string(),
            session_id: Some(session_id),
            columns,
            rows,
        })
    }

    pub fn count_rows(&self, table: &str, cap: i64) -> Result<CountResult> {
        self.ensure_table(table)?;
        let quoted = quote_ident(table);
        let sql = format!("SELECT COUNT(*) FROM (SELECT 1 FROM {quoted} LIMIT ?1)");
        let count: i64 = self.conn.query_row(&sql, [cap + 1], |row| row.get(0))?;
        let capped = count > cap;
        Ok(CountResult {
            count: if capped { cap } else { count },
            capped,
        })
    }

    pub fn create_table(&self, design: &TableDesign) -> Result<TableInfo> {
        let table_name = design.name.trim();
        validate_ident(table_name)?;
        if table_name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "names starting with _dbgo_ are reserved".to_string(),
            ));
        }
        if design.columns.is_empty() {
            return Err(DbError::InvalidDesign(
                "a table needs at least one column".to_string(),
            ));
        }
        if self.table_exists(table_name)? {
            return Err(DbError::InvalidDesign(format!(
                "table already exists: {table_name}"
            )));
        }

        let mut seen = HashSet::new();
        let mut primary_keys = 0;
        for column in &design.columns {
            validate_ident(&column.name)?;
            if !seen.insert(column.name.to_ascii_lowercase()) {
                return Err(DbError::InvalidDesign(format!(
                    "duplicate column: {}",
                    column.name
                )));
            }
            normalize_type(&column.type_name)?;
            if column.primary_key {
                primary_keys += 1;
            }
            if let Some(references) = &column.references {
                self.validate_foreign_key(design, column, references)?;
            }
        }
        if primary_keys > 1 {
            return Err(DbError::InvalidDesign(
                "only one primary key column is supported".to_string(),
            ));
        }

        let create_sql = render_create_table(design)?;
        let mut statements = vec![create_sql];
        if design.index_foreign_keys {
            for column in &design.columns {
                if column.references.is_some() {
                    statements.push(render_index_for_column(table_name, &column.name));
                }
            }
        }
        let migration_sql = format!("{};", statements.join(";\n"));

        self.run_schema_change(
            &format!("create_table_{table_name}"),
            statements,
            "create_table",
            table_name,
            None,
            Some(serde_json::json!({ "ddl": migration_sql })),
        )?;

        Ok(TableInfo {
            name: table_name.to_string(),
            kind: "table".to_string(),
        })
    }

    pub fn table_ddl(&self, table: &str) -> Result<String> {
        self.ensure_table(table)?;
        Ok(self
            .conn
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table' AND name = ?1 COLLATE NOCASE",
                [table],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .unwrap_or_default())
    }

    pub fn foreign_keys(&self, table: &str) -> Result<Vec<ForeignKeyInfo>> {
        self.ensure_table(table)?;
        let pragma = format!("PRAGMA foreign_key_list({})", quote_ident(table));
        let mut statement = self.conn.prepare(&pragma)?;
        let mut rows = statement.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let referenced_table: String = row.get(2)?;
            let column: String = row.get(3)?;
            let referenced_column: Option<String> = row.get(4)?;
            let referenced_column = match referenced_column {
                Some(column) if !column.is_empty() => column,
                _ => self
                    .primary_key_column(&referenced_table)
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
            };
            out.push(ForeignKeyInfo {
                column,
                referenced_table,
                referenced_column,
                on_update: row.get(5)?,
                on_delete: row.get(6)?,
            });
        }
        Ok(out)
    }

    pub fn indexes(&self, table: Option<&str>) -> Result<Vec<IndexInfo>> {
        if let Some(table) = table {
            return self.indexes_for_table(table);
        }

        let mut statement = self.conn.prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND substr(name, 1, 6) <> '_dbgo_'
             ORDER BY name",
        )?;
        let names = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for name in names {
            out.extend(self.indexes_for_table(&name)?);
        }
        Ok(out)
    }

    fn indexes_for_table(&self, table: &str) -> Result<Vec<IndexInfo>> {
        self.ensure_table(table)?;
        let pragma = format!("PRAGMA index_list({})", quote_ident(table));
        let mut statement = self.conn.prepare(&pragma)?;
        let mut rows = statement.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            let unique: bool = row.get::<_, i64>(2)? != 0;
            let origin: String = row.get(3)?;
            let partial: bool = row.get::<_, i64>(4)? != 0;
            let sql: Option<String> = self
                .conn
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1 COLLATE NOCASE",
                    [&name],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();

            let info_pragma = format!("PRAGMA index_info({})", quote_ident(&name));
            let mut info_statement = self.conn.prepare(&info_pragma)?;
            let columns = info_statement
                .query_map([], |row| {
                    Ok(row
                        .get::<_, Option<String>>(2)?
                        .unwrap_or_else(|| "(expression)".to_string()))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            out.push(IndexInfo {
                name,
                table: table.to_string(),
                unique,
                origin,
                partial,
                columns,
                sql,
            });
        }
        Ok(out)
    }

    pub fn table_details(&self, table: &str) -> Result<TableDetails> {
        Ok(TableDetails {
            name: table.to_string(),
            kind: "table".to_string(),
            ddl: self.table_ddl(table)?,
            columns: self.columns(table)?,
            indexes: self.indexes(Some(table))?,
            foreign_keys: self.foreign_keys(table)?,
        })
    }

    fn plan_alter_table(
        &self,
        table: &str,
        operations: &[AlterTableOp],
    ) -> Result<(Vec<String>, Vec<ColumnInfo>)> {
        self.ensure_table(table)?;
        if table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "internal tables cannot be altered".to_string(),
            ));
        }
        if operations.is_empty() {
            return Err(DbError::InvalidDesign(
                "no alter-table operations were provided".to_string(),
            ));
        }

        let mut columns = self.columns(table)?;
        let mut statements = Vec::new();

        for operation in operations {
            match operation {
                AlterTableOp::AddColumn { column } => {
                    validate_ident(&column.name)?;
                    let type_name = normalize_type(&column.type_name)?;
                    if columns
                        .iter()
                        .any(|existing| existing.name.eq_ignore_ascii_case(&column.name))
                    {
                        return Err(DbError::InvalidDesign(format!(
                            "column already exists: {}",
                            column.name
                        )));
                    }
                    if column.primary_key {
                        return Err(DbError::InvalidDesign(
                            "cannot add a PRIMARY KEY column to an existing table".to_string(),
                        ));
                    }
                    if column.unique {
                        return Err(DbError::InvalidDesign(
                            "cannot add a UNIQUE column; create a unique index instead".to_string(),
                        ));
                    }
                    if column.not_null && column.default_value.is_none() {
                        return Err(DbError::InvalidDesign(format!(
                            "{} is NOT NULL, so it needs a default value",
                            column.name
                        )));
                    }
                    if let Some(references) = &column.references {
                        if column.not_null {
                            return Err(DbError::InvalidDesign(format!(
                                "{} has a foreign key, so it cannot be NOT NULL when added",
                                column.name
                            )));
                        }
                        self.validate_alter_reference(table, column, references, &columns)?;
                        let default_is_null = match column.default_value.as_deref() {
                            None => true,
                            Some(value) => value.trim().eq_ignore_ascii_case("NULL"),
                        };
                        if !default_is_null {
                            return Err(DbError::InvalidDesign(format!(
                                "{} has a foreign key, so its default must be NULL",
                                column.name
                            )));
                        }
                    }

                    statements.push(format!(
                        "ALTER TABLE {} ADD COLUMN {}",
                        quote_ident(table),
                        render_column_definition(column)?
                    ));
                    columns.push(ColumnInfo {
                        name: column.name.clone(),
                        type_name,
                        not_null: column.not_null,
                        primary_key: false,
                    });
                }
                AlterTableOp::RenameColumn { from, to } => {
                    validate_ident(from)?;
                    validate_ident(to)?;
                    if from.eq_ignore_ascii_case(to) {
                        return Err(DbError::InvalidDesign(
                            "rename source and target are the same".to_string(),
                        ));
                    }
                    if !columns
                        .iter()
                        .any(|column| column.name.eq_ignore_ascii_case(from))
                    {
                        return Err(DbError::UnknownColumn(from.clone()));
                    }
                    if columns
                        .iter()
                        .any(|column| column.name.eq_ignore_ascii_case(to))
                    {
                        return Err(DbError::InvalidDesign(format!(
                            "column already exists: {to}"
                        )));
                    }
                    statements.push(format!(
                        "ALTER TABLE {} RENAME COLUMN {} TO {}",
                        quote_ident(table),
                        quote_ident(from),
                        quote_ident(to)
                    ));
                    if let Some(column) = columns
                        .iter_mut()
                        .find(|column| column.name.eq_ignore_ascii_case(from))
                    {
                        column.name = to.clone();
                    }
                }
                AlterTableOp::DropColumn { name } => {
                    validate_ident(name)?;
                    let Some(column) = columns
                        .iter()
                        .find(|column| column.name.eq_ignore_ascii_case(name))
                    else {
                        return Err(DbError::UnknownColumn(name.clone()));
                    };
                    if column.primary_key {
                        return Err(DbError::InvalidDesign(
                            "cannot drop a primary key column".to_string(),
                        ));
                    }
                    if columns.len() <= 1 {
                        return Err(DbError::InvalidDesign(
                            "cannot drop the last column in a table".to_string(),
                        ));
                    }
                    if let Some(index) = self.indexes_for_table(table)?.into_iter().find(|index| {
                        index
                            .columns
                            .iter()
                            .any(|column| column.eq_ignore_ascii_case(name))
                    }) {
                        return Err(DbError::InvalidDesign(format!(
                            "column {name} is used by index {}; drop the index first",
                            index.name
                        )));
                    }
                    statements.push(format!(
                        "ALTER TABLE {} DROP COLUMN {}",
                        quote_ident(table),
                        quote_ident(name)
                    ));
                    columns.retain(|column| !column.name.eq_ignore_ascii_case(name));
                }
            }
        }

        Ok((statements, columns))
    }

    pub fn preview_alter_table(
        &self,
        table: &str,
        operations: &[AlterTableOp],
    ) -> Result<Vec<String>> {
        Ok(self.plan_alter_table(table, operations)?.0)
    }

    pub fn alter_table(&self, table: &str, operations: &[AlterTableOp]) -> Result<SchemaChange> {
        let before_ddl = self.table_ddl(table)?;
        let (statements, _) = self.plan_alter_table(table, operations)?;
        self.run_schema_change(
            &format!("alter_{table}"),
            statements,
            "alter_table",
            table,
            Some(serde_json::json!({ "ddl": before_ddl })),
            Some(serde_json::json!({ "operations": operations })),
        )
    }

    fn validate_alter_reference(
        &self,
        table: &str,
        column: &ColumnDesign,
        references: &ForeignKeyDesign,
        columns: &[ColumnInfo],
    ) -> Result<()> {
        validate_ident(&references.table)?;
        validate_ident(&references.column)?;
        if references.table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "cannot reference internal tables".to_string(),
            ));
        }

        let target_columns = if references.table.eq_ignore_ascii_case(table) {
            columns.to_vec()
        } else {
            if !self.table_exists(&references.table)? {
                return Err(DbError::InvalidDesign(format!(
                    "referenced table does not exist: {}",
                    references.table
                )));
            }
            self.columns(&references.table)?
        };
        if !target_columns
            .iter()
            .any(|target| target.name.eq_ignore_ascii_case(&references.column))
        {
            return Err(DbError::InvalidDesign(format!(
                "{} references missing column {}.{}",
                column.name, references.table, references.column
            )));
        }

        let on_delete = normalize_on_delete(references.on_delete.as_deref())?;
        if on_delete == "SET NULL" && column.not_null {
            return Err(DbError::InvalidDesign(format!(
                "{} is NOT NULL, so ON DELETE SET NULL is not allowed",
                column.name
            )));
        }
        Ok(())
    }

    pub fn create_index(
        &self,
        table: &str,
        name: &str,
        columns: &[String],
        unique: bool,
    ) -> Result<SchemaChange> {
        self.ensure_table(table)?;
        if table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "internal tables cannot be indexed".to_string(),
            ));
        }
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_")
            || name.to_ascii_lowercase().starts_with("sqlite_")
        {
            return Err(DbError::InvalidDesign(format!(
                "reserved index name: {name}"
            )));
        }
        if columns.is_empty() {
            return Err(DbError::InvalidDesign(
                "an index needs at least one column".to_string(),
            ));
        }
        let table_columns = self.columns(table)?;
        for column in columns {
            validate_ident(column)?;
            if !table_columns
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(column))
            {
                return Err(DbError::UnknownColumn(column.clone()));
            }
        }
        if self.any_object_exists(name)? {
            return Err(DbError::InvalidDesign(format!(
                "object already exists: {name}"
            )));
        }

        let column_sql = columns
            .iter()
            .map(|column| quote_ident(column))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "CREATE {}INDEX {} ON {} ({column_sql})",
            if unique { "UNIQUE " } else { "" },
            quote_ident(name),
            quote_ident(table)
        );
        self.run_schema_change(
            &format!("index_{name}"),
            vec![sql.clone()],
            "create_index",
            table,
            None,
            Some(serde_json::json!({
                "index": name,
                "table": table,
                "unique": unique,
                "columns": columns,
                "ddl": sql,
            })),
        )
    }

    pub fn drop_index(&self, name: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_")
            || name.to_ascii_lowercase().starts_with("sqlite_")
        {
            return Err(DbError::InvalidDesign(format!(
                "reserved index name: {name}"
            )));
        }
        let Some((table, Some(sql))) = self.object_record("index", name)? else {
            return Err(DbError::InvalidDesign(format!(
                "index not found or automatic: {name}"
            )));
        };
        self.run_schema_change(
            &format!("drop_index_{name}"),
            vec![format!("DROP INDEX {}", quote_ident(name))],
            "drop_index",
            &table,
            Some(serde_json::json!({ "index": name, "ddl": sql })),
            None,
        )
    }

    pub fn views(&self) -> Result<Vec<ViewInfo>> {
        let mut statement = self.conn.prepare(
            "SELECT name, sql FROM sqlite_master
             WHERE type = 'view'
               AND substr(name, 1, 6) <> '_dbgo_'
             ORDER BY name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ViewInfo {
                name: row.get(0)?,
                sql: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn create_view(&self, name: &str, select_sql: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(format!(
                "reserved view name: {name}"
            )));
        }
        if self.any_object_exists(name)? {
            return Err(DbError::InvalidDesign(format!(
                "object already exists: {name}"
            )));
        }
        let select_sql = select_sql.trim().trim_end_matches(';');
        if select_sql.is_empty() {
            return Err(DbError::InvalidDesign(
                "a view needs a SELECT statement".to_string(),
            ));
        }
        {
            let statement = self.conn.prepare(select_sql)?;
            if !statement.readonly() {
                return Err(DbError::ReadOnly);
            }
        }

        let sql = format!("CREATE VIEW {} AS {}", quote_ident(name), select_sql);
        self.run_schema_change(
            &format!("view_{name}"),
            vec![sql.clone()],
            "create_view",
            name,
            None,
            Some(serde_json::json!({ "view": name, "ddl": sql })),
        )
    }

    pub fn drop_view(&self, name: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(format!(
                "reserved view name: {name}"
            )));
        }
        let Some((_, Some(sql))) = self.object_record("view", name)? else {
            return Err(DbError::InvalidDesign(format!("view not found: {name}")));
        };
        self.run_schema_change(
            &format!("drop_view_{name}"),
            vec![format!("DROP VIEW {}", quote_ident(name))],
            "drop_view",
            name,
            Some(serde_json::json!({ "view": name, "ddl": sql })),
            None,
        )
    }

    pub fn migrations(&self) -> Result<Vec<MigrationInfo>> {
        self.ensure_migrations_table()?;
        let applied = {
            let mut statement = self
                .conn
                .prepare("SELECT name, sql, applied_at FROM _dbgo_migrations ORDER BY id")?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        let mut used = vec![false; applied.len()];
        let mut out = Vec::new();
        if let Some(project_dir) = &self.project_dir {
            let directory = project_dir.join("migrations");
            if directory.is_dir() {
                let mut files = std::fs::read_dir(&directory)?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("sql"))
                    })
                    .collect::<Vec<_>>();
                files.sort();

                for path in files {
                    let name = path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or_default()
                        .to_string();
                    let content = std::fs::read_to_string(&path)?;
                    let matched = applied.iter().position(|(applied_name, applied_sql, _)| {
                        applied_name == &name || applied_sql.trim() == content.trim()
                    });
                    let (is_applied, applied_at) = match matched {
                        Some(index) => {
                            used[index] = true;
                            (true, Some(applied[index].2.clone()))
                        }
                        None => (false, None),
                    };
                    out.push(MigrationInfo {
                        name,
                        applied: is_applied,
                        applied_at,
                        path: Some(path.display().to_string()),
                    });
                }
            }
        }

        for (index, (name, _, applied_at)) in applied.into_iter().enumerate() {
            if !used[index] {
                out.push(MigrationInfo {
                    name,
                    applied: true,
                    applied_at: Some(applied_at),
                    path: None,
                });
            }
        }
        Ok(out)
    }

    pub fn apply_migrations(&self) -> Result<MigrateResult> {
        let pending = self
            .migrations()?
            .into_iter()
            .filter(|migration| !migration.applied)
            .filter_map(|migration| migration.path.map(|path| (migration.name, path)))
            .collect::<Vec<_>>();

        let mut applied = Vec::new();
        for (name, path) in pending {
            let sql = std::fs::read_to_string(&path)?;
            if sql.trim().is_empty() {
                return Err(DbError::InvalidDesign(format!(
                    "migration is empty: {path}"
                )));
            }
            applied.push(self.apply_migration_sql(&name, &sql)?);
        }
        Ok(MigrateResult { applied })
    }

    fn apply_migration_sql(&self, name: &str, sql: &str) -> Result<AppliedMigration> {
        self.ensure_migrations_table()?;
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO _dbgo_migrations (name, sql) VALUES (?1, ?2)",
            rusqlite::params![name, sql],
        )?;
        record_audit(
            &transaction,
            &session_id,
            "migrations",
            None,
            "migration",
            None,
            Some(&serde_json::json!({ "name": name })),
        )?;
        transaction.commit()?;

        if let Err(error) = self.write_schema_md() {
            eprintln!("[dbgo] warning: could not update schema.md: {error}");
        }

        Ok(AppliedMigration {
            name: name.to_string(),
            session_id,
            backup_path: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn run_schema_change(
        &self,
        slug: &str,
        statements: Vec<String>,
        action: &str,
        object_name: &str,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
    ) -> Result<SchemaChange> {
        if statements.is_empty() {
            return Err(DbError::InvalidDesign(
                "no schema statements to apply".to_string(),
            ));
        }

        self.ensure_migrations_table()?;
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let sql = format!(
            "{};",
            statements
                .iter()
                .map(|statement| statement.trim().trim_end_matches(';'))
                .collect::<Vec<_>>()
                .join(";\n")
        );

        let transaction = self.conn.unchecked_transaction()?;
        let migration_id: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM _dbgo_migrations",
            [],
            |row| row.get(0),
        )?;
        let migration_name = format!("{migration_id:04}_{}", sanitize_slug(slug));
        transaction.execute_batch(&sql)?;
        transaction.execute(
            "INSERT INTO _dbgo_migrations (id, name, sql) VALUES (?1, ?2, ?3)",
            rusqlite::params![migration_id, migration_name, sql],
        )?;
        record_audit(
            &transaction,
            &session_id,
            object_name,
            None,
            action,
            before.as_ref(),
            after.as_ref(),
        )?;
        transaction.commit()?;

        let migration_path = if let Some(project_dir) = &self.project_dir {
            match write_migration_named(project_dir, &migration_name, action, object_name, &sql) {
                Ok(path) => Some(path.display().to_string()),
                Err(error) => {
                    eprintln!("[dbgo] warning: could not write migration file: {error}");
                    None
                }
            }
        } else {
            None
        };

        if let Err(error) = self.write_schema_md() {
            eprintln!("[dbgo] warning: could not update schema.md: {error}");
        }

        Ok(SchemaChange {
            session_id,
            backup_path: None,
            migration_id,
            migration_name,
            migration_path,
            statements,
        })
    }

    fn object_record(&self, kind: &str, name: &str) -> Result<Option<(String, Option<String>)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT tbl_name, sql FROM sqlite_master
                 WHERE type = ?1 AND name = ?2 COLLATE NOCASE",
                [kind, name],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?)
    }

    fn any_object_exists(&self, name: &str) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE name = ?1 COLLATE NOCASE
                  AND type IN ('table', 'view', 'index', 'trigger')
             )",
            [name],
            |row| row.get::<_, i64>(0),
        )? != 0)
    }

    fn validate_foreign_key(
        &self,
        design: &TableDesign,
        column: &ColumnDesign,
        references: &ForeignKeyDesign,
    ) -> Result<()> {
        validate_ident(&references.table)?;
        validate_ident(&references.column)?;
        if references.table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "cannot reference internal tables".to_string(),
            ));
        }

        if references.table.eq_ignore_ascii_case(design.name.trim()) {
            if !design
                .columns
                .iter()
                .any(|candidate| candidate.name.eq_ignore_ascii_case(&references.column))
            {
                return Err(DbError::InvalidDesign(format!(
                    "{} references missing column {}.{}",
                    column.name, references.table, references.column
                )));
            }
        } else {
            if !self.table_exists(&references.table)? {
                return Err(DbError::InvalidDesign(format!(
                    "referenced table does not exist: {}",
                    references.table
                )));
            }
            let target_columns = self.columns(&references.table)?;
            if !target_columns
                .iter()
                .any(|candidate| candidate.name.eq_ignore_ascii_case(&references.column))
            {
                return Err(DbError::InvalidDesign(format!(
                    "{} references missing column {}.{}",
                    column.name, references.table, references.column
                )));
            }
        }

        let on_delete = normalize_on_delete(references.on_delete.as_deref())?;
        if on_delete == "SET NULL" && column.not_null {
            return Err(DbError::InvalidDesign(format!(
                "{} is NOT NULL, so ON DELETE SET NULL is not allowed",
                column.name
            )));
        }
        Ok(())
    }

    fn table_exists(&self, table: &str) -> Result<bool> {
        self.conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master
                    WHERE type = 'table' AND name = ?1 COLLATE NOCASE
                 )",
                [table],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    fn ensure_table(&self, table: &str) -> Result<()> {
        if self.table_exists(table)? {
            Ok(())
        } else {
            Err(DbError::TableNotFound(table.to_string()))
        }
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn cell_from_value_ref(value: ValueRef<'_>) -> CellValue {
    match value {
        ValueRef::Null => CellValue::Null,
        ValueRef::Integer(n) => CellValue::Integer { value: n },
        ValueRef::Real(f) => CellValue::Real { value: f },
        ValueRef::Text(t) => CellValue::Text {
            value: String::from_utf8_lossy(t).into_owned(),
        },
        ValueRef::Blob(b) => CellValue::Blob {
            value: base64::engine::general_purpose::STANDARD.encode(b),
            size: b.len(),
        },
    }
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn validate_ident(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(DbError::InvalidDesign(
            "identifiers must be 1-64 characters".to_string(),
        ));
    }
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return Err(DbError::InvalidDesign("identifier is empty".to_string()));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(DbError::InvalidDesign(format!(
            "invalid identifier: {name}"
        )));
    }
    if !characters.all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return Err(DbError::InvalidDesign(format!(
            "invalid identifier: {name}"
        )));
    }
    Ok(())
}

const ALLOWED_TYPES: &[&str] = &[
    "TEXT",
    "INTEGER",
    "REAL",
    "BLOB",
    "NUMERIC",
    "BOOLEAN",
    "DATE",
    "TIMESTAMP",
    "JSON",
];

fn normalize_type(value: &str) -> Result<String> {
    let normalized = value.trim().to_ascii_uppercase();
    if ALLOWED_TYPES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(DbError::InvalidDesign(format!(
            "unsupported column type: {value}"
        )))
    }
}

fn normalize_on_delete(value: Option<&str>) -> Result<String> {
    let normalized = value.unwrap_or("NO ACTION").trim().to_ascii_uppercase();
    match normalized.as_str() {
        "NO ACTION" | "CASCADE" | "SET NULL" | "RESTRICT" => Ok(normalized),
        _ => Err(DbError::InvalidDesign(format!(
            "unsupported ON DELETE action: {normalized}"
        ))),
    }
}

fn render_default(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok("''".to_string());
    }
    let upper = trimmed.to_ascii_uppercase();
    match upper.as_str() {
        "NULL" => Ok("NULL".to_string()),
        "CURRENT_TIMESTAMP" | "CURRENT_DATE" | "CURRENT_TIME" => Ok(upper),
        "TRUE" => Ok("1".to_string()),
        "FALSE" => Ok("0".to_string()),
        _ => {
            if trimmed.parse::<f64>().is_ok()
                && !trimmed.eq_ignore_ascii_case("nan")
                && !trimmed.eq_ignore_ascii_case("inf")
                && !trimmed.eq_ignore_ascii_case("-inf")
            {
                Ok(trimmed.to_string())
            } else {
                Ok(format!("'{}'", trimmed.replace('\'', "''")))
            }
        }
    }
}

fn render_column_definition(column: &ColumnDesign) -> Result<String> {
    let type_name = normalize_type(&column.type_name)?;
    let mut parts = vec![quote_ident(&column.name), type_name];
    if column.primary_key {
        parts.push("PRIMARY KEY".to_string());
    }
    if column.not_null || column.primary_key {
        parts.push("NOT NULL".to_string());
    }
    if column.unique && !column.primary_key {
        parts.push("UNIQUE".to_string());
    }
    if let Some(default_value) = &column.default_value {
        parts.push(format!("DEFAULT {}", render_default(default_value)?));
    }
    if let Some(references) = &column.references {
        let mut reference_sql = format!(
            "REFERENCES {}({})",
            quote_ident(&references.table),
            quote_ident(&references.column)
        );
        let on_delete = normalize_on_delete(references.on_delete.as_deref())?;
        if on_delete != "NO ACTION" {
            reference_sql.push_str(&format!(" ON DELETE {on_delete}"));
        }
        parts.push(reference_sql);
    }
    Ok(parts.join(" "))
}

fn render_create_table(design: &TableDesign) -> Result<String> {
    let mut definitions = Vec::with_capacity(design.columns.len());
    for column in &design.columns {
        definitions.push(format!("  {}", render_column_definition(column)?));
    }
    Ok(format!(
        "CREATE TABLE {} (\n{}\n)",
        quote_ident(design.name.trim()),
        definitions.join(",\n")
    ))
}

fn render_index_for_column(table: &str, column: &str) -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {} ON {} ({})",
        quote_ident(&format!("idx_{table}_{column}")),
        quote_ident(table),
        quote_ident(column)
    )
}

fn backup_database(path: &Path) -> Result<PathBuf> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("database");
    let backup = path.with_file_name(format!("{file_name}.bak-{}", now_millis()));
    backup_database_to(path, &backup)?;
    Ok(backup)
}

fn backup_database_to(source: &Path, destination: &Path) -> Result<()> {
    let connection = Connection::open(source)?;
    connection.execute("VACUUM INTO ?1", [destination.to_string_lossy().as_ref()])?;
    Ok(())
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn detect_project_dir(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let config_path = parent.join("dbgo.json");
    let content = std::fs::read_to_string(config_path).ok()?;
    let config: ProjectConfig = serde_json::from_str(&content).ok()?;
    if parent.join(&config.database) == path {
        Some(parent.to_path_buf())
    } else {
        None
    }
}

fn read_project_config(project_dir: &Path) -> Result<ProjectConfig> {
    let path = project_dir.join("dbgo.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|_| DbError::NotAProject(project_dir.display().to_string()))?;
    serde_json::from_str(&content)
        .map_err(|error| DbError::InvalidDesign(format!("invalid dbgo.json: {error}")))
}

fn write_project_config(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    let config_json = serde_json::to_string_pretty(config)?;
    std::fs::write(project_dir.join("dbgo.json"), config_json)?;
    Ok(())
}

fn write_project_files(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    write_project_config(project_dir, config)?;
    std::fs::write(project_dir.join("AGENTS.md"), agents_markdown(config))?;
    std::fs::write(
        project_dir.join("schema.md"),
        "# Schema\n\nNo tables yet.\n",
    )?;
    Ok(())
}

fn looks_like_dbgo_agents(text: &str) -> bool {
    text.trim_start().starts_with("# Working with ")
}

fn looks_like_dbgo_schema(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("# Schema")
        && (trimmed.contains("No tables yet.")
            || trimmed.contains("| Column | Type | Constraints |"))
}

fn merge_marked_document(
    existing: Option<&str>,
    generated: &str,
    marker: &str,
    looks_generated: fn(&str) -> bool,
) -> String {
    let Some(existing) = existing else {
        return generated.to_string();
    };
    if existing.trim().is_empty() {
        return generated.to_string();
    }

    let start = format!("<!-- {marker}:start -->");
    let end = format!("<!-- {marker}:end -->");
    if let Some(start_index) = existing.find(&start) {
        let section = format!("{start}\n{}\n{end}", generated.trim_end());
        if let Some(relative_end) = existing[start_index..].find(&end) {
            let end_index = start_index + relative_end;
            let mut merged = String::new();
            merged.push_str(&existing[..start_index]);
            merged.push_str(&section);
            merged.push_str(&existing[end_index + end.len()..]);
            return merged;
        }
        let mut merged = existing[..start_index].trim_end().to_string();
        merged.push_str("\n\n");
        merged.push_str(&section);
        merged.push('\n');
        return merged;
    }

    if looks_generated(existing) {
        return generated.to_string();
    }

    let mut merged = existing.trim_end().to_string();
    merged.push_str("\n\n");
    merged.push_str(&start);
    merged.push('\n');
    merged.push_str(generated.trim_end());
    merged.push('\n');
    merged.push_str(&end);
    merged.push('\n');
    merged
}

fn agents_markdown(config: &ProjectConfig) -> String {
    const TEMPLATE: &str = r#"# Working with {{NAME}}

The database is `{{DATABASE}}` in this folder. Never edit the database file directly.

## Discover the schema

- `dbgo graph . --json` — full tables, columns, and relationships
- `dbgo schema . companies --json` — columns for one table
- `dbgo tables .` — table list
- `schema.md` is a generated snapshot; the CLI is authoritative.

## Read

- `dbgo query . "select * from companies limit 20" --json`
- `dbgo page . companies --limit 50`
- `dbgo count . companies`
- `dbgo explain . "select ..." --json` — query plan without executing it
- `dbgo stats . --json` — page, table, and index sizes

## Write data

Use JSONL files, one JSON object per line:

```jsonl
{"name":"Acme","domain":"acme.test","city":"Berlin"}
{"name":"Globex","domain":"globex.test","city":"Austin"}
```

```bash
dbgo insert . companies --file rows.jsonl
dbgo upsert . companies --key domain --file rows.jsonl
```

Updates and deletes use a JSON filter and require `--yes`:

```bash
dbgo update . companies \
  --where '{"column":"domain","op":"eq","value":"acme.test"}' \
  --set '{"city":"Munich"}' \
  --yes

dbgo delete . companies \
  --where '{"column":"domain","op":"eq","value":"globex.test"}' \
  --yes
```

- `upsert` needs a UNIQUE or PRIMARY KEY constraint on the key column.
- Updating primary key columns is not supported.
- Every write is audited. Backups are manual (`dbgo backup .`).

`--where` also accepts an AND/OR/NOT expression tree:

```json
{
  "type": "and",
  "filters": [
    { "type": "condition", "column": "id", "op": "gte", "value": "2" },
    { "type": "or", "filters": [
      { "type": "condition", "column": "city", "op": "eq", "value": "Berlin" },
      { "type": "condition", "column": "city", "op": "eq", "value": "Paris" }
    ] }
  ]
}
```

Node types: `condition`, `and`, `or`, `not`. Old single-condition JSON still
works. Values may be strings, numbers, or booleans.

## Execute arbitrary SQL

`dbgo exec` is read-only unless `--write` and `--yes` are provided.

```bash
dbgo exec . "select * from companies limit 20"
dbgo exec . "update employees set salary = salary + 1" --write --yes
dbgo exec . --file script.sql --write --yes
dbgo exec . "update companies set city = 'Berlin' returning id, city" \
  --write --yes --returning
dbgo exec . "vacuum" --write --yes --no-transaction
```

- Normal writes run in one transaction.
- `--no-transaction` is required for VACUUM, ATTACH, write PRAGMAs, or scripts
  with their own BEGIN/COMMIT.
- `--returning` executes a single statement and returns rows.
- Writes get one audit session; no automatic backup.
- `--limit` caps returned rows.

## DDL and maintenance

```bash
dbgo drop-table . legacy_companies --yes
dbgo rename-table . companies organizations
dbgo trigger create . audit_companies --sql "create trigger audit_companies after update on companies begin select 1; end"
dbgo trigger create . audit_companies --file trigger.sql
dbgo trigger list . companies
dbgo trigger drop . audit_companies --yes
dbgo vacuum .
dbgo analyze .
dbgo pragma . user_version
dbgo pragma . user_version=3 --yes
```

- Drop table/trigger require `--yes`.
- Schema changes write migrations and refresh `schema.md`.
- `vacuum`, `analyze`, and write PRAGMAs are audited but do not create migrations.
- Use `dbgo exec` for column type changes, table rebuilds, ATTACH, and other
  arbitrary DDL.

## Export

```bash
dbgo export . companies --format csv --output companies.csv
dbgo export . companies --format json --where '{"column":"id","op":"gte","value":1}'
dbgo export . --query "select * from companies order by id" --format json --output result.json
```

- Exports are read-only and do not create backups.
- Omit `--output` to write data to stdout.
- BLOB values are base64-encoded.

## Import

```bash
dbgo import . people --file people.csv --preview
dbgo import . people --file people.csv
dbgo import . --file people.csv --create-table
dbgo import . people --file people.csv --upsert-key domain
dbgo import . people --file people.csv --map "Full Name=name"
```

- CSV, JSON, and JSONL are supported.
- Empty CSV cells become NULL.
- Default mapping matches source and target column names case-insensitively.
- `--map` overrides matching columns without disabling default mapping; use
  `--map "Source="` to skip a source column.
- Create-table imports infer column types and sanitize column names.
- Upsert requires a PRIMARY KEY or UNIQUE constraint on the key column.
- Imports write directly; backups are manual (`dbgo backup .`).

## D1 export

```bash
dbgo d1 export . --output ./d1 --name my-db
```

- Generates `schema.sql`, `data-*.sql`, `wrangler.toml`, `manifest.json`, and `README.md`.
- Internal `_dbgo_*` tables are excluded.
- Tables are ordered by foreign keys; data is split into batches.
- Deploy manually with Wrangler; DBGo does not deploy.
- Use `--force` to overwrite a non-empty output directory.

## GUI edits

The desktop app stages row edits and writes them only when the user clicks
**Save changes**. Each save creates one audit session. Backups are manual.
The app does not reload the current page while edits are staged, so an agent
can write safely and the user decides when to reload.

Cells can be edited inline, selected in bulk, and copied/pasted as TSV. All of
that stays pending until **Save changes**.

## Create tables

Write a JSON design file, for example `table.json`:

```json
{
  "name": "companies",
  "columns": [
    {
      "name": "id",
      "type_name": "INTEGER",
      "primary_key": true,
      "not_null": true,
      "unique": false,
      "default_value": null,
      "references": null
    },
    {
      "name": "name",
      "type_name": "TEXT",
      "primary_key": false,
      "not_null": true,
      "unique": false,
      "default_value": null,
      "references": null
    }
  ],
  "index_foreign_keys": true
}
```

Then run:

```bash
dbgo create-table . --file table.json
```

## Alter tables

An operations file keeps multiple changes atomic:

```json
{
  "operations": [
    {
      "op": "add_column",
      "column": {
        "name": "country",
        "type_name": "TEXT",
        "primary_key": false,
        "not_null": true,
        "unique": false,
        "default_value": "US",
        "references": null
      }
    },
    { "op": "rename_column", "from": "city", "to": "location" },
    { "op": "drop_column", "name": "legacy_id" }
  ]
}
```

```bash
dbgo alter-table . companies --file alter.json
dbgo alter-table . companies --add-column '{"name":"note","type_name":"TEXT","primary_key":false,"not_null":false,"unique":false,"default_value":null,"references":null}'
dbgo alter-table . companies --rename-column city=location
dbgo alter-table . companies --drop-column legacy_id --yes
dbgo alter-table . companies --file alter.json --sql-only
```

- `--sql-only` validates and prints SQL without writing.
- Dropping a column requires `--yes`.
- Adding a NOT NULL column requires a default.
- Adding a primary-key or UNIQUE column is rejected; create a unique index instead.
- A dropped column cannot be used by an index; drop the index first.

## Indexes and views

```bash
dbgo index create . companies --name idx_companies_city --columns city
dbgo index create . companies --name idx_companies_domain --columns domain --unique
dbgo index list . companies
dbgo index drop . companies idx_companies_city --yes

dbgo view create . active_companies --sql "select * from companies where active = 1"
dbgo view list .
dbgo view drop . active_companies --yes
```

## Migrations

Schema commands write numbered `.sql` files into `migrations/` and record them in
`_dbgo_migrations`. Apply any files added by hand or by another tool:

```bash
dbgo migrations .
dbgo migrate .
```

Migrations run in filename order. Each applied file gets an audit session. Do
not edit a migration that is already recorded as applied.

## Backups and audit

- Backups are manual. Use `dbgo backup .` when you want a snapshot.
- `dbgo backups .` lists snapshots, newest first.
- `dbgo restore . --latest --yes` restores the newest snapshot.
- `dbgo restore . --backup <path> --yes` restores one specific snapshot.
- A restore validates the snapshot. Take a backup first if you need to undo it.
- `dbgo sessions .` lists recent write sessions and their action counts.

## Rules

- Use `dbgo`; never write the SQLite file directly.
- Run `dbgo graph . --json` before changing the schema.
- Use `--json` or `--jsonl` for machine-readable output.
- Run commands from this folder and use `.` as the database path.
- Use `--yes` for update/delete, drop-table, trigger drop, `exec --write`, and
  write PRAGMAs.
- Use `--yes` when dropping columns, indexes, or views.
- Use `--yes` when restoring a backup.
- Prefer `upsert` with a natural key over blind inserts.
- Do not update primary key columns.
- Do not edit a migration that is already applied; add a new numbered file instead.
- Check `dbgo migrations .` before and after changing the schema manually.
"#;

    TEMPLATE
        .replace("{{NAME}}", &config.name)
        .replace("{{DATABASE}}", &config.database)
}

fn sanitize_slug(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    slug.trim_matches('_').to_string()
}

fn write_migration_named(
    project_dir: &Path,
    name: &str,
    action: &str,
    object_name: &str,
    sql: &str,
) -> Result<PathBuf> {
    let migrations = project_dir.join("migrations");
    std::fs::create_dir_all(&migrations)?;
    let path = migrations.join(format!("{name}.sql"));
    std::fs::write(&path, format!("-- {action} {object_name}\n{sql}\n"))?;
    Ok(path)
}

fn schema_markdown(graph: &SchemaGraph) -> String {
    let mut output = String::from("# Schema\n\n");
    if graph.tables.is_empty() {
        output.push_str("No tables yet.\n");
        return output;
    }

    for table in &graph.tables {
        output.push_str(&format!("## {}\n\n", md_escape(&table.name)));
        output.push_str("| Column | Type | Constraints |\n|---|---|---|\n");
        for column in &table.columns {
            let mut constraints = Vec::new();
            if column.primary_key {
                constraints.push("PK");
            }
            if column.foreign_key {
                constraints.push("FK");
            }
            if column.not_null {
                constraints.push("NOT NULL");
            }
            output.push_str(&format!(
                "| {} | {} | {} |\n",
                md_escape(&column.name),
                md_escape(&column.type_name),
                constraints.join(", ")
            ));
        }
        output.push('\n');
    }

    if !graph.relationships.is_empty() {
        output.push_str("## Relationships\n\n");
        for relationship in &graph.relationships {
            output.push_str(&format!(
                "- `{}.{}` → `{}.{}` (ON DELETE {})\n",
                md_escape(&relationship.from_table),
                md_escape(&relationship.from_column),
                md_escape(&relationship.to_table),
                md_escape(&relationship.to_column),
                relationship.on_delete
            ));
        }
        output.push('\n');
    }

    output
}

fn md_escape(value: &str) -> String {
    value.replace('|', "\\|")
}

fn filter_condition_clause(
    column_name: &str,
    op: FilterOp,
    value: Option<&str>,
    columns: &[ColumnInfo],
) -> Result<(String, Vec<rusqlite::types::Value>)> {
    if !columns.iter().any(|column| column.name == column_name) {
        return Err(DbError::UnknownColumn(column_name.to_string()));
    }
    let column = quote_ident(column_name);
    let mut params = Vec::new();

    let clause = match op {
        FilterOp::Contains => {
            let value = value.ok_or(DbError::MissingFilterValue(op))?;
            params.push(rusqlite::types::Value::Text(format!(
                "%{}%",
                escape_like(value)
            )));
            format!("CAST({column} AS TEXT) LIKE ? ESCAPE '\\'")
        }
        FilterOp::Eq
        | FilterOp::Neq
        | FilterOp::Gt
        | FilterOp::Gte
        | FilterOp::Lt
        | FilterOp::Lte => {
            let operator = match op {
                FilterOp::Eq => "=",
                FilterOp::Neq => "<>",
                FilterOp::Gt => ">",
                FilterOp::Gte => ">=",
                FilterOp::Lt => "<",
                FilterOp::Lte => "<=",
                _ => unreachable!(),
            };
            let value = value.ok_or(DbError::MissingFilterValue(op))?;
            params.push(rusqlite::types::Value::Text(value.to_string()));
            format!("{column} {operator} ?")
        }
        FilterOp::IsNull => format!("{column} IS NULL"),
        FilterOp::NotNull => format!("{column} IS NOT NULL"),
    };

    Ok((clause, params))
}

fn filter_clause(
    filter: &FilterSpec,
    columns: &[ColumnInfo],
) -> Result<(String, Vec<rusqlite::types::Value>)> {
    let (clause, params) =
        filter_condition_clause(&filter.column, filter.op, filter.value.as_deref(), columns)?;
    Ok((format!(" WHERE {clause}"), params))
}

fn filter_expr_clause(
    filter: &FilterExpr,
    columns: &[ColumnInfo],
) -> Result<(String, Vec<rusqlite::types::Value>)> {
    match filter {
        FilterExpr::Condition { column, op, value } => {
            let (clause, params) = filter_condition_clause(column, *op, value.as_deref(), columns)?;
            Ok((format!("({clause})"), params))
        }
        FilterExpr::And { filters } | FilterExpr::Or { filters } => {
            if filters.is_empty() {
                return Err(DbError::InvalidDesign(
                    "AND/OR filters need at least one condition".to_string(),
                ));
            }
            let joiner = if matches!(filter, FilterExpr::And { .. }) {
                " AND "
            } else {
                " OR "
            };
            let mut clauses = Vec::with_capacity(filters.len());
            let mut params = Vec::new();
            for child in filters {
                let (clause, child_params) = filter_expr_clause(child, columns)?;
                clauses.push(clause);
                params.extend(child_params);
            }
            Ok((format!("({})", clauses.join(joiner)), params))
        }
        FilterExpr::Not { filter } => {
            let (clause, params) = filter_expr_clause(filter, columns)?;
            Ok((format!("NOT {clause}"), params))
        }
    }
}

fn validate_json_column(name: &str, columns: &[ColumnInfo]) -> Result<()> {
    if columns.iter().any(|column| column.name == name) {
        Ok(())
    } else {
        Err(DbError::UnknownColumn(name.to_string()))
    }
}

fn validate_json_columns(row: &JsonObject, columns: &[ColumnInfo]) -> Result<()> {
    for name in row.keys() {
        validate_json_column(name, columns)?;
    }
    Ok(())
}

fn validate_update_values(values: &JsonObject, columns: &[ColumnInfo]) -> Result<()> {
    for name in values.keys() {
        validate_json_column(name, columns)?;
        if columns
            .iter()
            .any(|column| column.name == *name && column.primary_key)
        {
            return Err(DbError::InvalidDesign(
                "updating primary key columns is not supported".to_string(),
            ));
        }
    }
    Ok(())
}

fn json_to_sql(value: &serde_json::Value) -> rusqlite::types::Value {
    match value {
        serde_json::Value::Null => rusqlite::types::Value::Null,
        serde_json::Value::Bool(flag) => rusqlite::types::Value::Integer(if *flag { 1 } else { 0 }),
        serde_json::Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                rusqlite::types::Value::Integer(integer)
            } else if let Some(real) = number.as_f64() {
                rusqlite::types::Value::Real(real)
            } else {
                rusqlite::types::Value::Text(number.to_string())
            }
        }
        serde_json::Value::String(text) => rusqlite::types::Value::Text(text.clone()),
        other => rusqlite::types::Value::Text(other.to_string()),
    }
}

fn cell_to_json(value: ValueRef<'_>) -> serde_json::Value {
    match value {
        ValueRef::Null => serde_json::Value::Null,
        ValueRef::Integer(n) => serde_json::Value::Number(n.into()),
        ValueRef::Real(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(f.to_string())),
        ValueRef::Text(t) => serde_json::Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => serde_json::Value::String(format!("<blob {} bytes>", b.len())),
    }
}

fn export_json(value: ValueRef<'_>) -> serde_json::Value {
    match value {
        ValueRef::Null => serde_json::Value::Null,
        ValueRef::Integer(n) => serde_json::Value::Number(n.into()),
        ValueRef::Real(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(f.to_string())),
        ValueRef::Text(t) => serde_json::Value::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => {
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
    }
}

fn csv_value(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(n) => n.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(b) => base64::engine::general_purpose::STANDARD.encode(b),
    }
}

fn csv_escape(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn preview_import(
    path: &Path,
    format: ImportFormat,
    has_headers: bool,
    delimiter: Option<&str>,
) -> Result<ImportPreview> {
    let (source_columns, source_rows) = read_import_source(path, format, has_headers, delimiter)?;
    let columns = infer_import_columns(&source_columns, &source_rows, format);
    let rows = source_rows.iter().take(20).cloned().collect();
    Ok(ImportPreview {
        format,
        columns,
        rows,
        total_rows: source_rows.len(),
        suggested_table: suggest_table_name(path),
        source_path: path.display().to_string(),
    })
}

fn read_import_source(
    path: &Path,
    format: ImportFormat,
    has_headers: bool,
    delimiter: Option<&str>,
) -> Result<(Vec<String>, Vec<JsonObject>)> {
    match format {
        ImportFormat::Csv => read_csv_source(path, has_headers, delimiter),
        ImportFormat::Json => read_json_source(path),
        ImportFormat::Jsonl => read_jsonl_source(path),
    }
}

fn read_csv_source(
    path: &Path,
    has_headers: bool,
    delimiter: Option<&str>,
) -> Result<(Vec<String>, Vec<JsonObject>)> {
    let delimiter = parse_csv_delimiter(delimiter)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(has_headers)
        .flexible(true)
        .delimiter(delimiter)
        .from_path(path)
        .map_err(csv_error)?;

    if has_headers {
        let headers = reader
            .headers()
            .map_err(csv_error)?
            .iter()
            .map(|header| header.to_string())
            .collect::<Vec<_>>();
        if headers.is_empty() {
            return Err(DbError::InvalidDesign(
                "CSV file has no header row".to_string(),
            ));
        }
        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record.map_err(csv_error)?;
            let mut row = JsonObject::new();
            for (index, header) in headers.iter().enumerate() {
                row.insert(
                    header.clone(),
                    serde_json::Value::String(record.get(index).unwrap_or("").to_string()),
                );
            }
            rows.push(row);
        }
        Ok((headers, rows))
    } else {
        let mut records = Vec::new();
        let mut width = 0;
        for record in reader.records() {
            let record = record.map_err(csv_error)?;
            width = width.max(record.len());
            records.push(
                record
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>(),
            );
        }
        let headers = (0..width)
            .map(|index| format!("column_{}", index + 1))
            .collect::<Vec<_>>();
        let rows = records
            .into_iter()
            .map(|record| {
                let mut row = JsonObject::new();
                for (index, header) in headers.iter().enumerate() {
                    row.insert(
                        header.clone(),
                        serde_json::Value::String(record.get(index).cloned().unwrap_or_default()),
                    );
                }
                row
            })
            .collect();
        Ok((headers, rows))
    }
}

fn read_json_source(path: &Path) -> Result<(Vec<String>, Vec<JsonObject>)> {
    let content = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| DbError::InvalidDesign(format!("invalid JSON: {error}")))?;
    let rows = match value {
        serde_json::Value::Array(items) => items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                item.as_object().cloned().ok_or_else(|| {
                    DbError::InvalidDesign(format!("JSON row {} is not an object", index + 1))
                })
            })
            .collect::<Result<Vec<_>>>()?,
        serde_json::Value::Object(map) => vec![map],
        _ => {
            return Err(DbError::InvalidDesign(
                "JSON import must be an object or an array of objects".to_string(),
            ))
        }
    };
    let columns = collect_import_columns(&rows);
    Ok((columns, rows))
}

fn read_jsonl_source(path: &Path) -> Result<(Vec<String>, Vec<JsonObject>)> {
    let content = std::fs::read_to_string(path)?;
    let mut rows = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| DbError::InvalidDesign(format!("line {}: {error}", index + 1)))?;
        let object = value.as_object().cloned().ok_or_else(|| {
            DbError::InvalidDesign(format!("line {}: expected a JSON object", index + 1))
        })?;
        rows.push(object);
    }
    let columns = collect_import_columns(&rows);
    Ok((columns, rows))
}

fn collect_import_columns(rows: &[JsonObject]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut columns = Vec::new();
    for row in rows {
        for key in row.keys() {
            if seen.insert(key.clone()) {
                columns.push(key.clone());
            }
        }
    }
    columns
}

fn parse_csv_delimiter(value: Option<&str>) -> Result<u8> {
    match value {
        None => Ok(b','),
        Some(value) if value == "\\t" || value.eq_ignore_ascii_case("tab") => Ok(b'\t'),
        Some(value) if value.len() == 1 => Ok(value.as_bytes()[0]),
        Some(value) => Err(DbError::InvalidDesign(format!(
            "CSV delimiter must be one character: {value}"
        ))),
    }
}

fn csv_error(error: csv::Error) -> DbError {
    DbError::InvalidDesign(format!("CSV error: {error}"))
}

fn infer_import_columns(
    source_columns: &[String],
    rows: &[JsonObject],
    format: ImportFormat,
) -> Vec<ImportColumn> {
    source_columns
        .iter()
        .enumerate()
        .map(|(index, source)| ImportColumn {
            source: source.clone(),
            suggested_name: sanitize_import_identifier(source, &format!("column_{}", index + 1)),
            inferred_type: infer_import_type(rows.iter().filter_map(|row| row.get(source)), format),
        })
        .collect()
}

fn infer_import_type<'a>(
    values: impl Iterator<Item = &'a serde_json::Value>,
    format: ImportFormat,
) -> String {
    let mut all_integers = true;
    let mut all_numbers = true;
    let mut all_booleans = true;
    let mut count = 0;

    for value in values {
        if value.is_null() {
            continue;
        }
        if let serde_json::Value::String(raw) = value {
            if format == ImportFormat::Csv && raw.is_empty() {
                continue;
            }
        }
        count += 1;

        match value {
            serde_json::Value::Bool(_) => {
                all_integers = false;
                all_numbers = false;
            }
            serde_json::Value::Number(number) => {
                all_booleans = false;
                if !(number.is_i64() || number.is_u64()) {
                    all_integers = false;
                }
                if number.as_f64().is_none() {
                    all_numbers = false;
                }
            }
            serde_json::Value::String(raw) => {
                if format == ImportFormat::Csv && !is_boolean_text(raw) {
                    all_booleans = false;
                }
                if format == ImportFormat::Csv && !is_canonical_integer(raw) {
                    all_integers = false;
                }
                if format == ImportFormat::Csv {
                    match raw.trim().parse::<f64>() {
                        Ok(value) if value.is_finite() => {}
                        _ => all_numbers = false,
                    }
                    if is_padded_number(raw) {
                        all_numbers = false;
                    }
                } else {
                    all_integers = false;
                    all_numbers = false;
                    all_booleans = false;
                }
            }
            _ => {
                all_integers = false;
                all_numbers = false;
                all_booleans = false;
            }
        }
    }

    if count == 0 {
        "TEXT".to_string()
    } else if all_booleans {
        "BOOLEAN".to_string()
    } else if all_integers {
        "INTEGER".to_string()
    } else if all_numbers {
        "REAL".to_string()
    } else {
        "TEXT".to_string()
    }
}

fn is_boolean_text(value: &str) -> bool {
    value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false")
}

fn is_canonical_integer(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    let digits = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    !digits.is_empty()
        && digits.chars().all(|character| character.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
}

fn is_padded_number(value: &str) -> bool {
    let value = value.trim();
    let digits = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    let integer_part = digits.split('.').next().unwrap_or(digits);
    integer_part.len() > 1
        && integer_part.starts_with('0')
        && integer_part
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn convert_csv_import_value(
    raw: &str,
    column: &ColumnInfo,
    row: usize,
) -> Result<serde_json::Value> {
    if raw.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    let type_name = column.type_name.to_ascii_uppercase();
    let trimmed = raw.trim();

    if type_name.contains("INT") {
        trimmed
            .parse::<i64>()
            .map(|value| serde_json::Value::Number(value.into()))
            .map_err(|_| {
                DbError::InvalidDesign(format!(
                    "row {row}, column {}: expected an integer, got {raw:?}",
                    column.name
                ))
            })
    } else if type_name.contains("REAL")
        || type_name.contains("NUM")
        || type_name.contains("FLOAT")
        || type_name.contains("DOUBLE")
        || type_name.contains("DEC")
    {
        let value = trimmed.parse::<f64>().map_err(|_| {
            DbError::InvalidDesign(format!(
                "row {row}, column {}: expected a number, got {raw:?}",
                column.name
            ))
        })?;
        if !value.is_finite() {
            return Err(DbError::InvalidDesign(format!(
                "row {row}, column {}: expected a finite number",
                column.name
            )));
        }
        Ok(serde_json::Value::Number(
            serde_json::Number::from_f64(value).expect("finite numbers are valid JSON"),
        ))
    } else if type_name.contains("BOOL") {
        match trimmed.to_ascii_lowercase().as_str() {
            "1" | "true" => Ok(serde_json::Value::Number(1.into())),
            "0" | "false" => Ok(serde_json::Value::Number(0.into())),
            _ => Err(DbError::InvalidDesign(format!(
                "row {row}, column {}: expected true/false or 1/0",
                column.name
            ))),
        }
    } else if type_name.contains("BLOB") {
        Err(DbError::InvalidDesign(format!(
            "row {row}, column {}: BLOB import is not supported",
            column.name
        )))
    } else {
        Ok(serde_json::Value::String(raw.to_string()))
    }
}

fn resolve_import_mapping(
    source_columns: &[String],
    target_columns: &[ColumnInfo],
    requested: &[ImportColumnMapping],
) -> Result<Vec<(String, String)>> {
    let mut requested_by_source: HashMap<&str, Option<&str>> = HashMap::new();
    for mapping in requested {
        if !source_columns
            .iter()
            .any(|source| source == &mapping.source)
        {
            return Err(DbError::InvalidDesign(format!(
                "source column not found: {}",
                mapping.source
            )));
        }
        requested_by_source.insert(mapping.source.as_str(), mapping.target.as_deref());
    }

    let mut pairs = Vec::new();
    let mut used_targets = HashSet::new();
    for source in source_columns {
        let target = match requested_by_source.get(source.as_str()) {
            Some(None) => None,
            Some(Some(target)) if target.trim().is_empty() => None,
            Some(Some(target)) => {
                let actual = target_columns
                    .iter()
                    .find(|column| column.name.eq_ignore_ascii_case(target))
                    .ok_or_else(|| DbError::UnknownColumn(target.to_string()))?;
                Some(actual.name.clone())
            }
            None => target_columns
                .iter()
                .find(|column| column.name.eq_ignore_ascii_case(source))
                .map(|column| column.name.clone()),
        };

        if let Some(target) = target {
            if !used_targets.insert(target.to_ascii_lowercase()) {
                return Err(DbError::InvalidDesign(format!(
                    "duplicate target column: {target}"
                )));
            }
            pairs.push((source.clone(), target));
        }
    }

    Ok(pairs)
}

fn suggest_table_name(path: &Path) -> String {
    sanitize_import_identifier(
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("imported"),
        "imported_table",
    )
}

fn sanitize_import_identifier(raw: &str, fallback: &str) -> String {
    let mut cleaned = String::new();
    for character in raw.trim().chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            cleaned.push(character);
        } else {
            cleaned.push('_');
        }
    }
    while cleaned.contains("__") {
        cleaned = cleaned.replace("__", "_");
    }
    let mut cleaned = cleaned.trim_matches('_').to_string();
    if cleaned.is_empty() {
        cleaned = fallback.to_string();
    }
    if cleaned
        .chars()
        .next()
        .map(|character| character.is_ascii_digit())
        .unwrap_or(true)
    {
        cleaned = format!("c_{cleaned}");
    }
    if cleaned.to_ascii_lowercase().starts_with("_dbgo_") {
        cleaned = format!("c{cleaned}");
    }
    if cleaned.len() > 64 {
        cleaned.truncate(64);
    }
    cleaned
}

fn unique_import_identifier(raw: &str, used: &mut HashSet<String>, index: usize) -> String {
    let base = sanitize_import_identifier(raw, &format!("column_{index}"));
    let base = if base.len() > 58 {
        base[..58].to_string()
    } else {
        base
    };
    let mut candidate = base.clone();
    let mut suffix = 2;
    while !used.insert(candidate.to_ascii_lowercase()) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }
    candidate
}

struct D1SqlObject {
    kind: String,
    name: String,
    table: String,
    sql: String,
}

const D1_DATA_ROWS_PER_FILE: usize = 500;
const D1_DATA_BYTES_PER_FILE: usize = 500_000;

struct D1DataWriter {
    output_dir: PathBuf,
    generated_at: String,
    file: Option<BufWriter<File>>,
    sequence: usize,
    rows_in_file: usize,
    bytes_in_file: usize,
    files: Vec<String>,
}

impl D1DataWriter {
    fn new(output_dir: PathBuf, generated_at: String) -> Self {
        Self {
            output_dir,
            generated_at,
            file: None,
            sequence: 0,
            rows_in_file: 0,
            bytes_in_file: 0,
            files: Vec::new(),
        }
    }

    fn write(&mut self, statement: &str) -> Result<()> {
        if self.file.is_none()
            || self.rows_in_file >= D1_DATA_ROWS_PER_FILE
            || (self.bytes_in_file + statement.len() + 1 > D1_DATA_BYTES_PER_FILE
                && self.rows_in_file > 0)
        {
            self.rotate()?;
        }
        let file = self.file.as_mut().expect("data file is open");
        writeln!(file, "{statement}")?;
        self.rows_in_file += 1;
        self.bytes_in_file += statement.len() + 1;
        Ok(())
    }

    fn rotate(&mut self) -> Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        self.sequence += 1;
        let name = format!("data-{:04}.sql", self.sequence);
        let path = self.output_dir.join(&name);
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "-- DBGo D1 data export — {}", self.generated_at)?;
        writeln!(writer, "-- File {} of the export", self.sequence)?;
        writeln!(writer)?;
        self.file = Some(writer);
        self.rows_in_file = 0;
        self.bytes_in_file = 0;
        self.files.push(name);
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        Ok(())
    }
}

fn order_d1_tables(
    mut tables: Vec<String>,
    database: &Database,
    skipped: &HashSet<String>,
    warnings: &mut Vec<String>,
) -> Result<Vec<String>> {
    tables.sort();
    let by_lower = tables
        .iter()
        .map(|table| (table.to_ascii_lowercase(), table.clone()))
        .collect::<HashMap<_, _>>();
    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for table in &tables {
        let mut set = HashSet::new();
        for foreign_key in database.foreign_keys(table)? {
            let referenced = foreign_key.referenced_table.to_ascii_lowercase();
            if referenced != table.to_ascii_lowercase()
                && !skipped.contains(&referenced)
                && by_lower.contains_key(&referenced)
            {
                set.insert(referenced);
            }
        }
        deps.insert(table.to_ascii_lowercase(), set);
    }

    let mut remaining = by_lower.keys().cloned().collect::<HashSet<_>>();
    let mut ordered = Vec::with_capacity(tables.len());
    while !remaining.is_empty() {
        let mut ready = remaining
            .iter()
            .filter(|table| {
                deps.get(*table)
                    .map(|deps| deps.iter().all(|dep| !remaining.contains(dep)))
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();
        if ready.is_empty() {
            warnings.push(
                "foreign key cycle detected; remaining tables use alphabetical order".to_string(),
            );
            let mut rest = remaining.iter().cloned().collect::<Vec<_>>();
            rest.sort();
            for table in rest {
                remaining.remove(&table);
                if let Some(name) = by_lower.get(&table) {
                    ordered.push(name.clone());
                }
            }
            break;
        }
        ready.sort();
        for table in ready {
            remaining.remove(&table);
            if let Some(name) = by_lower.get(&table) {
                ordered.push(name.clone());
            }
        }
    }
    Ok(ordered)
}

fn d1_database_name(database: &Database, provided: Option<&str>) -> String {
    let raw = provided
        .map(ToOwned::to_owned)
        .or_else(|| {
            database.project_dir().and_then(|project_dir| {
                read_project_config(project_dir)
                    .ok()
                    .map(|config| config.name)
            })
        })
        .or_else(|| {
            database
                .path()
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "dbgo-db".to_string());
    sanitize_d1_name(&raw)
}

fn sanitize_d1_name(value: &str) -> String {
    let mut name = value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while name.contains("--") {
        name = name.replace("--", "-");
    }
    let name = name.trim_matches('-').to_string();
    if name.is_empty() {
        "dbgo-db".to_string()
    } else {
        name.chars().take(63).collect()
    }
}

fn d1_wrangler_toml(name: &str) -> String {
    format!(
        "# Generated by DBGo\nname = \"{name}-worker\"\ncompatibility_date = \"2024-11-01\"\n\n[[d1_databases]]\nbinding = \"DB\"\ndatabase_name = \"{name}\"\ndatabase_id = \"<YOUR_D1_DATABASE_ID>\"\n"
    )
}

fn d1_readme(
    name: &str,
    source: &Path,
    generated_at: &str,
    data_files: &[String],
    tables: &[D1TableExport],
    warnings: &[String],
) -> String {
    let mut output = format!(
        "# D1 export\n\nGenerated by DBGo from `{}` at {}.\n\n",
        source.display(),
        generated_at
    );
    output.push_str("## Files\n\n- `schema.sql`\n");
    for file in data_files {
        output.push_str(&format!("- `{file}`\n"));
    }
    output.push_str("\n## Deploy\n\n```bash\n");
    output.push_str(&format!("wrangler d1 create {name}\n"));
    output.push_str("# Put the returned database_id into wrangler.toml.\n");
    output.push_str(&format!(
        "wrangler d1 execute {name} --remote --file schema.sql\n"
    ));
    for file in data_files {
        output.push_str(&format!(
            "wrangler d1 execute {name} --remote --file {file}\n"
        ));
    }
    output.push_str("```\n\n## Tables\n\n| Table | Rows |\n|---|---:|\n");
    for table in tables {
        output.push_str(&format!("| `{}` | {} |\n", table.name, table.rows));
    }
    if data_files.is_empty() {
        output.push_str("\nNo data rows were exported.\n");
    }
    if !warnings.is_empty() {
        output.push_str("\n## Warnings\n\n");
        for warning in warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output.push_str("\nDBGo internal tables (`_dbgo_*`) are excluded.\n");
    output
}

fn d1_sql_literal(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(integer) => integer.to_string(),
        ValueRef::Real(real) if real.is_finite() => real.to_string(),
        ValueRef::Real(_) => "NULL".to_string(),
        ValueRef::Text(text) => {
            if text.contains(&0) {
                format!("CAST(X'{}' AS TEXT)", hex_encode(text))
            } else {
                format!("'{}'", String::from_utf8_lossy(text).replace('\'', "''"))
            }
        }
        ValueRef::Blob(blob) => format!("X'{}'", hex_encode(blob)),
    }
}

fn exec_returning(
    connection: &Connection,
    sql: &str,
    limit: usize,
) -> Result<(Vec<ColumnInfo>, Vec<Row>, bool, usize)> {
    let mut statement = connection.prepare(sql)?;
    let columns = statement
        .columns()
        .map(|column| ColumnInfo {
            name: column.name().to_string(),
            type_name: column.decl_type().unwrap_or("").to_string(),
            not_null: false,
            primary_key: false,
        })
        .collect::<Vec<_>>();

    let before = connection.total_changes();
    if columns.is_empty() {
        connection.execute_batch(sql)?;
        let affected = connection.total_changes().saturating_sub(before) as usize;
        return Ok((Vec::new(), Vec::new(), false, affected));
    }

    let mut rows = statement.query([])?;
    let mut output = Vec::new();
    let mut truncated = false;
    while let Some(row) = rows.next()? {
        if output.len() < limit {
            let mut cells = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                cells.push(cell_from_value_ref(row.get_ref(index)?));
            }
            output.push(Row { rowid: None, cells });
        } else {
            truncated = true;
        }
    }
    let affected = connection.total_changes().saturating_sub(before) as usize;
    Ok((columns, output, truncated, affected))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn row_to_json(columns: &[ColumnInfo], row: &rusqlite::Row<'_>) -> Result<serde_json::Value> {
    let mut map = serde_json::Map::new();
    for (index, column) in columns.iter().enumerate() {
        map.insert(column.name.clone(), cell_to_json(row.get_ref(index + 1)?));
    }
    Ok(serde_json::Value::Object(map))
}

fn fetch_row_json(
    conn: &Connection,
    table: &str,
    columns: &[ColumnInfo],
    rowid: i64,
) -> Result<Option<serde_json::Value>> {
    let sql = format!(
        "SELECT rowid AS __dbgo_rowid, * FROM {} WHERE rowid = ?1",
        quote_ident(table)
    );
    let mut statement = conn.prepare(&sql)?;
    let mut rows = statement.query([rowid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_json(columns, row)?))
    } else {
        Ok(None)
    }
}

fn select_rows_json(
    conn: &Connection,
    quoted_table: &str,
    where_sql: &str,
    params: Vec<rusqlite::types::Value>,
    columns: &[ColumnInfo],
) -> Result<Vec<(i64, serde_json::Value)>> {
    let sql = format!("SELECT rowid AS __dbgo_rowid, * FROM {quoted_table}{where_sql}");
    let mut statement = conn.prepare(&sql)?;
    let mut rows = statement.query(rusqlite::params_from_iter(params))?;
    let mut matching = Vec::new();
    while let Some(row) = rows.next()? {
        let rowid = row.get::<_, i64>(0)?;
        matching.push((rowid, row_to_json(columns, row)?));
    }
    Ok(matching)
}

fn record_audit(
    conn: &Connection,
    session_id: &str,
    table: &str,
    rowid: Option<i64>,
    action: &str,
    before: Option<&serde_json::Value>,
    after: Option<&serde_json::Value>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO _dbgo_audit
         (session_id, table_name, rowid, action, before_json, after_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            session_id,
            table,
            rowid,
            action,
            before.map(|value| value.to_string()),
            after.map(|value| value.to_string())
        ],
    )?;
    Ok(())
}

fn new_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        now_millis(),
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn remove_sidecar(path: &Path, suffix: &str) {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(suffix);
    let _ = std::fs::remove_file(PathBuf::from(sidecar));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE companies (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                domain TEXT,
                score REAL,
                logo BLOB
             );
             INSERT INTO companies (name, domain, score, logo)
             VALUES ('Acme', 'acme.test', 9.5, x'0102'),
                    ('Globex', 'globex.test', 8.0, NULL),
                    ('Initech', NULL, NULL, NULL);",
        )
        .unwrap();
        Database {
            path: PathBuf::from(":memory:"),
            conn,
            project_dir: None,
        }
    }

    #[test]
    fn lists_tables() {
        let db = memory_db();
        let tables = db.tables().unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "companies");
    }

    #[test]
    fn pages_with_keyset() {
        let db = memory_db();
        let first = db.page("companies", None, Direction::Next, 2).unwrap();
        assert_eq!(first.rows.len(), 2);
        assert!(first.has_more);
        assert_eq!(first.rows[0].rowid, Some(1));

        let second = db
            .page("companies", first.next_cursor, Direction::Next, 2)
            .unwrap();
        assert_eq!(second.rows.len(), 1);
        assert!(!second.has_more);
        assert_eq!(second.rows[0].rowid, Some(3));

        let back = db
            .page("companies", second.prev_cursor, Direction::Prev, 2)
            .unwrap();
        assert_eq!(back.rows.len(), 2);
        assert_eq!(back.rows[0].rowid, Some(1));
    }

    #[test]
    fn counts_rows() {
        let db = memory_db();
        let count = db.count_rows("companies", 1_000_000).unwrap();
        assert_eq!(count.count, 3);
        assert!(!count.capped);
    }

    #[test]
    fn runs_read_only_query() {
        let db = memory_db();
        let result = db
            .query("SELECT id, name, score FROM companies ORDER BY id", 10)
            .unwrap();
        assert_eq!(result.columns.len(), 3);
        assert_eq!(result.columns[0].name, "id");
        assert_eq!(result.rows.len(), 3);
        assert!(!result.truncated);
        assert!(matches!(result.rows[0].cells[1], CellValue::Text { .. }));
    }

    #[test]
    fn truncates_query_results() {
        let db = memory_db();
        let result = db.query("SELECT * FROM companies", 2).unwrap();
        assert_eq!(result.rows.len(), 2);
        assert!(result.truncated);
    }

    #[test]
    fn rejects_write_statements() {
        let db = memory_db();
        let error = db.query("DELETE FROM companies", 10).unwrap_err();
        assert!(matches!(error, DbError::ReadOnly));
    }

    #[test]
    fn builds_schema_map() {
        let db = memory_db();
        let map = db.schema_map().unwrap();
        let columns: Vec<&str> = map["companies"].iter().map(String::as_str).collect();
        assert_eq!(columns, vec!["id", "name", "domain", "score", "logo"]);
    }

    #[test]
    fn builds_schema_graph() {
        let db = memory_db();
        db.conn
            .execute_batch(
                "CREATE TABLE links (
                    id INTEGER PRIMARY KEY,
                    company_id INTEGER NOT NULL REFERENCES companies(id) ON DELETE CASCADE
                );",
            )
            .unwrap();

        let graph = db.schema_graph().unwrap();
        assert_eq!(graph.tables.len(), 2);

        let links = graph
            .tables
            .iter()
            .find(|table| table.name == "links")
            .unwrap();
        let company_id = links
            .columns
            .iter()
            .find(|column| column.name == "company_id")
            .unwrap();
        assert!(company_id.foreign_key);
        assert_eq!(links.primary_key, vec!["id".to_string()]);

        assert_eq!(graph.relationships.len(), 1);
        let relationship = &graph.relationships[0];
        assert_eq!(relationship.from_table, "links");
        assert_eq!(relationship.from_column, "company_id");
        assert_eq!(relationship.to_table, "companies");
        assert_eq!(relationship.to_column, "id");
        assert_eq!(relationship.on_delete, "CASCADE");
    }

    #[test]
    fn sorts_table_page() {
        let db = memory_db();
        let asc = SortSpec {
            column: "name".into(),
            direction: SortDirection::Asc,
        };
        let page = db.table_page("companies", None, Some(&asc), 0, 10).unwrap();
        let names: Vec<&str> = page
            .rows
            .iter()
            .map(|row| match &row.cells[1] {
                CellValue::Text { value } => value.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(names, vec!["Acme", "Globex", "Initech"]);

        let desc = SortSpec {
            column: "name".into(),
            direction: SortDirection::Desc,
        };
        let page = db
            .table_page("companies", None, Some(&desc), 0, 10)
            .unwrap();
        let names: Vec<&str> = page
            .rows
            .iter()
            .map(|row| match &row.cells[1] {
                CellValue::Text { value } => value.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(names, vec!["Initech", "Globex", "Acme"]);
    }

    #[test]
    fn filters_table_page() {
        let db = memory_db();
        let contains = FilterSpec {
            column: "name".into(),
            op: FilterOp::Contains,
            value: Some("glob".into()),
        };
        let page = db
            .table_page("companies", Some(&contains), None, 0, 10)
            .unwrap();
        assert_eq!(page.rows.len(), 1);

        let is_null = FilterSpec {
            column: "domain".into(),
            op: FilterOp::IsNull,
            value: None,
        };
        let page = db
            .table_page("companies", Some(&is_null), None, 0, 10)
            .unwrap();
        assert_eq!(page.rows.len(), 1);
    }

    #[test]
    fn rejects_unknown_sort_column() {
        let db = memory_db();
        let sort = SortSpec {
            column: "missing".into(),
            direction: SortDirection::Asc,
        };
        assert!(matches!(
            db.table_page("companies", None, Some(&sort), 0, 10),
            Err(DbError::UnknownColumn(_))
        ));
    }

    fn column(name: &str, type_name: &str) -> ColumnDesign {
        ColumnDesign {
            name: name.into(),
            type_name: type_name.into(),
            primary_key: false,
            not_null: false,
            unique: false,
            default_value: None,
            references: None,
        }
    }

    #[test]
    fn creates_database_with_migrations_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.db");
        let (db, backup) = Database::create(&path, false).unwrap();
        assert!(backup.is_none());
        assert!(db.tables().unwrap().is_empty());

        let migrations: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM _dbgo_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(migrations, 0);

        assert!(matches!(
            Database::create(&path, false),
            Err(DbError::DatabaseExists(_))
        ));
    }

    #[test]
    fn creates_table_with_foreign_key_and_index() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.db");
        let (db, _) = Database::create(&path, false).unwrap();

        let companies = TableDesign {
            name: "companies".into(),
            columns: vec![
                ColumnDesign {
                    primary_key: true,
                    not_null: true,
                    ..column("id", "INTEGER")
                },
                ColumnDesign {
                    not_null: true,
                    ..column("name", "TEXT")
                },
            ],
            index_foreign_keys: true,
        };
        db.create_table(&companies).unwrap();

        let links = TableDesign {
            name: "links".into(),
            columns: vec![
                ColumnDesign {
                    primary_key: true,
                    not_null: true,
                    ..column("id", "INTEGER")
                },
                ColumnDesign {
                    not_null: true,
                    references: Some(ForeignKeyDesign {
                        table: "companies".into(),
                        column: "id".into(),
                        on_delete: Some("CASCADE".into()),
                    }),
                    ..column("company_id", "INTEGER")
                },
            ],
            index_foreign_keys: true,
        };
        db.create_table(&links).unwrap();

        assert_eq!(db.tables().unwrap().len(), 2);

        let (target_table, from_column, to_column): (String, String, String) = db
            .conn
            .query_row("PRAGMA foreign_key_list('links')", [], |row| {
                Ok((row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .unwrap();
        assert_eq!(target_table, "companies");
        assert_eq!(from_column, "company_id");
        assert_eq!(to_column, "id");

        let indexes: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_links_company_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(indexes, 1);

        let migrations: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM _dbgo_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(migrations, 2);
    }

    #[test]
    fn rejects_duplicate_column_names() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.db");
        let (db, _) = Database::create(&path, false).unwrap();
        let design = TableDesign {
            name: "things".into(),
            columns: vec![column("name", "TEXT"), column("NAME", "TEXT")],
            index_foreign_keys: true,
        };
        assert!(matches!(
            db.create_table(&design),
            Err(DbError::InvalidDesign(_))
        ));
    }

    #[test]
    fn backs_up_existing_database_before_replacing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.db");
        let (db, _) = Database::create(&path, false).unwrap();
        db.create_table(&TableDesign {
            name: "keep".into(),
            columns: vec![ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            }],
            index_foreign_keys: true,
        })
        .unwrap();
        drop(db);

        let (fresh, backup) = Database::create(&path, true).unwrap();
        assert!(backup.is_some());
        assert!(backup.as_ref().unwrap().exists());
        assert!(fresh.tables().unwrap().is_empty());
    }

    #[test]
    fn creates_project_with_agent_files() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("database");
        let (db, backup) = Database::create_project(&project, false).unwrap();
        assert!(backup.is_none());
        assert!(project.join("dbgo.json").exists());
        assert!(project.join("AGENTS.md").exists());
        assert!(project.join("schema.md").exists());
        assert!(project.join("backups").is_dir());
        assert!(project.join("migrations").is_dir());
        assert_eq!(db.project_dir().unwrap(), project.as_path());
        assert!(db.tables().unwrap().is_empty());

        let reopened = Database::open_project(&project).unwrap();
        assert_eq!(reopened.project_dir().unwrap(), project.as_path());
        assert!(reopened.tables().unwrap().is_empty());
    }

    #[test]
    fn create_table_updates_project_files() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("database");
        let (db, _) = Database::create_project(&project, false).unwrap();
        db.create_table(&TableDesign {
            name: "companies".into(),
            columns: vec![ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            }],
            index_foreign_keys: true,
        })
        .unwrap();

        let schema = std::fs::read_to_string(project.join("schema.md")).unwrap();
        assert!(schema.contains("## companies"));
        assert!(schema.contains("| id | INTEGER | PK, NOT NULL |"));

        let migrations: Vec<_> = std::fs::read_dir(project.join("migrations"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .collect();
        assert_eq!(migrations.len(), 1);
        let migration = std::fs::read_to_string(migrations[0].path()).unwrap();
        assert!(migration.contains("CREATE TABLE \"companies\""));
    }

    #[test]
    fn backup_writes_into_project_backups() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("database");
        let (db, _) = Database::create_project(&project, false).unwrap();
        let backup = db.backup().unwrap();
        assert!(backup.starts_with(project.join("backups")));
        assert!(backup.exists());
    }

    fn temp_companies_db() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let (db, _) = Database::create(&path, false).unwrap();
        db.create_table(&TableDesign {
            name: "companies".into(),
            columns: vec![
                ColumnDesign {
                    primary_key: true,
                    not_null: true,
                    ..column("id", "INTEGER")
                },
                ColumnDesign {
                    not_null: true,
                    unique: true,
                    ..column("domain", "TEXT")
                },
                ColumnDesign {
                    ..column("city", "TEXT")
                },
            ],
            index_foreign_keys: true,
        })
        .unwrap();
        (dir, db)
    }

    fn json_row(values: &[(&str, serde_json::Value)]) -> JsonObject {
        values
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect()
    }

    fn city_of(db: &Database, domain: &str) -> String {
        let result = db
            .query(
                &format!("SELECT city FROM companies WHERE domain = '{domain}'"),
                10,
            )
            .unwrap();
        match &result.rows[0].cells[0] {
            CellValue::Text { value } => value.clone(),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn inserts_rows_and_audits() {
        let (_dir, db) = temp_companies_db();
        let result = db
            .insert_rows(
                "companies",
                &[json_row(&[
                    ("id", serde_json::json!(1)),
                    ("domain", serde_json::json!("acme.test")),
                    ("city", serde_json::json!("Berlin")),
                ])],
            )
            .unwrap();

        assert_eq!(result.affected, 1);
        assert!(result.backup_path.is_none());
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
        assert_eq!(city_of(&db, "acme.test"), "Berlin");

        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        let inserted = sessions
            .iter()
            .find(|session| session.session_id == result.session_id)
            .expect("insert session is listed");
        assert_eq!(inserted.actions, 1);
    }

    #[test]
    fn upserts_rows_on_unique_key() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();

        let result = db
            .upsert_rows(
                "companies",
                "domain",
                &[json_row(&[
                    ("domain", serde_json::json!("acme.test")),
                    ("city", serde_json::json!("Munich")),
                ])],
            )
            .unwrap();

        assert_eq!(result.affected, 1);
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
        assert_eq!(city_of(&db, "acme.test"), "Munich");
    }

    #[test]
    fn updates_rows_by_filter() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();

        let filter = FilterSpec {
            column: "domain".into(),
            op: FilterOp::Eq,
            value: Some("acme.test".into()),
        };
        let result = db
            .update_rows(
                "companies",
                &filter,
                &json_row(&[("city", serde_json::json!("Paris"))]),
            )
            .unwrap();

        assert_eq!(result.affected, 1);
        assert_eq!(city_of(&db, "acme.test"), "Paris");
    }

    #[test]
    fn deletes_rows_by_filter() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();

        let filter = FilterSpec {
            column: "domain".into(),
            op: FilterOp::Eq,
            value: Some("acme.test".into()),
        };
        let result = db.delete_rows("companies", &filter).unwrap();
        assert_eq!(result.affected, 1);
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 0);
    }

    #[test]
    fn rejects_primary_key_updates() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
            ])],
        )
        .unwrap();

        let filter = FilterSpec {
            column: "domain".into(),
            op: FilterOp::Eq,
            value: Some("acme.test".into()),
        };
        let result = db.update_rows(
            "companies",
            &filter,
            &json_row(&[("id", serde_json::json!(2))]),
        );
        assert!(matches!(result, Err(DbError::InvalidDesign(_))));
    }

    #[test]
    fn saves_batch_of_updates_inserts_and_deletes_in_one_session() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[
                json_row(&[
                    ("id", serde_json::json!(1)),
                    ("domain", serde_json::json!("acme.test")),
                    ("city", serde_json::json!("Berlin")),
                ]),
                json_row(&[
                    ("id", serde_json::json!(2)),
                    ("domain", serde_json::json!("globex.test")),
                    ("city", serde_json::json!("Austin")),
                ]),
            ],
        )
        .unwrap();

        let rowids = |db: &Database| -> Vec<i64> {
            db.query("SELECT rowid FROM companies ORDER BY id", 10)
                .unwrap()
                .rows
                .iter()
                .map(|row| match row.cells[0] {
                    CellValue::Integer { value } => value,
                    ref other => panic!("expected integer rowid, got {other:?}"),
                })
                .collect()
        };
        let ids = rowids(&db);

        let result = db
            .save_changes(
                "companies",
                &[RowUpdate {
                    rowid: ids[0],
                    values: json_row(&[("city", serde_json::json!("Munich"))]),
                }],
                &[json_row(&[
                    ("id", serde_json::json!(3)),
                    ("domain", serde_json::json!("initech.test")),
                    ("city", serde_json::json!("Tokyo")),
                ])],
                &[ids[1]],
            )
            .unwrap();

        assert_eq!(result.affected, 3);
        assert!(result.backup_path.is_none());
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 2);
        assert_eq!(city_of(&db, "acme.test"), "Munich");
        assert_eq!(city_of(&db, "initech.test"), "Tokyo");

        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 3);
        let saved = sessions
            .iter()
            .find(|session| session.session_id == result.session_id)
            .expect("saved session is listed");
        assert_eq!(saved.actions, 3);
    }

    #[test]
    fn save_changes_can_replace_a_deleted_key_in_one_batch() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();
        let rowid = match db.query("SELECT rowid FROM companies", 10).unwrap().rows[0].cells[0] {
            CellValue::Integer { value } => value,
            ref other => panic!("expected integer rowid, got {other:?}"),
        };

        db.save_changes(
            "companies",
            &[],
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme-new.test")),
                ("city", serde_json::json!("Paris")),
            ])],
            &[rowid],
        )
        .unwrap();

        assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
        assert_eq!(city_of(&db, "acme-new.test"), "Paris");
    }

    #[test]
    fn save_changes_rejects_primary_key_updates() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
            ])],
        )
        .unwrap();
        let rowid = match db.query("SELECT rowid FROM companies", 10).unwrap().rows[0].cells[0] {
            CellValue::Integer { value } => value,
            ref other => panic!("expected integer rowid, got {other:?}"),
        };

        let result = db.save_changes(
            "companies",
            &[RowUpdate {
                rowid,
                values: json_row(&[("id", serde_json::json!(2))]),
            }],
            &[],
            &[],
        );
        assert!(matches!(result, Err(DbError::InvalidDesign(_))));
    }

    #[test]
    fn save_changes_rolls_back_the_whole_batch() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();
        let ids = db
            .query("SELECT rowid FROM companies", 10)
            .unwrap()
            .rows
            .iter()
            .map(|row| match row.cells[0] {
                CellValue::Integer { value } => value,
                ref other => panic!("expected integer rowid, got {other:?}"),
            })
            .collect::<Vec<_>>();

        let result = db.save_changes(
            "companies",
            &[RowUpdate {
                rowid: ids[0],
                values: json_row(&[("city", serde_json::json!("Paris"))]),
            }],
            &[],
            &[999_999],
        );

        assert!(result.is_err());
        assert_eq!(city_of(&db, "acme.test"), "Berlin");
    }

    #[test]
    fn alters_table_add_rename_and_drop_columns() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();

        let operations = vec![
            AlterTableOp::AddColumn {
                column: ColumnDesign {
                    name: "country".into(),
                    type_name: "TEXT".into(),
                    primary_key: false,
                    not_null: true,
                    unique: false,
                    default_value: Some("US".into()),
                    references: None,
                },
            },
            AlterTableOp::RenameColumn {
                from: "city".into(),
                to: "location".into(),
            },
            AlterTableOp::DropColumn {
                name: "country".into(),
            },
        ];

        let preview = db.preview_alter_table("companies", &operations).unwrap();
        assert_eq!(preview.len(), 3);
        assert!(db
            .columns("companies")
            .unwrap()
            .iter()
            .any(|c| c.name == "city"));

        let change = db.alter_table("companies", &operations).unwrap();
        assert_eq!(change.statements.len(), 3);
        assert!(change.backup_path.is_none());

        let columns = db.columns("companies").unwrap();
        let names = columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>();
        assert_eq!(names, vec!["id", "domain", "location"]);
        let result = db
            .query(
                "SELECT location FROM companies WHERE domain = 'acme.test'",
                10,
            )
            .unwrap();
        assert!(matches!(
            result.rows[0].cells[0],
            CellValue::Text { ref value } if value == "Berlin"
        ));
    }

    #[test]
    fn rejects_unsafe_alter_operations() {
        let (_dir, db) = temp_companies_db();
        let not_null = db.preview_alter_table(
            "companies",
            &[AlterTableOp::AddColumn {
                column: ColumnDesign {
                    name: "country".into(),
                    type_name: "TEXT".into(),
                    primary_key: false,
                    not_null: true,
                    unique: false,
                    default_value: None,
                    references: None,
                },
            }],
        );
        assert!(matches!(not_null, Err(DbError::InvalidDesign(_))));

        let drop_indexed = db.preview_alter_table(
            "companies",
            &[AlterTableOp::DropColumn {
                name: "domain".into(),
            }],
        );
        assert!(matches!(drop_indexed, Err(DbError::InvalidDesign(_))));
    }

    #[test]
    fn creates_and_drops_indexes() {
        let (_dir, db) = temp_companies_db();
        let change = db
            .create_index(
                "companies",
                "idx_companies_city",
                &["city".to_string()],
                false,
            )
            .unwrap();
        assert!(change.migration_name.contains("idx_companies_city"));
        let indexes = db.indexes(Some("companies")).unwrap();
        let created = indexes
            .iter()
            .find(|index| index.name == "idx_companies_city")
            .expect("index is listed");
        assert_eq!(created.columns, vec!["city"]);

        let duplicate = db.create_index(
            "companies",
            "idx_companies_city",
            &["city".to_string()],
            false,
        );
        assert!(duplicate.is_err());

        db.drop_index("idx_companies_city").unwrap();
        assert!(!db
            .indexes(Some("companies"))
            .unwrap()
            .iter()
            .any(|index| index.name == "idx_companies_city"));
    }

    #[test]
    fn creates_and_drops_views() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();

        db.create_view(
            "berlin_companies",
            "SELECT * FROM companies WHERE city = 'Berlin'",
        )
        .unwrap();
        assert_eq!(db.views().unwrap().len(), 1);
        let result = db.query("SELECT domain FROM berlin_companies", 10).unwrap();
        assert_eq!(result.rows.len(), 1);

        db.drop_view("berlin_companies").unwrap();
        assert!(db.views().unwrap().is_empty());
    }

    #[test]
    fn lists_and_applies_pending_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("database");
        let (db, _) = Database::create_project(&project, false).unwrap();
        db.create_table(&TableDesign {
            name: "notes".into(),
            columns: vec![
                ColumnDesign {
                    primary_key: true,
                    not_null: true,
                    ..column("id", "INTEGER")
                },
                ColumnDesign {
                    not_null: true,
                    ..column("body", "TEXT")
                },
            ],
            index_foreign_keys: true,
        })
        .unwrap();

        let migrations = db.migrations().unwrap();
        assert!(migrations.iter().any(|m| m.applied));

        std::fs::write(
            project.join("migrations").join("0002_manual.sql"),
            "CREATE TABLE tags (id INTEGER PRIMARY KEY, label TEXT NOT NULL);\n",
        )
        .unwrap();

        let migrations = db.migrations().unwrap();
        let pending = migrations
            .iter()
            .find(|migration| migration.name == "0002_manual")
            .expect("manual migration is listed");
        assert!(!pending.applied);

        let result = db.apply_migrations().unwrap();
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.applied[0].name, "0002_manual");
        assert!(db.table_exists("tags").unwrap());

        let migrations = db.migrations().unwrap();
        assert!(
            migrations
                .iter()
                .find(|migration| migration.name == "0002_manual")
                .unwrap()
                .applied
        );
        assert!(db.apply_migrations().unwrap().applied.is_empty());
    }

    #[test]
    fn recognizes_old_create_table_migrations_by_sql() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("legacy");
        let (db, _) = Database::create_project(&project, false).unwrap();
        let sql = "CREATE TABLE \"legacy\" (\"id\" INTEGER PRIMARY KEY);";
        db.conn
            .execute(
                "INSERT INTO _dbgo_migrations (name, sql) VALUES (?1, ?2)",
                rusqlite::params!["create_table:legacy", sql],
            )
            .unwrap();
        std::fs::write(
            project.join("migrations").join("0001_create_legacy.sql"),
            format!("{sql}\n"),
        )
        .unwrap();

        let migrations = db.migrations().unwrap();
        let file = migrations
            .iter()
            .find(|migration| migration.name == "0001_create_legacy")
            .expect("legacy file is listed");
        assert!(file.applied);
    }

    #[test]
    fn lists_backups_and_restores() {
        let (_dir, mut db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();
        let backup = db.backup().unwrap();

        db.update_rows(
            "companies",
            &FilterSpec {
                column: "domain".into(),
                op: FilterOp::Eq,
                value: Some("acme.test".into()),
            },
            &json_row(&[("city", serde_json::json!("Paris"))]),
        )
        .unwrap();
        assert_eq!(city_of(&db, "acme.test"), "Paris");

        let listed = db.backups().unwrap();
        let listed_backup = listed
            .iter()
            .find(|info| info.path == backup.canonicalize().unwrap().display().to_string())
            .expect("explicit backup is listed");
        assert!(listed_backup.size_bytes > 0);

        let result = db.restore_backup(&backup.display().to_string()).unwrap();
        assert!(result.safety_backup.is_none());
        assert_eq!(result.tables, 1);
        assert_eq!(city_of(&db, "acme.test"), "Berlin");

        let after = db.backups().unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(
            after[0].path,
            backup.canonicalize().unwrap().display().to_string()
        );
        let sessions = db.sessions().unwrap();
        assert!(sessions
            .iter()
            .any(|session| session.session_id == result.session_id));
    }

    #[test]
    fn refuses_to_restore_a_file_outside_the_backup_directory() {
        let (_dir, mut db) = temp_companies_db();
        let result = db.restore_backup("/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn adopts_bare_database_with_agent_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("companies.db");
        let (mut db, _) = Database::create(&path, false).unwrap();
        db.create_table(&TableDesign {
            name: "companies".into(),
            columns: vec![
                ColumnDesign {
                    primary_key: true,
                    not_null: true,
                    ..column("id", "INTEGER")
                },
                ColumnDesign {
                    not_null: true,
                    unique: true,
                    ..column("domain", "TEXT")
                },
            ],
            index_foreign_keys: true,
        })
        .unwrap();

        let info = db.ensure_agent_files().unwrap();
        assert!(info.created);
        assert_eq!(info.database, "companies.db");
        assert_eq!(db.project_dir(), Some(dir.path()));

        for name in ["dbgo.json", "AGENTS.md", "schema.md"] {
            assert!(dir.path().join(name).is_file(), "{name} should exist");
        }
        assert!(dir.path().join("migrations").is_dir());
        assert!(dir.path().join("backups").is_dir());

        let agents = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(agents.contains("dbgo alter-table"));
        assert!(agents.contains("dbgo backups"));
        assert!(agents.contains("dbgo restore"));
        let schema = std::fs::read_to_string(dir.path().join("schema.md")).unwrap();
        assert!(schema.contains("## companies"));
        let config = read_project_config(dir.path()).unwrap();
        assert_eq!(config.database, "companies.db");

        let migration_files = std::fs::read_dir(dir.path().join("migrations"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .collect::<Vec<_>>();
        assert!(!migration_files.is_empty());
        assert!(migration_files.iter().any(|entry| {
            std::fs::read_to_string(entry.path())
                .map(|sql| sql.contains("CREATE TABLE"))
                .unwrap_or(false)
        }));

        let backup = db.backup().unwrap();
        assert!(backup.starts_with(dir.path().join("backups")));
    }

    #[test]
    fn adoption_preserves_existing_agent_and_schema_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("AGENTS.md"),
            "# My project\n\nCustom instructions.\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("schema.md"),
            "# My schema\n\nHand-written notes.\n",
        )
        .unwrap();
        let path = dir.path().join("companies.db");
        let (mut db, _) = Database::create(&path, false).unwrap();
        db.create_table(&TableDesign {
            name: "companies".into(),
            columns: vec![ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            }],
            index_foreign_keys: true,
        })
        .unwrap();

        db.ensure_agent_files().unwrap();
        db.ensure_agent_files().unwrap();

        let agents = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(agents.starts_with("# My project"));
        assert!(agents.contains("Custom instructions."));
        assert_eq!(agents.matches("<!-- dbgo:agent-guide:start -->").count(), 1);
        assert!(agents.contains("dbgo alter-table"));

        let schema = std::fs::read_to_string(dir.path().join("schema.md")).unwrap();
        assert!(schema.starts_with("# My schema"));
        assert!(schema.contains("Hand-written notes."));
        assert_eq!(schema.matches("<!-- dbgo:schema:start -->").count(), 1);
        assert!(schema.contains("## companies"));
    }

    #[test]
    fn refuses_adoption_when_project_file_points_elsewhere() {
        let dir = tempfile::tempdir().unwrap();
        write_project_config(
            dir.path(),
            &ProjectConfig {
                name: "other".into(),
                database: "other.db".into(),
                version: 1,
                created_at: now_secs(),
            },
        )
        .unwrap();
        let path = dir.path().join("companies.db");
        let (mut db, _) = Database::create(&path, false).unwrap();

        assert!(db.ensure_agent_files().is_err());
        assert!(!dir.path().join("AGENTS.md").exists());
    }

    #[test]
    fn reports_table_and_database_stats() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[
                json_row(&[
                    ("id", serde_json::json!(1)),
                    ("domain", serde_json::json!("acme.test")),
                    ("city", serde_json::json!("Berlin")),
                ]),
                json_row(&[
                    ("id", serde_json::json!(2)),
                    ("domain", serde_json::json!("globex.test")),
                    ("city", serde_json::json!("Austin")),
                ]),
            ],
        )
        .unwrap();

        let table = db.table_stats("companies").unwrap();
        assert_eq!(table.row_count, 2);
        assert!(!table.row_count_estimated);
        assert!(!table.indexes.is_empty());
        assert!(table.size_bytes.is_some());
        assert!(table.size_bytes.unwrap() > 0);

        let stats = db.database_stats().unwrap();
        assert!(stats.dbstat);
        assert!(stats.total_bytes > 0);
        assert!(stats
            .tables
            .iter()
            .any(|candidate| candidate.name == "companies"));
    }

    #[test]
    fn exports_table_and_query_results() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Ber, \"lin\"\nline")),
            ])],
        )
        .unwrap();

        let mut csv = Vec::new();
        let rows = db
            .export_table("companies", None, None, ExportFormat::Csv, &mut csv)
            .unwrap();
        assert_eq!(rows, 1);
        let csv = String::from_utf8(csv).unwrap();
        assert!(csv.starts_with("id,domain,city\n"));
        assert!(csv.contains("\"Ber, \"\"lin\"\"\nline\""));

        let mut json = Vec::new();
        let rows = db
            .export_query(
                "SELECT domain, city FROM companies",
                ExportFormat::Json,
                &mut json,
            )
            .unwrap();
        assert_eq!(rows, 1);
        let parsed: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 1);
        assert_eq!(parsed[0]["domain"], "acme.test");

        let mut rejected = Vec::new();
        assert!(matches!(
            db.export_query("DELETE FROM companies", ExportFormat::Csv, &mut rejected),
            Err(DbError::ReadOnly)
        ));
    }

    #[test]
    fn previews_and_imports_csv() {
        let (_dir, db) = temp_companies_db();
        let files = tempfile::tempdir().unwrap();
        let path = files.path().join("rows.csv");
        std::fs::write(
            &path,
            "id,domain,city\n1,acme.test,Berlin\n2,globex.test,Austin\n",
        )
        .unwrap();

        let preview = preview_import(&path, ImportFormat::Csv, true, None).unwrap();
        assert_eq!(preview.total_rows, 2);
        assert_eq!(preview.columns.len(), 3);
        assert_eq!(preview.columns[0].inferred_type, "INTEGER");
        assert_eq!(preview.columns[2].inferred_type, "TEXT");

        let result = db
            .import_file(
                &path,
                &ImportRequest {
                    table: Some("companies".into()),
                    create_table: false,
                    format: ImportFormat::Csv,
                    has_headers: true,
                    delimiter: None,
                    mapping: Vec::new(),
                    upsert_key: None,
                },
            )
            .unwrap();

        assert_eq!(result.rows, 2);
        assert!(!result.created_table);
        assert!(result.ignored_columns.is_empty());
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 2);
        assert_eq!(city_of(&db, "acme.test"), "Berlin");
    }

    #[test]
    fn imports_json_and_creates_a_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let (db, _) = Database::create(&path, false).unwrap();
        let json = dir.path().join("people.json");
        std::fs::write(
            &json,
            r#"[{"id":1,"name":"Ada","score":9.5},{"id":2,"name":"Grace","score":8}]"#,
        )
        .unwrap();

        let result = db
            .import_file(
                &json,
                &ImportRequest {
                    table: Some("people".into()),
                    create_table: true,
                    format: ImportFormat::Json,
                    has_headers: true,
                    delimiter: None,
                    mapping: Vec::new(),
                    upsert_key: None,
                },
            )
            .unwrap();

        assert!(result.created_table);
        assert_eq!(result.table, "people");
        assert_eq!(result.rows, 2);
        assert!(db.table_exists("people").unwrap());
        let columns = db.columns("people").unwrap();
        assert!(columns
            .iter()
            .any(|column| column.name == "id" && column.type_name == "INTEGER"));
        assert!(columns
            .iter()
            .any(|column| column.name == "name" && column.type_name == "TEXT"));
        assert!(columns
            .iter()
            .any(|column| column.name == "score" && column.type_name == "REAL"));
    }

    #[test]
    fn import_sanitizes_csv_headers_and_supports_upsert() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let (db, _) = Database::create(&path, false).unwrap();
        let csv = dir.path().join("people.csv");
        std::fs::write(&csv, "id,First Name\n1,Ada\n").unwrap();

        let result = db
            .import_file(
                &csv,
                &ImportRequest {
                    table: None,
                    create_table: true,
                    format: ImportFormat::Csv,
                    has_headers: true,
                    delimiter: None,
                    mapping: Vec::new(),
                    upsert_key: None,
                },
            )
            .unwrap();
        assert_eq!(result.table, "people");
        let columns = db.columns("people").unwrap();
        assert!(columns.iter().any(|column| column.name == "First_Name"));
    }

    #[test]
    fn import_rejects_invalid_csv_values() {
        let (_dir, db) = temp_companies_db();
        let files = tempfile::tempdir().unwrap();
        let path = files.path().join("bad.csv");
        std::fs::write(&path, "id,domain\nnot-a-number,acme.test\n").unwrap();

        let result = db.import_file(
            &path,
            &ImportRequest {
                table: Some("companies".into()),
                create_table: false,
                format: ImportFormat::Csv,
                has_headers: true,
                delimiter: None,
                mapping: Vec::new(),
                upsert_key: None,
            },
        );
        assert!(result.is_err());
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 0);
    }

    #[test]
    fn import_upserts_using_a_key() {
        let (_dir, db) = temp_companies_db();
        let files = tempfile::tempdir().unwrap();
        let first = files.path().join("first.csv");
        std::fs::write(&first, "id,domain,city\n1,acme.test,Berlin\n").unwrap();
        db.import_file(
            &first,
            &ImportRequest {
                table: Some("companies".into()),
                create_table: false,
                format: ImportFormat::Csv,
                has_headers: true,
                delimiter: None,
                mapping: Vec::new(),
                upsert_key: None,
            },
        )
        .unwrap();

        let second = files.path().join("second.csv");
        std::fs::write(&second, "domain,city\nacme.test,Paris\n").unwrap();
        let result = db
            .import_file(
                &second,
                &ImportRequest {
                    table: Some("companies".into()),
                    create_table: false,
                    format: ImportFormat::Csv,
                    has_headers: true,
                    delimiter: None,
                    mapping: Vec::new(),
                    upsert_key: Some("domain".into()),
                },
            )
            .unwrap();

        assert_eq!(result.rows, 1);
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
        assert_eq!(city_of(&db, "acme.test"), "Paris");
    }

    #[test]
    fn import_mapping_overrides_without_disabling_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let (db, _) = Database::create(&db_path, false).unwrap();
        db.create_table(&TableDesign {
            name: "people".into(),
            columns: vec![
                ColumnDesign {
                    ..column("name", "TEXT")
                },
                ColumnDesign {
                    unique: true,
                    ..column("domain", "TEXT")
                },
                ColumnDesign {
                    ..column("city", "TEXT")
                },
            ],
            index_foreign_keys: true,
        })
        .unwrap();

        let files = tempfile::tempdir().unwrap();
        let path = files.path().join("mapped.csv");
        std::fs::write(&path, "domain,Full Name,city\nacme.test,Acme,Berlin\n").unwrap();

        let result = db
            .import_file(
                &path,
                &ImportRequest {
                    table: Some("people".into()),
                    create_table: false,
                    format: ImportFormat::Csv,
                    has_headers: true,
                    delimiter: None,
                    mapping: vec![
                        ImportColumnMapping {
                            source: "Full Name".into(),
                            target: Some("name".into()),
                        },
                        ImportColumnMapping {
                            source: "city".into(),
                            target: None,
                        },
                    ],
                    upsert_key: None,
                },
            )
            .unwrap();

        assert_eq!(result.rows, 1);
        assert_eq!(result.ignored_columns, vec!["city".to_string()]);
        let row = db
            .query(
                "SELECT name, city FROM people WHERE domain = 'acme.test'",
                1,
            )
            .unwrap();
        assert!(matches!(
            &row.rows[0].cells[0],
            CellValue::Text { value } if value == "Acme"
        ));
        assert!(matches!(row.rows[0].cells[1], CellValue::Null));
    }

    #[test]
    fn import_preserves_leading_zero_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let (db, _) = Database::create(&path, false).unwrap();
        let csv = dir.path().join("zips.csv");
        std::fs::write(&csv, "zip\n00123\n00456\n").unwrap();

        db.import_file(
            &csv,
            &ImportRequest {
                table: None,
                create_table: true,
                format: ImportFormat::Csv,
                has_headers: true,
                delimiter: None,
                mapping: Vec::new(),
                upsert_key: None,
            },
        )
        .unwrap();

        let columns = db.columns("zips").unwrap();
        assert!(columns
            .iter()
            .any(|column| column.name == "zip" && column.type_name == "TEXT"));
        let result = db.query("SELECT zip FROM zips ORDER BY zip", 10).unwrap();
        assert!(matches!(
            result.rows[0].cells[0],
            CellValue::Text { ref value } if value == "00123"
        ));
    }

    #[test]
    fn exports_a_d1_bundle_in_foreign_key_order() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("database");
        let (db, _) = Database::create_project(&project, false).unwrap();

        db.create_table(&TableDesign {
            name: "authors".into(),
            columns: vec![
                ColumnDesign {
                    primary_key: true,
                    not_null: true,
                    ..column("id", "INTEGER")
                },
                ColumnDesign {
                    not_null: true,
                    ..column("name", "TEXT")
                },
            ],
            index_foreign_keys: true,
        })
        .unwrap();
        db.create_table(&TableDesign {
            name: "books".into(),
            columns: vec![
                ColumnDesign {
                    primary_key: true,
                    not_null: true,
                    ..column("id", "INTEGER")
                },
                ColumnDesign {
                    not_null: true,
                    references: Some(ForeignKeyDesign {
                        table: "authors".into(),
                        column: "id".into(),
                        on_delete: Some("CASCADE".into()),
                    }),
                    ..column("author_id", "INTEGER")
                },
                ColumnDesign {
                    ..column("title", "TEXT")
                },
            ],
            index_foreign_keys: true,
        })
        .unwrap();

        db.insert_rows(
            "authors",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("name", serde_json::json!("O'Brien")),
            ])],
        )
        .unwrap();
        db.insert_rows(
            "books",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("author_id", serde_json::json!(1)),
                ("title", serde_json::json!("A book")),
            ])],
        )
        .unwrap();
        db.conn
            .execute(
                "INSERT INTO books (id, author_id, title) VALUES (2, 1, ?1)",
                rusqlite::params![&b"book\x00blob"[..]],
            )
            .unwrap();
        db.create_view("book_titles", "SELECT title FROM books")
            .unwrap();

        let output = dir.path().join("d1");
        let result = db.export_d1(&output, Some("my-db"), false).unwrap();
        assert_eq!(result.rows, 3);
        assert_eq!(result.tables.len(), 2);
        assert!(result.tables.iter().any(|table| table.name == "authors"));
        assert!(result.tables.iter().any(|table| table.name == "books"));
        assert!(!result.data_files.is_empty());

        let schema = std::fs::read_to_string(&result.schema_file).unwrap();
        assert!(schema.contains("CREATE TABLE \"authors\""));
        assert!(schema.contains("CREATE TABLE \"books\""));
        assert!(schema.contains("CREATE INDEX"));
        assert!(schema.contains("CREATE VIEW \"book_titles\""));
        assert!(!schema.contains("_dbgo_"));
        assert!(!schema.contains("sqlite_"));

        let mut data = String::new();
        for file in &result.data_files {
            data.push_str(&std::fs::read_to_string(output.join(file)).unwrap());
        }
        let authors_position = data.find("INSERT INTO \"authors\"").unwrap();
        let books_position = data.find("INSERT INTO \"books\"").unwrap();
        assert!(authors_position < books_position);
        assert!(data.contains("O''Brien"));
        assert!(data.contains("X'626f6f6b00626c6f62'"));

        let wrangler = std::fs::read_to_string(&result.wrangler_file).unwrap();
        assert!(wrangler.contains("database_name = \"my-db\""));
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&result.manifest_file).unwrap()).unwrap();
        assert_eq!(manifest["total_rows"], 3);
        assert_eq!(manifest["database_name"], "my-db");
    }

    #[test]
    fn d1_export_refuses_a_non_empty_directory_without_force() {
        let (_dir, db) = temp_companies_db();
        let output = tempfile::tempdir().unwrap();
        std::fs::write(output.path().join("keep.txt"), "keep").unwrap();

        assert!(db.export_d1(output.path(), None, false).is_err());
        db.export_d1(output.path(), None, true).unwrap();
        assert!(output.path().join("schema.sql").is_file());
        assert!(output.path().join("keep.txt").is_file());
    }

    #[test]
    fn exec_runs_reads_writes_returning_and_maintenance() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[
                json_row(&[
                    ("id", serde_json::json!(1)),
                    ("domain", serde_json::json!("acme.test")),
                    ("city", serde_json::json!("Berlin")),
                ]),
                json_row(&[
                    ("id", serde_json::json!(2)),
                    ("domain", serde_json::json!("globex.test")),
                    ("city", serde_json::json!("Austin")),
                ]),
            ],
        )
        .unwrap();

        let read = db
            .exec(
                "SELECT city FROM companies ORDER BY id",
                false,
                true,
                false,
                10,
            )
            .unwrap();
        assert!(read.read_only);
        assert_eq!(read.rows.len(), 2);
        assert!(read.session_id.is_none());

        assert!(matches!(
            db.exec("DELETE FROM companies", false, true, false, 10),
            Err(DbError::ReadOnly)
        ));

        let update = db
            .exec(
                "UPDATE companies SET city = 'Paris' WHERE id = 1",
                true,
                true,
                false,
                10,
            )
            .unwrap();
        assert_eq!(update.affected, 1);
        assert!(update.session_id.is_some());
        assert_eq!(city_of(&db, "acme.test"), "Paris");

        let returning = db
            .exec(
                "UPDATE companies SET city = 'Rome' WHERE id = 1 RETURNING id, city",
                true,
                true,
                true,
                10,
            )
            .unwrap();
        assert_eq!(returning.columns.len(), 2);
        assert_eq!(returning.rows.len(), 1);
        assert_eq!(returning.affected, 1);

        let multi = db
            .exec(
                "UPDATE companies SET city = city WHERE id = 1; UPDATE companies SET city = city WHERE id = 2;",
                true,
                true,
                false,
                10,
            )
            .unwrap();
        assert_eq!(multi.affected, 2);

        assert!(db.exec("VACUUM", true, true, false, 10).is_err());
        db.exec("VACUUM", true, false, false, 10).unwrap();
    }

    #[test]
    fn filters_support_and_or_not_expressions() {
        let (_dir, db) = temp_companies_db();
        db.insert_rows(
            "companies",
            &[
                json_row(&[
                    ("id", serde_json::json!(1)),
                    ("domain", serde_json::json!("acme.test")),
                    ("city", serde_json::json!("Berlin")),
                ]),
                json_row(&[
                    ("id", serde_json::json!(2)),
                    ("domain", serde_json::json!("globex.test")),
                    ("city", serde_json::json!("Austin")),
                ]),
                json_row(&[
                    ("id", serde_json::json!(3)),
                    ("domain", serde_json::json!("initech.test")),
                    ("city", serde_json::json!("Paris")),
                ]),
            ],
        )
        .unwrap();

        let filter = FilterExpr::And {
            filters: vec![
                FilterExpr::Condition {
                    column: "id".into(),
                    op: FilterOp::Gte,
                    value: Some("2".into()),
                },
                FilterExpr::Or {
                    filters: vec![
                        FilterExpr::Condition {
                            column: "city".into(),
                            op: FilterOp::Eq,
                            value: Some("Berlin".into()),
                        },
                        FilterExpr::Condition {
                            column: "city".into(),
                            op: FilterOp::Eq,
                            value: Some("Paris".into()),
                        },
                    ],
                },
            ],
        };
        let result = db
            .update_rows_expr(
                "companies",
                &filter,
                &json_row(&[("city", serde_json::json!("Matched"))]),
            )
            .unwrap();
        assert_eq!(result.affected, 1);
        assert_eq!(city_of(&db, "initech.test"), "Matched");
        assert_eq!(city_of(&db, "globex.test"), "Austin");

        let filter = FilterExpr::Not {
            filter: Box::new(FilterExpr::Condition {
                column: "id".into(),
                op: FilterOp::Eq,
                value: Some("3".into()),
            }),
        };
        let result = db.delete_rows_expr("companies", &filter).unwrap();
        assert_eq!(result.affected, 2);
        assert_eq!(db.count_rows("companies", 10).unwrap().count, 1);
    }

    #[test]
    fn filter_values_accept_json_scalars() {
        let input: FilterInput =
            serde_json::from_str(r#"{"column":"id","op":"eq","value":1}"#).unwrap();
        match input.into_expr() {
            FilterExpr::Condition { value, .. } => assert_eq!(value.as_deref(), Some("1")),
            other => panic!("unexpected filter: {other:?}"),
        }

        let input: FilterInput = serde_json::from_str(
            r#"{"type":"not","filter":{"type":"condition","column":"active","op":"eq","value":true}}"#,
        )
        .unwrap();
        match input.into_expr() {
            FilterExpr::Not { filter } => match *filter {
                FilterExpr::Condition { value, .. } => assert_eq!(value.as_deref(), Some("1")),
                other => panic!("unexpected filter: {other:?}"),
            },
            other => panic!("unexpected filter: {other:?}"),
        }

        let input: FilterInput =
            serde_json::from_str(r#"{"column":"city","op":"is_null","value":null}"#).unwrap();
        match input.into_expr() {
            FilterExpr::Condition { value, .. } => assert_eq!(value, None),
            other => panic!("unexpected filter: {other:?}"),
        }

        assert!(
            serde_json::from_str::<FilterInput>(r#"{"column":"id","op":"eq","value":[1]}"#)
                .is_err()
        );
    }

    #[test]
    fn drops_renames_and_manages_triggers() {
        let (_dir, db) = temp_companies_db();
        let trigger = db
            .create_trigger(
                "companies_touch",
                "CREATE TRIGGER companies_touch AFTER UPDATE ON companies
                 BEGIN UPDATE companies SET city = city WHERE id = NEW.id; END",
            )
            .unwrap();
        assert!(trigger.migration_name.contains("trigger_companies_touch"));
        assert_eq!(db.triggers(None).unwrap().len(), 1);

        db.rename_table("companies", "companies2").unwrap();
        assert!(db.table_exists("companies2").unwrap());
        assert!(!db.table_exists("companies").unwrap());
        assert_eq!(db.triggers(Some("companies2")).unwrap().len(), 1);

        db.drop_trigger("companies_touch").unwrap();
        assert!(db.triggers(None).unwrap().is_empty());
        db.drop_table("companies2").unwrap();
        assert!(!db.table_exists("companies2").unwrap());
    }

    #[test]
    fn runs_vacuum_analyze_and_pragmas() {
        let (_dir, db) = temp_companies_db();
        let write = db.pragma("user_version=7", true).unwrap();
        assert!(write.session_id.is_some());
        let read = db.pragma("user_version", false).unwrap();
        assert!(read.session_id.is_none());
        assert!(matches!(
            read.rows[0].cells[0],
            CellValue::Integer { value: 7 }
        ));
        assert!(db.analyze().unwrap().session_id.is_some());
        assert!(db.vacuum().unwrap().session_id.is_some());
    }

    #[test]
    fn explains_query_plans_and_flags_full_scans() {
        let (_dir, db) = temp_companies_db();
        let indexed = db
            .explain_query("SELECT * FROM companies WHERE domain = 'acme.test'")
            .unwrap();
        assert!(!indexed.steps.is_empty());
        assert!(indexed.indexes_used >= 1);
        assert_eq!(indexed.full_scans, 0);

        let full_scan = db.explain_query("SELECT * FROM companies").unwrap();
        assert!(full_scan.full_scans >= 1);
    }

    #[test]
    fn data_version_changes_from_another_connection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("version.db");
        let (db, _) = Database::create(&path, false).unwrap();
        let before = db.data_version().unwrap();

        let other = Database::open(&path).unwrap();
        other
            .conn
            .execute_batch("CREATE TABLE external_change (id INTEGER PRIMARY KEY);")
            .unwrap();

        let after = db.data_version().unwrap();
        assert_ne!(before, after);
    }
}

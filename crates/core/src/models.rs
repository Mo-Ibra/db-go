//! Serializable data types shared by the core, CLI, and GUI.

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
    /// Convert a legacy single-condition filter into the expression-tree form.
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
    /// Normalize either a legacy filter or an expression tree into a filter expression.
    pub fn into_expr(self) -> FilterExpr {
        match self {
            FilterInput::Spec(spec) => spec.into(),
            FilterInput::Expr(expr) => expr,
        }
    }
}

/// Accept string, number, boolean, or null filter values and normalize them to strings.
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

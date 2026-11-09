use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
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
    pub value: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
pub struct DbInfo {
    pub path: String,
    pub tables: Vec<TableInfo>,
    pub backup_path: Option<String>,
    pub project_path: Option<String>,
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

    fn write_schema_md(&self) -> Result<()> {
        let Some(project_dir) = &self.project_dir else {
            return Ok(());
        };
        let graph = self.schema_graph()?;
        std::fs::write(project_dir.join("schema.md"), schema_markdown(&graph))?;
        Ok(())
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

    pub fn refresh_agent_guide(&self) -> Result<PathBuf> {
        let Some(project_dir) = &self.project_dir else {
            return Err(DbError::NotAProject(
                "agent guide requires a project folder".to_string(),
            ));
        };
        let config = read_project_config(project_dir)?;
        let path = project_dir.join("AGENTS.md");
        std::fs::write(&path, agents_markdown(&config))?;
        Ok(path)
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

        let backup = self.backup()?;
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
            backup_path: Some(backup.display().to_string()),
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

        let backup = self.backup()?;
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
            backup_path: Some(backup.display().to_string()),
        })
    }

    pub fn update_rows(
        &self,
        table: &str,
        filter: &FilterSpec,
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

        let backup = self.backup()?;
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let (where_sql, where_params) = filter_clause(filter, &columns)?;
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
            backup_path: Some(backup.display().to_string()),
        })
    }

    pub fn delete_rows(&self, table: &str, filter: &FilterSpec) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        let backup = self.backup()?;
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let (where_sql, where_params) = filter_clause(filter, &columns)?;
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
            backup_path: Some(backup.display().to_string()),
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

        let backup = self.backup()?;
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
            backup_path: Some(backup.display().to_string()),
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

        self.ensure_migrations_table()?;
        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute_batch(&migration_sql)?;
        transaction.execute(
            "INSERT INTO _dbgo_migrations (name, sql) VALUES (?1, ?2)",
            rusqlite::params![format!("create_table:{table_name}"), migration_sql],
        )?;
        let migration_id = transaction.last_insert_rowid();
        transaction.commit()?;

        if let Some(project_dir) = &self.project_dir {
            if let Err(error) =
                write_migration_file(project_dir, migration_id, table_name, &migration_sql)
            {
                eprintln!("[dbgo] warning: could not write migration file: {error}");
            }
            if let Err(error) = self.write_schema_md() {
                eprintln!("[dbgo] warning: could not update schema.md: {error}");
            }
        }

        Ok(TableInfo {
            name: table_name.to_string(),
            kind: "table".to_string(),
        })
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

fn render_create_table(design: &TableDesign) -> Result<String> {
    let mut definitions = Vec::with_capacity(design.columns.len());
    for column in &design.columns {
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
        definitions.push(format!("  {}", parts.join(" ")));
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

fn write_project_files(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    let config_json = serde_json::to_string_pretty(config)?;
    std::fs::write(project_dir.join("dbgo.json"), config_json)?;
    std::fs::write(project_dir.join("AGENTS.md"), agents_markdown(config))?;
    std::fs::write(
        project_dir.join("schema.md"),
        "# Schema\n\nNo tables yet.\n",
    )?;
    Ok(())
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
- Every write creates a timestamped backup and an audit row.

## GUI edits

The desktop app stages row edits and writes them only when the user clicks
**Save changes**. Each save creates one backup and one audit session. The app
does not reload the current page while edits are staged, so an agent can write
safely and the user decides when to reload.

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

## Backups and audit

- Every write creates a timestamped snapshot in `backups/`.
- `dbgo backup .` creates one explicitly.
- `dbgo sessions .` lists recent write sessions and their action counts.

## Rules

- Use `dbgo`; never write the SQLite file directly.
- Run `dbgo graph . --json` before changing the schema.
- Use `--json` or `--jsonl` for machine-readable output.
- Run commands from this folder and use `.` as the database path.
- Use `--yes` for update/delete.
- Prefer `upsert` with a natural key over blind inserts.
- Do not update primary key columns.
- Do not edit `migrations/` by hand unless applying them to D1.
"#;

    TEMPLATE
        .replace("{{NAME}}", &config.name)
        .replace("{{DATABASE}}", &config.database)
}

fn write_migration_file(
    project_dir: &Path,
    migration_id: i64,
    table: &str,
    sql: &str,
) -> Result<()> {
    let migrations = project_dir.join("migrations");
    std::fs::create_dir_all(&migrations)?;
    let path = migrations.join(format!("{:04}_create_{}.sql", migration_id, table));
    std::fs::write(path, format!("{sql}\n"))?;
    Ok(())
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

fn filter_clause(
    filter: &FilterSpec,
    columns: &[ColumnInfo],
) -> Result<(String, Vec<rusqlite::types::Value>)> {
    if !columns.iter().any(|column| column.name == filter.column) {
        return Err(DbError::UnknownColumn(filter.column.clone()));
    }
    let column = quote_ident(&filter.column);
    let mut params = Vec::new();

    let clause = match filter.op {
        FilterOp::Contains => {
            let value = filter
                .value
                .as_deref()
                .ok_or(DbError::MissingFilterValue(filter.op))?;
            params.push(rusqlite::types::Value::Text(format!(
                "%{}%",
                escape_like(value)
            )));
            format!(" WHERE CAST({column} AS TEXT) LIKE ? ESCAPE '\\'")
        }
        FilterOp::Eq
        | FilterOp::Neq
        | FilterOp::Gt
        | FilterOp::Gte
        | FilterOp::Lt
        | FilterOp::Lte => {
            let operator = match filter.op {
                FilterOp::Eq => "=",
                FilterOp::Neq => "<>",
                FilterOp::Gt => ">",
                FilterOp::Gte => ">=",
                FilterOp::Lt => "<",
                FilterOp::Lte => "<=",
                _ => unreachable!(),
            };
            let value = filter
                .value
                .as_deref()
                .ok_or(DbError::MissingFilterValue(filter.op))?;
            params.push(rusqlite::types::Value::Text(value.to_string()));
            format!(" WHERE {column} {operator} ?")
        }
        FilterOp::IsNull => format!(" WHERE {column} IS NULL"),
        FilterOp::NotNull => format!(" WHERE {column} IS NOT NULL"),
    };

    Ok((clause, params))
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
        assert!(result.backup_path.is_some());
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
        assert_eq!(city_of(&db, "acme.test"), "Berlin");

        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].actions, 1);
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
        assert!(result.backup_path.is_some());
        assert_eq!(db.count_rows("companies", 100).unwrap().count, 2);
        assert_eq!(city_of(&db, "acme.test"), "Munich");
        assert_eq!(city_of(&db, "initech.test"), "Tokyo");

        let sessions = db.sessions().unwrap();
        assert_eq!(sessions.len(), 2);
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

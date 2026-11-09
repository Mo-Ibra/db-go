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

        let mut where_sql = String::new();
        let mut params: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(filter) = filter {
            if !columns.iter().any(|column| column.name == filter.column) {
                return Err(DbError::UnknownColumn(filter.column.clone()));
            }

            let column = quote_ident(&filter.column);
            match filter.op {
                FilterOp::Contains => {
                    let value = filter
                        .value
                        .as_deref()
                        .ok_or(DbError::MissingFilterValue(filter.op))?;
                    where_sql
                        .push_str(&format!(" WHERE CAST({column} AS TEXT) LIKE ? ESCAPE '\\'"));
                    params.push(rusqlite::types::Value::Text(format!(
                        "%{}%",
                        escape_like(value)
                    )));
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
                    where_sql.push_str(&format!(" WHERE {column} {operator} ?"));
                    params.push(rusqlite::types::Value::Text(value.to_string()));
                }
                FilterOp::IsNull => where_sql.push_str(&format!(" WHERE {column} IS NULL")),
                FilterOp::NotNull => where_sql.push_str(&format!(" WHERE {column} IS NOT NULL")),
            }
        }

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

## Backups

- `dbgo backup .` writes a timestamped snapshot into `backups/`.
- The database, migrations, and backups live in this folder.

## Rules

- Use `dbgo`; never write the SQLite file directly.
- Run `dbgo graph . --json` before changing the schema.
- Use `--json` or `--jsonl` for machine-readable output.
- Run commands from this folder and use `.` as the database path.
- Do not edit `migrations/` by hand unless applying them to D1.

## Current limitations

- Insert / update / delete are the next milestone.
- MCP is not available yet; use the CLI.
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
}

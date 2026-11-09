use std::path::{Path, PathBuf};

use base64::Engine as _;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use serde::Serialize;
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

#[derive(Debug, Clone, Serialize)]
pub struct DbInfo {
    pub path: String,
    pub tables: Vec<TableInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Next,
    Prev,
}

pub struct Database {
    path: PathBuf,
    conn: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
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
        Ok(Self { path, conn })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn tables(&self) -> Result<Vec<TableInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, type
             FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
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

    fn ensure_table(&self, table: &str) -> Result<()> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = ?1
             )",
            [table],
            |row| row.get(0),
        )?;
        if exists {
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
}

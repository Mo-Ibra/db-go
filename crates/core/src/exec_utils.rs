//! Low-level execution, row conversion, and audit helpers.

use super::*;

/// Execute one statement that returns rows, such as INSERT/UPDATE/DELETE ... RETURNING.
pub(crate) fn exec_returning(
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

/// Encode bytes as lowercase hexadecimal without a prefix.
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

/// Convert a SQLite row into a JSON object keyed by column name.
pub(crate) fn row_to_json(
    columns: &[ColumnInfo],
    row: &rusqlite::Row<'_>,
) -> Result<serde_json::Value> {
    let mut map = serde_json::Map::new();
    for (index, column) in columns.iter().enumerate() {
        map.insert(column.name.clone(), cell_to_json(row.get_ref(index + 1)?));
    }
    Ok(serde_json::Value::Object(map))
}

/// Fetch one row by rowid and return it as a JSON object.
pub(crate) fn fetch_row_json(
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

/// Run a filtered SELECT and return rowid/JSON pairs used for auditing writes.
pub(crate) fn select_rows_json(
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

/// Insert one audit-log row for a write or schema change.
pub(crate) fn record_audit(
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

/// Generate a unique, sortable write-session identifier.
pub(crate) fn new_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        now_millis(),
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// Remove a database sidecar file such as -wal or -shm when present.
pub(crate) fn remove_sidecar(path: &Path, suffix: &str) {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(suffix);
    let _ = std::fs::remove_file(PathBuf::from(sidecar));
}

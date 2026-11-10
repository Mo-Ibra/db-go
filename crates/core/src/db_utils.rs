//! Row/value validation and JSON/CSV conversion helpers.

use super::*;

/// Ensure a JSON object key names an existing table column.
pub(crate) fn validate_json_column(name: &str, columns: &[ColumnInfo]) -> Result<()> {
    if columns.iter().any(|column| column.name == name) {
        Ok(())
    } else {
        Err(DbError::UnknownColumn(name.to_string()))
    }
}

/// Ensure every key in a JSON row names an existing column.
pub(crate) fn validate_json_columns(row: &JsonObject, columns: &[ColumnInfo]) -> Result<()> {
    for name in row.keys() {
        validate_json_column(name, columns)?;
    }
    Ok(())
}

/// Validate update payload keys and reject primary-key updates.
pub(crate) fn validate_update_values(values: &JsonObject, columns: &[ColumnInfo]) -> Result<()> {
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

/// Convert a JSON scalar into the corresponding SQLite value.
pub(crate) fn json_to_sql(value: &serde_json::Value) -> rusqlite::types::Value {
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

/// Convert a SQLite value into JSON for display or audit output.
pub(crate) fn cell_to_json(value: ValueRef<'_>) -> serde_json::Value {
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

/// Convert a SQLite value into export JSON, base64-encoding blobs.
pub(crate) fn export_json(value: ValueRef<'_>) -> serde_json::Value {
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

/// Convert a SQLite value into CSV cell text, base64-encoding blobs.
pub(crate) fn csv_value(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(n) => n.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(b) => base64::engine::general_purpose::STANDARD.encode(b),
    }
}

/// Quote a CSV cell when it contains delimiters, quotes, or newlines.
pub(crate) fn csv_escape(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

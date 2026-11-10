//! SQL identifier validation and DDL rendering helpers.

use super::*;

/// Quote a SQL identifier with double quotes, escaping embedded quotes.
pub(crate) fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Convert a raw SQLite value into a serializable cell value, base64-encoding blobs.
pub(crate) fn cell_from_value_ref(value: ValueRef<'_>) -> CellValue {
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

/// Escape LIKE wildcard characters so user input is matched literally.
pub(crate) fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Validate a table, column, index, or trigger identifier.
pub(crate) fn validate_ident(name: &str) -> Result<()> {
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

pub(crate) const ALLOWED_TYPES: &[&str] = &[
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

/// Normalize and validate a declared column type against the supported SQLite types.
pub(crate) fn normalize_type(value: &str) -> Result<String> {
    let normalized = value.trim().to_ascii_uppercase();
    if ALLOWED_TYPES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(DbError::InvalidDesign(format!(
            "unsupported column type: {value}"
        )))
    }
}

/// Normalize and validate a foreign-key ON DELETE action.
pub(crate) fn normalize_on_delete(value: Option<&str>) -> Result<String> {
    let normalized = value.unwrap_or("NO ACTION").trim().to_ascii_uppercase();
    match normalized.as_str() {
        "NO ACTION" | "CASCADE" | "SET NULL" | "RESTRICT" => Ok(normalized),
        _ => Err(DbError::InvalidDesign(format!(
            "unsupported ON DELETE action: {normalized}"
        ))),
    }
}

/// Render a column default as SQL, quoting strings and recognizing keywords.
pub(crate) fn render_default(value: &str) -> Result<String> {
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

/// Render one column definition for a CREATE TABLE statement.
pub(crate) fn render_column_definition(column: &ColumnDesign) -> Result<String> {
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

/// Render the complete CREATE TABLE statement for a table design.
pub(crate) fn render_create_table(design: &TableDesign) -> Result<String> {
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

/// Build a CREATE INDEX statement for a single foreign-key column.
pub(crate) fn render_index_for_column(table: &str, column: &str) -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {} ON {} ({})",
        quote_ident(&format!("idx_{table}_{column}")),
        quote_ident(table),
        quote_ident(column)
    )
}

//! Shared CLI helpers for opening databases and reading input files.

use std::path::Path;

use db_go_core::{AlterTableOp, ColumnDesign, Database, ImportFormat, JsonObject};

/// Open a database file or DBGo project directory, returning a user-facing error if missing.
pub(crate) fn open(path: &Path) -> Result<Database, String> {
    if !path.exists() {
        return Err(format!("database or project not found: {}", path.display()));
    }
    Database::open_project(path).map_err(err)
}

/// Infer CSV, JSON, or JSONL import format from a file extension.
pub(crate) fn infer_import_format(path: &Path) -> Result<ImportFormat, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    match extension.as_deref() {
        Some("csv") => Ok(ImportFormat::Csv),
        Some("json") => Ok(ImportFormat::Json),
        Some("jsonl") | Some("ndjson") => Ok(ImportFormat::Jsonl),
        Some(other) => Err(format!(
            "cannot infer import format from .{other}; use --format"
        )),
        None => Err("cannot infer import format; use --format".to_string()),
    }
}

/// Read a JSONL file into row objects, reporting the offending line on errors.
pub(crate) fn read_jsonl(path: &Path) -> Result<Vec<JsonObject>, String> {
    let content = std::fs::read_to_string(path).map_err(err)?;
    let mut rows = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|error| format!("line {}: {error}", index + 1))?;
        let object = value
            .as_object()
            .ok_or_else(|| format!("line {}: expected a JSON object", index + 1))?
            .clone();
        rows.push(object);
    }
    if rows.is_empty() {
        return Err(format!("no rows found in {}", path.display()));
    }
    Ok(rows)
}

/// Combine an operations file with repeatable add, rename, and drop options.
pub(crate) fn build_alter_operations(
    file: Option<&Path>,
    add_columns: &[String],
    rename_columns: &[String],
    drop_columns: &[String],
) -> Result<Vec<AlterTableOp>, String> {
    let mut operations = Vec::new();

    if let Some(path) = file {
        let content = std::fs::read_to_string(path).map_err(err)?;
        let value: serde_json::Value = serde_json::from_str(&content).map_err(err)?;
        let operations_value = if value.is_array() {
            value
        } else if let Some(operations) = value.get("operations") {
            operations.clone()
        } else {
            return Err(format!(
                "{}: expected a JSON array or an object with an operations array",
                path.display()
            ));
        };
        let parsed: Vec<AlterTableOp> = serde_json::from_value(operations_value).map_err(err)?;
        operations.extend(parsed);
    }

    for spec in add_columns {
        let column: ColumnDesign = serde_json::from_str(spec).map_err(err)?;
        operations.push(AlterTableOp::AddColumn { column });
    }
    for spec in rename_columns {
        let (from, to) = spec
            .split_once('=')
            .or_else(|| spec.split_once(':'))
            .ok_or_else(|| format!("rename expects FROM=TO: {spec}"))?;
        operations.push(AlterTableOp::RenameColumn {
            from: from.trim().to_string(),
            to: to.trim().to_string(),
        });
    }
    for name in drop_columns {
        operations.push(AlterTableOp::DropColumn {
            name: name.trim().to_string(),
        });
    }

    Ok(operations)
}

/// Convert an error into the CLI's string error type.
pub(crate) fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

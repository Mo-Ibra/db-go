//! CSV, JSON, and JSONL import parsing, inference, and mapping.

use super::*;

/// Parse an import file and return a preview of columns, inferred types, and first rows.
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

/// Dispatch to the CSV, JSON, or JSONL reader for an import file.
pub(crate) fn read_import_source(
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

/// Read CSV data, optionally treating the first row as headers.
pub(crate) fn read_csv_source(
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

/// Read a JSON object or array of objects as import rows.
pub(crate) fn read_json_source(path: &Path) -> Result<(Vec<String>, Vec<JsonObject>)> {
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

/// Read one JSON object per line as import rows.
pub(crate) fn read_jsonl_source(path: &Path) -> Result<(Vec<String>, Vec<JsonObject>)> {
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

/// Collect the union of row keys, preserving first-seen order.
pub(crate) fn collect_import_columns(rows: &[JsonObject]) -> Vec<String> {
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

/// Parse a one-character or escaped-tab CSV delimiter.
pub(crate) fn parse_csv_delimiter(value: Option<&str>) -> Result<u8> {
    match value {
        None => Ok(b','),
        Some(value) if value == "\\t" || value.eq_ignore_ascii_case("tab") => Ok(b'\t'),
        Some(value) if value.len() == 1 => Ok(value.as_bytes()[0]),
        Some(value) => Err(DbError::InvalidDesign(format!(
            "CSV delimiter must be one character: {value}"
        ))),
    }
}

/// Convert a CSV reader error into a DBGo error.
pub(crate) fn csv_error(error: csv::Error) -> DbError {
    DbError::InvalidDesign(format!("CSV error: {error}"))
}

/// Infer import column names and SQLite types from sample rows.
pub(crate) fn infer_import_columns(
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

/// Infer a SQLite type for one imported column from its string values.
pub(crate) fn infer_import_type<'a>(
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

/// Return true for recognized boolean spellings.
pub(crate) fn is_boolean_text(value: &str) -> bool {
    value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false")
}

/// Return true for integers without significant leading zeros.
pub(crate) fn is_canonical_integer(value: &str) -> bool {
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

/// Return true when numeric-looking text has significant leading zeros.
pub(crate) fn is_padded_number(value: &str) -> bool {
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

/// Convert one CSV string into a typed JSON value for its target column.
pub(crate) fn convert_csv_import_value(
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

/// Resolve source-to-target mappings, merging defaults with explicit --map overrides.
pub(crate) fn resolve_import_mapping(
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

/// Derive a default table name from an import file name.
pub(crate) fn suggest_table_name(path: &Path) -> String {
    sanitize_import_identifier(
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("imported"),
        "imported_table",
    )
}

/// Sanitize an import header into a valid SQL identifier.
pub(crate) fn sanitize_import_identifier(raw: &str, fallback: &str) -> String {
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

/// Make an import column identifier unique within a table.
pub(crate) fn unique_import_identifier(
    raw: &str,
    used: &mut HashSet<String>,
    index: usize,
) -> String {
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

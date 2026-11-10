use std::result::Result;

use db_go_core::*;

use super::format::*;

/// Render available backups with sizes and timestamps.
pub(crate) fn output_backups(
    backups: &[BackupInfo],
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(backups);
    }
    if jsonl {
        return write_jsonl(backups);
    }

    let mut table = human_table();
    table.set_header(vec!["name", "size", "modified", "path"]);
    for backup in backups {
        table.add_row(vec![
            backup.name.clone(),
            format_bytes(backup.size_bytes),
            format_age(backup.modified_ms),
            backup.path.clone(),
        ]);
    }
    print_table(table)
}

/// Render a backup restore result.
pub(crate) fn output_restore(
    result: &RestoreResult,
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(result);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(result));
    }
    print_line(&format!("restored from {}", result.restored_from))?;
    if let Some(safety) = &result.safety_backup {
        print_line(&format!("safety backup {safety}"))?;
    }
    print_line(&format!(
        "session {} · {} table(s)",
        result.session_id, result.tables
    ))
}

/// Render recent audit sessions.
pub(crate) fn output_sessions(
    sessions: &[SessionSummary],
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(sessions);
    }
    if jsonl {
        return write_jsonl(sessions);
    }

    let mut table = human_table();
    table.set_header(vec!["session", "actions", "started_at"]);
    for session in sessions {
        table.add_row(vec![
            session.session_id.clone(),
            session.actions.to_string(),
            session.started_at.clone(),
        ]);
    }
    print_table(table)
}

/// Render an import preview with columns, inferred types, rows, and table suggestion.
pub(crate) fn output_import_preview(
    preview: &ImportPreview,
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(preview);
    }
    if jsonl {
        return write_jsonl(&preview.rows);
    }

    print_line(&format!(
        "{} · {} row(s) · suggested table {}",
        import_format_label(preview.format),
        preview.total_rows,
        preview.suggested_table
    ))?;

    let mut columns = human_table();
    columns.set_header(vec!["source", "suggested", "type"]);
    for column in &preview.columns {
        columns.add_row(vec![
            column.source.clone(),
            column.suggested_name.clone(),
            column.inferred_type.clone(),
        ]);
    }
    print_table(columns)?;

    if !preview.rows.is_empty() {
        let mut rows = human_table();
        rows.set_header(
            preview
                .columns
                .iter()
                .map(|column| column.source.clone())
                .collect::<Vec<_>>(),
        );
        for row in &preview.rows {
            rows.add_row(
                preview
                    .columns
                    .iter()
                    .map(|column| {
                        row.get(&column.source)
                            .map(json_value_text)
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>(),
            );
        }
        print_table(rows)?;
    }
    Ok(())
}

/// Render the result of an import.
pub(crate) fn output_import_result(
    result: &ImportResult,
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(result);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(result));
    }

    let mut message = format!(
        "imported {} row(s) into {} · session {}",
        result.rows, result.table, result.session_id
    );
    if result.created_table {
        message.push_str(" · created table");
    }
    if !result.ignored_columns.is_empty() {
        message.push_str(&format!(
            " · ignored source columns: {}",
            result.ignored_columns.join(", ")
        ));
    }
    print_line(&message)
}

/// Render a D1 export summary and generated file paths.
pub(crate) fn output_d1_export(
    result: &D1ExportResult,
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(result);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(result));
    }

    print_line(&format!(
        "exported {} row(s) from {} table(s) to {}",
        result.rows,
        result.tables.len(),
        result.output_dir
    ))?;
    print_line(&format!("schema {}", result.schema_file))?;
    for file in &result.data_files {
        print_line(&format!("data {file}"))?;
    }
    print_line(&format!("wrangler {}", result.wrangler_file))?;
    print_line(&format!("readme {}", result.readme_file))?;
    if !result.warnings.is_empty() {
        print_line("warnings:")?;
        for warning in &result.warnings {
            print_line(&format!("  - {warning}"))?;
        }
    }
    Ok(())
}

/// Render database, table, and index size statistics.
pub(crate) fn output_stats(stats: &DatabaseStats, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(stats);
    }
    if jsonl {
        return write_jsonl(&stats.tables);
    }

    print_line(&format!(
        "{} · {} · page {} · {} pages · free {} · dbstat {}",
        stats.path,
        format_bytes(stats.total_bytes.max(0) as u64),
        stats.page_size,
        stats.page_count,
        format_bytes(stats.free_bytes.max(0) as u64),
        if stats.dbstat { "yes" } else { "unavailable" }
    ))?;

    let mut table = human_table();
    table.set_header(vec!["table", "rows", "table size", "index size"]);
    for stats in &stats.tables {
        let rows = if stats.row_count_capped {
            format!("{}+", stats.row_count)
        } else if stats.row_count_estimated {
            format!("~{}", stats.row_count)
        } else {
            stats.row_count.to_string()
        };
        table.add_row(vec![
            stats.name.clone(),
            rows,
            optional_bytes(stats.size_bytes),
            optional_bytes(stats.index_bytes),
        ]);
    }
    print_table(table)?;

    let mut index_table = human_table();
    index_table.set_header(vec!["table", "index", "size", "columns"]);
    let mut has_indexes = false;
    for table in &stats.tables {
        for index in &table.indexes {
            has_indexes = true;
            index_table.add_row(vec![
                table.name.clone(),
                index.name.clone(),
                optional_bytes(index.size_bytes),
                index.columns.join(", "),
            ]);
        }
    }
    if has_indexes {
        print_table(index_table)?;
    }
    Ok(())
}

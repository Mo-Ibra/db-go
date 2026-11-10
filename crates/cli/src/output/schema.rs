use std::result::Result;

use db_go_core::*;

use super::format::*;

/// Render the table list as JSON, JSONL, or a human table.
pub(crate) fn output_tables(tables: &[TableInfo], json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(tables);
    }
    if jsonl {
        return write_jsonl(tables);
    }

    let mut table = human_table();
    table.set_header(vec!["name", "kind"]);
    for item in tables {
        table.add_row(vec![item.name.clone(), item.kind.clone()]);
    }
    print_table(table)
}

/// Render column metadata as JSON, JSONL, or a human table.
pub(crate) fn output_columns(
    columns: &[ColumnInfo],
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(columns);
    }
    if jsonl {
        return write_jsonl(columns);
    }

    let mut table = human_table();
    table.set_header(vec!["name", "type", "not_null", "primary_key"]);
    for column in columns {
        table.add_row(vec![
            column.name.clone(),
            if column.type_name.is_empty() {
                "-".to_string()
            } else {
                column.type_name.clone()
            },
            yes_no(column.not_null),
            yes_no(column.primary_key),
        ]);
    }
    print_table(table)
}

/// Render the schema graph as JSON, JSONL, or a human-readable tree.
pub(crate) fn output_graph(graph: &SchemaGraph, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(graph);
    }
    if jsonl {
        return write_jsonl(&graph.tables);
    }

    let mut output = String::new();
    for table in &graph.tables {
        output.push_str(&format!("{}\n", table.name));
        for column in &table.columns {
            let marker = if column.primary_key {
                " PK"
            } else if column.foreign_key {
                " FK"
            } else {
                ""
            };
            output.push_str(&format!(
                "  {} {}{}{}\n",
                column.name,
                column.type_name,
                marker,
                if column.not_null { " NOT NULL" } else { "" }
            ));
        }
        output.push('\n');
    }

    if !graph.relationships.is_empty() {
        output.push_str("relationships:\n");
        for relationship in &graph.relationships {
            output.push_str(&format!(
                "  {}.{} -> {}.{} (ON DELETE {})\n",
                relationship.from_table,
                relationship.from_column,
                relationship.to_table,
                relationship.to_column,
                relationship.on_delete
            ));
        }
    }

    print_line(&output)
}

/// Render a schema-change result, including migration and backup details.
pub(crate) fn output_schema_change(
    change: &SchemaChange,
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(change);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(change));
    }

    print_line(&format!(
        "migration {} · session {}{}",
        change.migration_name,
        change.session_id,
        change
            .backup_path
            .as_deref()
            .map(|path| format!(" · backup {path}"))
            .unwrap_or_default()
    ))?;
    if let Some(path) = &change.migration_path {
        print_line(&format!("file {path}"))?;
    }
    for statement in &change.statements {
        print_line(&format!("{};", statement.trim().trim_end_matches(';')))?;
    }
    Ok(())
}

/// Render index metadata as JSON, JSONL, or a human table.
pub(crate) fn output_indexes(indexes: &[IndexInfo], json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(indexes);
    }
    if jsonl {
        return write_jsonl(indexes);
    }

    let mut table = human_table();
    table.set_header(vec!["name", "table", "unique", "columns", "origin"]);
    for index in indexes {
        table.add_row(vec![
            index.name.clone(),
            index.table.clone(),
            yes_no(index.unique),
            index.columns.join(", "),
            index.origin.clone(),
        ]);
    }
    print_table(table)
}

/// Render trigger metadata as JSON, JSONL, or a human table.
pub(crate) fn output_triggers(
    triggers: &[TriggerInfo],
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(triggers);
    }
    if jsonl {
        return write_jsonl(triggers);
    }

    let mut table = human_table();
    table.set_header(vec!["name", "table", "sql"]);
    for trigger in triggers {
        table.add_row(vec![
            trigger.name.clone(),
            trigger.table.clone(),
            truncate(&trigger.sql.replace(['\n', '\r', '\t'], " "), 100),
        ]);
    }
    print_table(table)
}

/// Render a maintenance result, including returned PRAGMA rows.
pub(crate) fn output_maintenance(
    result: &MaintenanceResult,
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(result);
    }
    if jsonl {
        if !result.rows.is_empty() {
            return write_jsonl(&result.rows);
        }
        return write_jsonl(std::slice::from_ref(result));
    }

    if !result.columns.is_empty() {
        let mut table = human_table();
        table.set_header(
            result
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect::<Vec<_>>(),
        );
        for row in &result.rows {
            table.add_row(row.cells.iter().map(cell_to_string).collect::<Vec<_>>());
        }
        print_table(table)?;
    }

    let session = result
        .session_id
        .as_deref()
        .map(|session| format!(" · session {session}"))
        .unwrap_or_default();
    print_line(&format!("{}{}", result.action, session))
}

/// Render views as JSON, JSONL, or a human table.
pub(crate) fn output_views(views: &[ViewInfo], json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(views);
    }
    if jsonl {
        return write_jsonl(views);
    }

    let mut table = human_table();
    table.set_header(vec!["name", "sql"]);
    for view in views {
        table.add_row(vec![
            view.name.clone(),
            truncate(&view.sql.replace(['\n', '\r', '\t'], " "), 100),
        ]);
    }
    print_table(table)
}

/// Render applied and pending migration status.
pub(crate) fn output_migrations(
    migrations: &[MigrationInfo],
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(migrations);
    }
    if jsonl {
        return write_jsonl(migrations);
    }

    let mut table = human_table();
    table.set_header(vec!["name", "status", "applied_at", "file"]);
    for migration in migrations {
        table.add_row(vec![
            migration.name.clone(),
            if migration.applied {
                "applied".to_string()
            } else {
                "pending".to_string()
            },
            migration.applied_at.clone().unwrap_or_default(),
            migration.path.clone().unwrap_or_default(),
        ]);
    }
    print_table(table)
}

/// Render the result of applying migrations.
pub(crate) fn output_migrate(
    result: &MigrateResult,
    json: bool,
    jsonl: bool,
) -> Result<(), String> {
    if json {
        return write_json(result);
    }
    if jsonl {
        return write_jsonl(&result.applied);
    }
    if result.applied.is_empty() {
        return print_line("no pending migrations");
    }
    for migration in &result.applied {
        print_line(&format!(
            "applied {} · session {}{}",
            migration.name,
            migration.session_id,
            migration
                .backup_path
                .as_deref()
                .map(|path| format!(" · backup {path}"))
                .unwrap_or_default()
        ))?;
    }
    print_line(&format!("{} migration(s) applied", result.applied.len()))
}

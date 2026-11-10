use std::result::Result;

use db_go_core::*;

use super::format::*;

/// Render a write result with affected rows and audit session.
pub(crate) fn output_write(result: &WriteResult, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(result);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(result));
    }
    print_line(&format!(
        "{} row(s) · session {}{}",
        result.affected,
        result.session_id,
        result
            .backup_path
            .as_deref()
            .map(|path| format!(" · backup {path}"))
            .unwrap_or_default()
    ))
}

/// Render a row-count result, noting when the cap was reached.
pub(crate) fn output_count(count: &CountResult, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(count);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(count));
    }

    if count.capped {
        print_line(&format!("{}+ (capped)", count.count))
    } else {
        print_line(&count.count.to_string())
    }
}

/// Render one table page and its pagination cursors.
pub(crate) fn output_page(page: &Page, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(page);
    }
    if jsonl {
        return write_jsonl(&page.rows);
    }

    let mut table = human_table();
    let mut header = vec!["rowid".to_string()];
    header.extend(page.columns.iter().map(|column| column.name.clone()));
    table.set_header(header);

    for row in &page.rows {
        let mut values = Vec::with_capacity(row.cells.len() + 1);
        values.push(
            row.rowid
                .map(|rowid| rowid.to_string())
                .unwrap_or_else(String::new),
        );
        values.extend(row.cells.iter().map(cell_to_string));
        table.add_row(values);
    }

    print_table(table)?;
    print_line(&format!(
        "rows: {}  next_cursor: {}  has_more: {}",
        page.rows.len(),
        page.next_cursor
            .map(|cursor| cursor.to_string())
            .unwrap_or_else(|| "-".to_string()),
        page.has_more
    ))
}

/// Render a query result and any truncation notice.
pub(crate) fn output_query(result: &QueryResult, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(result);
    }
    if jsonl {
        return write_jsonl(&result.rows);
    }

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
    if result.truncated {
        print_line("(truncated — raise --limit to see more)")?;
    }
    Ok(())
}

/// Render arbitrary SQL execution results, including returned rows.
pub(crate) fn output_exec(result: &ExecResult, json: bool, jsonl: bool) -> Result<(), String> {
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
        if result.truncated {
            print_line("(truncated — raise --limit to see more)")?;
        }
        if let Some(session) = &result.session_id {
            print_line(&format!(
                "affected {} row(s) · session {session} · {}",
                result.affected,
                if result.transactional {
                    "transactional"
                } else {
                    "no transaction"
                }
            ))?;
        }
        return Ok(());
    }

    if result.read_only {
        return print_line("no rows returned");
    }

    print_line(&format!(
        "affected {} row(s){}{}",
        result.affected,
        result
            .session_id
            .as_deref()
            .map(|session| format!(" · session {session}"))
            .unwrap_or_default(),
        if result.transactional {
            " · transactional"
        } else {
            " · no transaction"
        }
    ))
}

/// Render a query plan with full-scan and index warnings.
pub(crate) fn output_plan(plan: &QueryPlan, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(plan);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(plan));
    }

    let mut table = human_table();
    table.set_header(vec!["id", "parent", "detail"]);
    for step in &plan.steps {
        table.add_row(vec![
            step.id.to_string(),
            step.parent.to_string(),
            if step.full_scan {
                format!("⚠ {}", step.detail)
            } else {
                step.detail.clone()
            },
        ]);
    }
    print_table(table)?;
    print_line(&format!(
        "{} step(s) · {} full scan(s) · {} index lookup(s)",
        plan.steps.len(),
        plan.full_scans,
        plan.indexes_used
    ))
}

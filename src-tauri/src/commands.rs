use std::collections::BTreeMap;

use db_go_core::{
    ColumnInfo, CountResult, Database, DbInfo, Direction, FilterSpec, Page, QueryResult,
    SchemaGraph, SortSpec, TableDesign, TableInfo,
};
use tauri::State;

use crate::state::AppState;

fn to_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn lock_error() -> String {
    "database lock poisoned".to_string()
}

#[tauri::command]
pub async fn initial_database() -> Result<Option<String>, String> {
    let candidate = std::env::args()
        .skip(1)
        .find(|argument| !argument.starts_with('-'));
    match candidate {
        Some(path) if std::path::Path::new(&path).exists() => Ok(Some(path)),
        _ => Ok(None),
    }
}

#[tauri::command]
pub async fn open_database(path: String, state: State<'_, AppState>) -> Result<DbInfo, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<DbInfo, String> {
        let database = Database::open_project(&path).map_err(to_error)?;
        let database_path = database.path().display().to_string();
        let tables = database.tables().map_err(to_error)?;
        eprintln!(
            "[dbgo] opened database: {database_path} ({} tables)",
            tables.len()
        );
        let info = DbInfo {
            path: database_path,
            tables,
            backup_path: None,
            project_path: database.project_dir().map(|p| p.display().to_string()),
        };
        *db.lock().map_err(|_| lock_error())? = Some(database);
        Ok(info)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn create_project(
    path: String,
    overwrite: bool,
    state: State<'_, AppState>,
) -> Result<DbInfo, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<DbInfo, String> {
        let (database, backup) = Database::create_project(&path, overwrite).map_err(to_error)?;
        let tables = database.tables().map_err(to_error)?;
        let database_path = database.path().display().to_string();
        let project_path = database.project_dir().map(|p| p.display().to_string());
        let backup_path = backup.map(|p| p.display().to_string());
        eprintln!(
            "[dbgo] created project: {path} (database: {database_path}, backup: {})",
            backup_path.as_deref().unwrap_or("none")
        );
        let info = DbInfo {
            path: database_path,
            tables,
            backup_path,
            project_path,
        };
        *db.lock().map_err(|_| lock_error())? = Some(database);
        Ok(info)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn create_database(
    path: String,
    overwrite: bool,
    state: State<'_, AppState>,
) -> Result<DbInfo, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<DbInfo, String> {
        let (database, backup) = Database::create(&path, overwrite).map_err(to_error)?;
        let tables = database.tables().map_err(to_error)?;
        let info = DbInfo {
            path: path.clone(),
            tables,
            backup_path: backup.map(|path| path.display().to_string()),
            project_path: database.project_dir().map(|p| p.display().to_string()),
        };
        eprintln!(
            "[dbgo] created database: {path} (backup: {})",
            info.backup_path.as_deref().unwrap_or("none")
        );
        *db.lock().map_err(|_| lock_error())? = Some(database);
        Ok(info)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn create_table(
    design: TableDesign,
    state: State<'_, AppState>,
) -> Result<TableInfo, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<TableInfo, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let table = database.create_table(&design).map_err(to_error)?;
        eprintln!("[dbgo] created table: {}", table.name);
        Ok(table)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn close_database(state: State<'_, AppState>) -> Result<(), String> {
    *state.db.lock().map_err(|_| lock_error())? = None;
    Ok(())
}

#[tauri::command]
pub async fn list_tables(state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<TableInfo>, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        database.tables().map_err(to_error)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn describe_table(
    table: String,
    state: State<'_, AppState>,
) -> Result<Vec<ColumnInfo>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<ColumnInfo>, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        database.columns(&table).map_err(to_error)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn load_page(
    table: String,
    cursor: Option<i64>,
    direction: String,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Page, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Page, String> {
        let direction = match direction.as_str() {
            "prev" => Direction::Prev,
            _ => Direction::Next,
        };
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let page = database
            .page(&table, cursor, direction, limit)
            .map_err(to_error)?;
        eprintln!(
            "[dbgo] page table={table} cursor={cursor:?} rows={} limit={limit}",
            page.rows.len()
        );
        Ok(page)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn load_table_page(
    table: String,
    filter: Option<FilterSpec>,
    sort: Option<SortSpec>,
    offset: i64,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Page, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Page, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        database
            .table_page(&table, filter.as_ref(), sort.as_ref(), offset, limit)
            .map_err(to_error)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn run_query(
    sql: String,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<QueryResult, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let result = database.query(&sql, limit).map_err(to_error)?;
        eprintln!(
            "[dbgo] query rows={} truncated={}",
            result.rows.len(),
            result.truncated
        );
        Ok(result)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn schema_map(
    state: State<'_, AppState>,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(
        move || -> Result<BTreeMap<String, Vec<String>>, String> {
            let guard = db.lock().map_err(|_| lock_error())?;
            let database = guard
                .as_ref()
                .ok_or_else(|| "no database open".to_string())?;
            let map = database.schema_map().map_err(to_error)?;
            eprintln!("[dbgo] schema map: {} tables", map.len());
            Ok(map)
        },
    )
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn schema_graph(state: State<'_, AppState>) -> Result<SchemaGraph, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<SchemaGraph, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let graph = database.schema_graph().map_err(to_error)?;
        eprintln!(
            "[dbgo] schema graph: {} tables, {} relationships",
            graph.tables.len(),
            graph.relationships.len()
        );
        Ok(graph)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn count_rows(table: String, state: State<'_, AppState>) -> Result<CountResult, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<CountResult, String> {
        // Use a separate connection so a capped count on a large table never
        // blocks paging on the main connection.
        let path = {
            let guard = db.lock().map_err(|_| lock_error())?;
            let database = guard
                .as_ref()
                .ok_or_else(|| "no database open".to_string())?;
            database.path().to_path_buf()
        };
        let database = Database::open(path).map_err(to_error)?;
        database.count_rows(&table, 1_000_000).map_err(to_error)
    })
    .await
    .map_err(to_error)?
}

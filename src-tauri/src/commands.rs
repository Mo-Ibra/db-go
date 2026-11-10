use std::collections::BTreeMap;

use db_go_core::{
    AlterTableOp, BackupInfo, ColumnInfo, CountResult, Database, DbInfo, Direction, FilterSpec,
    JsonObject, MigrateResult, MigrationInfo, Page, QueryResult, RestoreResult, RowUpdate,
    SchemaChange, SchemaGraph, SortSpec, TableDesign, TableDetails, TableInfo, WriteResult,
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
        let mut database = Database::open_project(&path).map_err(to_error)?;
        let (agent_files, agent_files_error) = match database.ensure_agent_files() {
            Ok(info) => (Some(info), None),
            Err(error) => {
                eprintln!("[dbgo] warning: could not create agent files: {error}");
                (None, Some(error.to_string()))
            }
        };
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
            agent_files,
            agent_files_error,
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
            agent_files: None,
            agent_files_error: None,
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
            agent_files: None,
            agent_files_error: None,
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
pub async fn table_details(
    table: String,
    state: State<'_, AppState>,
) -> Result<TableDetails, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<TableDetails, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        database.table_details(&table).map_err(to_error)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn preview_alter_table(
    table: String,
    operations: Vec<AlterTableOp>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<String>, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        database
            .preview_alter_table(&table, &operations)
            .map_err(to_error)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn alter_table(
    table: String,
    operations: Vec<AlterTableOp>,
    state: State<'_, AppState>,
) -> Result<SchemaChange, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<SchemaChange, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let change = database
            .alter_table(&table, &operations)
            .map_err(to_error)?;
        eprintln!(
            "[dbgo] altered table {table} migration={} session={}",
            change.migration_name, change.session_id
        );
        Ok(change)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn create_index(
    table: String,
    name: String,
    columns: Vec<String>,
    unique: bool,
    state: State<'_, AppState>,
) -> Result<SchemaChange, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<SchemaChange, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let change = database
            .create_index(&table, &name, &columns, unique)
            .map_err(to_error)?;
        eprintln!(
            "[dbgo] created index {name} on {table} migration={}",
            change.migration_name
        );
        Ok(change)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn drop_index(name: String, state: State<'_, AppState>) -> Result<SchemaChange, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<SchemaChange, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let change = database.drop_index(&name).map_err(to_error)?;
        eprintln!(
            "[dbgo] dropped index {name} migration={}",
            change.migration_name
        );
        Ok(change)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn list_migrations(state: State<'_, AppState>) -> Result<Vec<MigrationInfo>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<MigrationInfo>, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        database.migrations().map_err(to_error)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn apply_migrations(state: State<'_, AppState>) -> Result<MigrateResult, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<MigrateResult, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let result = database.apply_migrations().map_err(to_error)?;
        eprintln!("[dbgo] applied {} migration(s)", result.applied.len());
        Ok(result)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupInfo>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<BackupInfo>, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        database.backups().map_err(to_error)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn restore_backup(
    backup: String,
    state: State<'_, AppState>,
) -> Result<RestoreResult, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<RestoreResult, String> {
        let mut guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_mut()
            .ok_or_else(|| "no database open".to_string())?;
        let result = database.restore_backup(&backup).map_err(to_error)?;
        eprintln!(
            "[dbgo] restored {} tables={} session={}",
            result.restored_from, result.tables, result.session_id
        );
        Ok(result)
    })
    .await
    .map_err(to_error)?
}

#[tauri::command]
pub async fn save_changes(
    table: String,
    updates: Vec<RowUpdate>,
    inserts: Vec<JsonObject>,
    deletes: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<WriteResult, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<WriteResult, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        let result = database
            .save_changes(&table, &updates, &inserts, &deletes)
            .map_err(to_error)?;
        eprintln!(
            "[dbgo] saved {} change(s) session={}",
            result.affected, result.session_id
        );
        Ok(result)
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
pub async fn data_version(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<i64, String> {
        let guard = db.lock().map_err(|_| lock_error())?;
        let database = guard
            .as_ref()
            .ok_or_else(|| "no database open".to_string())?;
        database.data_version().map_err(to_error)
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

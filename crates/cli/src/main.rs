use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use db_go_core::{
    AlterTableOp, BackupInfo, CellValue, ColumnDesign, ColumnInfo, CountResult, Database,
    Direction, FilterSpec, IndexInfo, JsonObject, MigrateResult, MigrationInfo, Page, QueryResult,
    RestoreResult, SchemaChange, SchemaGraph, SessionSummary, TableDesign, TableInfo, ViewInfo,
    WriteResult,
};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "dbgo", version, about = "Local-first SQL workbench CLI.")]
struct Cli {
    /// Emit a single JSON document.
    #[arg(long, global = true, conflicts_with = "jsonl")]
    json: bool,

    /// Emit one JSON object per line.
    #[arg(long, global = true, conflicts_with = "json")]
    jsonl: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new database project (or a single-file database with --single-file).
    Init {
        db: PathBuf,
        /// Replace an existing database/project, keeping a timestamped backup.
        #[arg(long)]
        force: bool,
        /// Create a bare .db file instead of a project folder.
        #[arg(long)]
        single_file: bool,
    },

    /// Show the full schema graph: tables, columns, and relationships.
    Graph { db: PathBuf },

    /// Create a table from a JSON design file.
    CreateTable {
        db: PathBuf,
        #[arg(long)]
        file: PathBuf,
    },

    /// Alter an existing table: add, rename, or drop columns.
    AlterTable {
        db: PathBuf,
        table: String,
        /// JSON file with `{"operations":[...]}` or a JSON array.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Add a column from a JSON ColumnDesign object (repeatable).
        #[arg(long = "add-column", value_name = "JSON")]
        add_columns: Vec<String>,
        /// Rename a column as FROM=TO (repeatable).
        #[arg(long = "rename-column", value_name = "FROM=TO")]
        rename_columns: Vec<String>,
        /// Drop a column (requires --yes when applying).
        #[arg(long = "drop-column", value_name = "NAME")]
        drop_columns: Vec<String>,
        /// Print the SQL without applying it.
        #[arg(long)]
        sql_only: bool,
        /// Confirm destructive operations.
        #[arg(long)]
        yes: bool,
    },

    /// Manage indexes.
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },

    /// Manage views.
    View {
        #[command(subcommand)]
        command: ViewCommand,
    },

    /// List applied and pending migrations.
    Migrations { db: PathBuf },

    /// Apply pending .sql migrations in order.
    Migrate { db: PathBuf },

    /// Back up the database.
    Backup { db: PathBuf },

    /// List timestamped backups.
    Backups { db: PathBuf },

    /// Restore a backup. A safety backup of the current database is created first.
    Restore {
        db: PathBuf,
        /// Backup path or filename, as shown by `dbgo backups`.
        #[arg(long, conflicts_with = "latest")]
        backup: Option<String>,
        /// Restore the newest backup.
        #[arg(long, conflicts_with = "backup")]
        latest: bool,
        #[arg(long)]
        yes: bool,
    },

    /// Insert rows from a JSONL file.
    Insert {
        db: PathBuf,
        table: String,
        #[arg(long)]
        file: PathBuf,
    },

    /// Upsert rows from a JSONL file using a unique key column.
    Upsert {
        db: PathBuf,
        table: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        file: PathBuf,
    },

    /// Update rows matching a JSON filter.
    Update {
        db: PathBuf,
        table: String,
        #[arg(long = "where")]
        filter: String,
        #[arg(long = "set")]
        set: String,
        #[arg(long)]
        yes: bool,
    },

    /// Delete rows matching a JSON filter.
    Delete {
        db: PathBuf,
        table: String,
        #[arg(long = "where")]
        filter: String,
        #[arg(long)]
        yes: bool,
    },

    /// List recent write sessions from the audit log.
    Sessions { db: PathBuf },

    /// Create or refresh agent files beside the database.
    AgentGuide { db: PathBuf },

    /// List tables in a database.
    Tables { db: PathBuf },

    /// Show a table's columns.
    Schema { db: PathBuf, table: String },

    /// Count rows in a table, capped for speed.
    Count {
        db: PathBuf,
        table: String,
        #[arg(long, default_value_t = 1_000_000)]
        cap: i64,
    },

    /// Read one keyset-paginated page of a table.
    Page {
        db: PathBuf,
        table: String,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long)]
        cursor: Option<i64>,
        #[arg(long, value_enum, default_value_t = DirectionArg::Next)]
        direction: DirectionArg,
    },

    /// Run a read-only SQL query.
    Query {
        db: PathBuf,
        #[arg(help = "A single read-only SQL statement")]
        sql: String,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
}

#[derive(Subcommand)]
enum IndexCommand {
    /// Create an index on one or more columns.
    Create {
        db: PathBuf,
        table: String,
        #[arg(long)]
        name: String,
        /// Comma-separated column names.
        #[arg(long)]
        columns: String,
        #[arg(long)]
        unique: bool,
    },
    /// Drop an index.
    Drop {
        db: PathBuf,
        name: String,
        #[arg(long)]
        yes: bool,
    },
    /// List indexes, optionally for one table.
    List { db: PathBuf, table: Option<String> },
}

#[derive(Subcommand)]
enum ViewCommand {
    /// Create a view from a SELECT statement.
    Create {
        db: PathBuf,
        name: String,
        #[arg(long, conflicts_with = "file")]
        sql: Option<String>,
        #[arg(long, conflicts_with = "sql")]
        file: Option<PathBuf>,
    },
    /// Drop a view.
    Drop {
        db: PathBuf,
        name: String,
        #[arg(long)]
        yes: bool,
    },
    /// List views.
    List { db: PathBuf },
}

#[derive(Clone, Copy, ValueEnum)]
enum DirectionArg {
    Next,
    Prev,
}

impl From<DirectionArg> for Direction {
    fn from(value: DirectionArg) -> Self {
        match value {
            DirectionArg::Next => Direction::Next,
            DirectionArg::Prev => Direction::Prev,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("dbgo: {error}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Init {
            db,
            force,
            single_file,
        } => {
            if single_file {
                let (_database, backup) = Database::create(&db, force).map_err(err)?;
                if let Some(backup) = backup {
                    eprintln!("dbgo: existing database backed up to {}", backup.display());
                }
                println!("created database {}", db.display());
            } else {
                let (database, backup) = Database::create_project(&db, force).map_err(err)?;
                if let Some(backup) = backup {
                    eprintln!("dbgo: existing database backed up to {}", backup.display());
                }
                println!("created project {}", db.display());
                println!("database: {}", database.path().display());
                println!("agent guide: {}", db.join("AGENTS.md").display());
            }
            Ok(())
        }
        Command::Graph { db } => {
            let database = open(&db)?;
            let graph = database.schema_graph().map_err(err)?;
            output_graph(&graph, cli.json, cli.jsonl)
        }
        Command::CreateTable { db, file } => {
            let content = std::fs::read_to_string(&file).map_err(err)?;
            let design: TableDesign = serde_json::from_str(&content).map_err(err)?;
            let database = open(&db)?;
            let table = database.create_table(&design).map_err(err)?;
            println!("created table {}", table.name);
            Ok(())
        }
        Command::AlterTable {
            db,
            table,
            file,
            add_columns,
            rename_columns,
            drop_columns,
            sql_only,
            yes,
        } => {
            let operations = build_alter_operations(
                file.as_deref(),
                &add_columns,
                &rename_columns,
                &drop_columns,
            )?;
            if operations.is_empty() {
                return Err("no alter-table operations were provided".to_string());
            }
            if !sql_only
                && !yes
                && operations
                    .iter()
                    .any(|operation| matches!(operation, AlterTableOp::DropColumn { .. }))
            {
                return Err("refusing to drop a column without --yes".to_string());
            }

            let database = open(&db)?;
            if sql_only {
                let statements = database
                    .preview_alter_table(&table, &operations)
                    .map_err(err)?;
                print_line(&statements.join(";\n"))
            } else {
                let change = database.alter_table(&table, &operations).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
        }
        Command::Index { command } => match command {
            IndexCommand::Create {
                db,
                table,
                name,
                columns,
                unique,
            } => {
                let columns = columns
                    .split(',')
                    .map(str::trim)
                    .filter(|column| !column.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                let database = open(&db)?;
                let change = database
                    .create_index(&table, &name, &columns, unique)
                    .map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            IndexCommand::Drop { db, name, yes } => {
                if !yes {
                    return Err("refusing to drop an index without --yes".to_string());
                }
                let database = open(&db)?;
                let change = database.drop_index(&name).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            IndexCommand::List { db, table } => {
                let database = open(&db)?;
                let indexes = database.indexes(table.as_deref()).map_err(err)?;
                output_indexes(&indexes, cli.json, cli.jsonl)
            }
        },
        Command::View { command } => match command {
            ViewCommand::Create {
                db,
                name,
                sql,
                file,
            } => {
                let select_sql = match (sql, file) {
                    (Some(sql), None) => sql,
                    (None, Some(file)) => std::fs::read_to_string(&file).map_err(err)?,
                    (None, None) => {
                        return Err("view create needs --sql or --file".to_string());
                    }
                    (Some(_), Some(_)) => unreachable!("clap rejects conflicting args"),
                };
                let database = open(&db)?;
                let change = database.create_view(&name, &select_sql).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            ViewCommand::Drop { db, name, yes } => {
                if !yes {
                    return Err("refusing to drop a view without --yes".to_string());
                }
                let database = open(&db)?;
                let change = database.drop_view(&name).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            ViewCommand::List { db } => {
                let database = open(&db)?;
                let views = database.views().map_err(err)?;
                output_views(&views, cli.json, cli.jsonl)
            }
        },
        Command::Migrations { db } => {
            let database = open(&db)?;
            let migrations = database.migrations().map_err(err)?;
            output_migrations(&migrations, cli.json, cli.jsonl)
        }
        Command::Migrate { db } => {
            let database = open(&db)?;
            let result = database.apply_migrations().map_err(err)?;
            output_migrate(&result, cli.json, cli.jsonl)
        }
        Command::Backup { db } => {
            let database = open(&db)?;
            let backup = database.backup().map_err(err)?;
            println!("{}", backup.display());
            Ok(())
        }
        Command::Backups { db } => {
            let database = open(&db)?;
            let backups = database.backups().map_err(err)?;
            output_backups(&backups, cli.json, cli.jsonl)
        }
        Command::Restore {
            db,
            backup,
            latest,
            yes,
        } => {
            if !yes {
                return Err("refusing to restore without --yes".to_string());
            }
            let mut database = open(&db)?;
            let backup = if latest {
                database
                    .backups()
                    .map_err(err)?
                    .into_iter()
                    .next()
                    .map(|info| info.path)
                    .ok_or_else(|| "no backups found".to_string())?
            } else {
                backup.ok_or_else(|| "restore needs --backup <path> or --latest".to_string())?
            };
            let result = database.restore_backup(&backup).map_err(err)?;
            output_restore(&result, cli.json, cli.jsonl)
        }
        Command::Insert { db, table, file } => {
            let rows = read_jsonl(&file)?;
            let database = open(&db)?;
            let result = database.insert_rows(&table, &rows).map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Upsert {
            db,
            table,
            key,
            file,
        } => {
            let rows = read_jsonl(&file)?;
            let database = open(&db)?;
            let result = database.upsert_rows(&table, &key, &rows).map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Update {
            db,
            table,
            filter,
            set,
            yes,
        } => {
            if !yes {
                return Err("refusing to update without --yes".to_string());
            }
            let filter: FilterSpec = serde_json::from_str(&filter).map_err(err)?;
            let values: JsonObject = serde_json::from_str(&set).map_err(err)?;
            let database = open(&db)?;
            let result = database
                .update_rows(&table, &filter, &values)
                .map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Delete {
            db,
            table,
            filter,
            yes,
        } => {
            if !yes {
                return Err("refusing to delete without --yes".to_string());
            }
            let filter: FilterSpec = serde_json::from_str(&filter).map_err(err)?;
            let database = open(&db)?;
            let result = database.delete_rows(&table, &filter).map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Sessions { db } => {
            let database = open(&db)?;
            let sessions = database.sessions().map_err(err)?;
            output_sessions(&sessions, cli.json, cli.jsonl)
        }
        Command::AgentGuide { db } => {
            let mut database = open(&db)?;
            let info = database.refresh_agent_guide().map_err(err)?;
            println!(
                "{} agent files in {}",
                if info.created { "created" } else { "refreshed" },
                info.directory
            );
            println!("agent guide: {}", info.agents_path);
            println!("schema: {}", info.schema_path);
            Ok(())
        }
        Command::Tables { db } => {
            let database = open(&db)?;
            let tables = database.tables().map_err(err)?;
            output_tables(&tables, cli.json, cli.jsonl)
        }
        Command::Schema { db, table } => {
            let database = open(&db)?;
            let columns = database.columns(&table).map_err(err)?;
            output_columns(&columns, cli.json, cli.jsonl)
        }
        Command::Count { db, table, cap } => {
            let database = open(&db)?;
            let count = database.count_rows(&table, cap.max(1)).map_err(err)?;
            output_count(&count, cli.json, cli.jsonl)
        }
        Command::Page {
            db,
            table,
            limit,
            cursor,
            direction,
        } => {
            let database = open(&db)?;
            let page = database
                .page(&table, cursor, direction.into(), limit)
                .map_err(err)?;
            output_page(&page, cli.json, cli.jsonl)
        }
        Command::Query { db, sql, limit } => {
            let database = open(&db)?;
            let result = database.query(&sql, limit).map_err(err)?;
            output_query(&result, cli.json, cli.jsonl)
        }
    }
}

fn open(path: &Path) -> Result<Database, String> {
    if !path.exists() {
        return Err(format!("database or project not found: {}", path.display()));
    }
    Database::open_project(path).map_err(err)
}

fn output_tables(tables: &[TableInfo], json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_columns(columns: &[ColumnInfo], json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_graph(graph: &SchemaGraph, json: bool, jsonl: bool) -> Result<(), String> {
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

fn read_jsonl(path: &Path) -> Result<Vec<JsonObject>, String> {
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

fn build_alter_operations(
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

fn output_schema_change(change: &SchemaChange, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(change);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(change));
    }

    print_line(&format!(
        "migration {} · session {} · backup {}",
        change.migration_name,
        change.session_id,
        change.backup_path.as_deref().unwrap_or("-")
    ))?;
    if let Some(path) = &change.migration_path {
        print_line(&format!("file {path}"))?;
    }
    for statement in &change.statements {
        print_line(&format!("{};", statement.trim().trim_end_matches(';')))?;
    }
    Ok(())
}

fn output_indexes(indexes: &[IndexInfo], json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_views(views: &[ViewInfo], json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_migrations(migrations: &[MigrationInfo], json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_migrate(result: &MigrateResult, json: bool, jsonl: bool) -> Result<(), String> {
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
            "applied {} · session {} · backup {}",
            migration.name,
            migration.session_id,
            migration.backup_path.as_deref().unwrap_or("-")
        ))?;
    }
    print_line(&format!("{} migration(s) applied", result.applied.len()))
}

fn output_write(result: &WriteResult, json: bool, jsonl: bool) -> Result<(), String> {
    if json {
        return write_json(result);
    }
    if jsonl {
        return write_jsonl(std::slice::from_ref(result));
    }
    print_line(&format!(
        "{} row(s) · session {} · backup {}",
        result.affected,
        result.session_id,
        result.backup_path.as_deref().unwrap_or("-")
    ))
}

fn output_backups(backups: &[BackupInfo], json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_restore(result: &RestoreResult, json: bool, jsonl: bool) -> Result<(), String> {
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

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_age(modified_ms: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(modified_ms);
    let seconds = ((now - modified_ms).max(0) / 1000) as u64;
    match seconds {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{}m ago", seconds / 60),
        3600..=86_399 => format!("{}h ago", seconds / 3600),
        _ => format!("{}d ago", seconds / 86_400),
    }
}

fn output_sessions(sessions: &[SessionSummary], json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_count(count: &CountResult, json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_page(page: &Page, json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_query(result: &QueryResult, json: bool, jsonl: bool) -> Result<(), String> {
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

fn human_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table
}

fn print_table(table: Table) -> Result<(), String> {
    print_line(&table.to_string())
}

fn print_line(text: &str) -> Result<(), String> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{text}").map_err(err)
}

fn write_json<T: Serialize + ?Sized>(value: &T) -> Result<(), String> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value).map_err(err)?;
    writeln!(lock).map_err(err)
}

fn write_jsonl<T: Serialize>(items: &[T]) -> Result<(), String> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    for item in items {
        serde_json::to_writer(&mut lock, item).map_err(err)?;
        writeln!(lock).map_err(err)?;
    }
    Ok(())
}

fn cell_to_string(cell: &CellValue) -> String {
    match cell {
        CellValue::Null => "NULL".to_string(),
        CellValue::Integer { value } => value.to_string(),
        CellValue::Real { value } => value.to_string(),
        CellValue::Text { value } => truncate(&value.replace(['\n', '\r', '\t'], " "), 80),
        CellValue::Blob { size, .. } => format!("BLOB({size})"),
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (index, character) in value.chars().enumerate() {
        if index >= max_chars {
            out.push('…');
            break;
        }
        out.push(character);
    }
    out
}

fn yes_no(value: bool) -> String {
    if value {
        "yes".to_string()
    } else {
        String::new()
    }
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

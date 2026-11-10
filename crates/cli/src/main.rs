use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use db_go_core::{
    preview_import, AlterTableOp, BackupInfo, CellValue, ColumnDesign, ColumnInfo, CountResult,
    D1ExportResult, Database, DatabaseStats, Direction, ExecResult, ExportFormat, FilterInput,
    FilterSpec, ImportColumnMapping, ImportFormat, ImportPreview, ImportRequest, ImportResult,
    IndexInfo, JsonObject, MaintenanceResult, MigrateResult, MigrationInfo, Page, QueryPlan,
    QueryResult, RestoreResult, SchemaChange, SchemaGraph, SessionSummary, SortDirection, SortSpec,
    TableDesign, TableInfo, TriggerInfo, ViewInfo, WriteResult,
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

    /// Drop a table.
    DropTable {
        db: PathBuf,
        table: String,
        #[arg(long)]
        yes: bool,
    },

    /// Rename a table.
    RenameTable {
        db: PathBuf,
        from: String,
        to: String,
    },

    /// Manage triggers.
    Trigger {
        #[command(subcommand)]
        command: TriggerCommand,
    },

    /// Reclaim unused space with VACUUM.
    Vacuum { db: PathBuf },

    /// Update SQLite query-planner statistics with ANALYZE.
    Analyze { db: PathBuf },

    /// Read or write a PRAGMA.
    Pragma {
        db: PathBuf,
        #[arg(help = "PRAGMA name or assignment, e.g. user_version or user_version=3")]
        pragma: String,
        #[arg(long, help = "Confirm a write PRAGMA and record an audit entry")]
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

    /// Restore a backup. Take a backup first if you need to undo the restore.
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

    /// Show page, table, and index size statistics.
    Stats { db: PathBuf },

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

    /// Execute arbitrary SQL. Read-only unless --write is provided.
    Exec {
        db: PathBuf,
        #[arg(conflicts_with = "file", help = "SQL statement or block")]
        sql: Option<String>,
        #[arg(long, conflicts_with = "sql", help = "Read SQL from a file")]
        file: Option<PathBuf>,
        #[arg(long, help = "Allow writes and DDL")]
        write: bool,
        #[arg(long, help = "Confirm writes; required with --write")]
        yes: bool,
        #[arg(long, help = "Do not wrap the SQL in a transaction")]
        no_transaction: bool,
        #[arg(long, help = "Return rows (single statement, e.g. RETURNING)")]
        returning: bool,
        #[arg(long, default_value_t = 1000)]
        limit: u32,
    },

    /// Show SQLite's query plan without running the statement.
    Explain {
        db: PathBuf,
        #[arg(help = "A SQL statement to explain")]
        sql: String,
    },

    /// Export a table or query result as CSV or JSON.
    Export {
        db: PathBuf,
        #[arg(help = "Table to export")]
        table: Option<String>,
        #[arg(long, conflicts_with = "table", help = "Read-only query to export")]
        query: Option<String>,
        #[arg(long, value_enum, default_value_t = ExportFormatArg::Csv)]
        format: ExportFormatArg,
        #[arg(long, help = "Output file; stdout when omitted")]
        output: Option<PathBuf>,
        #[arg(long = "where", help = "JSON filter for a table export")]
        filter: Option<String>,
        #[arg(long, value_name = "COLUMN:ASC|DESC", help = "Sort for a table export")]
        sort: Option<String>,
    },

    /// Import CSV, JSON, or JSONL data.
    Import {
        db: PathBuf,
        #[arg(help = "Target table; defaults to the file name when creating")]
        table: Option<String>,
        #[arg(long)]
        file: PathBuf,
        #[arg(
            long,
            value_enum,
            help = "Inferred from the file extension when omitted"
        )]
        format: Option<ImportFormatArg>,
        #[arg(long, help = "Create the target table from the file")]
        create_table: bool,
        #[arg(long = "no-header", help = "CSV has no header row")]
        no_header: bool,
        #[arg(long, help = "CSV delimiter: one character or \\t")]
        delimiter: Option<String>,
        #[arg(
            long = "map",
            value_name = "SOURCE=TARGET",
            help = "Map a source column"
        )]
        mappings: Vec<String>,
        #[arg(long, help = "Upsert using this target key column")]
        upsert_key: Option<String>,
        #[arg(long, help = "Preview columns, types, and rows without writing")]
        preview: bool,
    },

    /// Export a Cloudflare D1 deployment bundle.
    D1 {
        #[command(subcommand)]
        command: D1Command,
    },
}

#[derive(Subcommand)]
enum TriggerCommand {
    /// Create a trigger from SQL.
    Create {
        db: PathBuf,
        name: String,
        #[arg(long, conflicts_with = "file")]
        sql: Option<String>,
        #[arg(long, conflicts_with = "sql")]
        file: Option<PathBuf>,
    },
    /// Drop a trigger.
    Drop {
        db: PathBuf,
        name: String,
        #[arg(long)]
        yes: bool,
    },
    /// List triggers, optionally for one table.
    List { db: PathBuf, table: Option<String> },
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

#[derive(Subcommand)]
enum D1Command {
    /// Export schema, data, wrangler.toml, and a README for Cloudflare D1.
    Export {
        db: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, help = "D1 database name; defaults to the project name")]
        name: Option<String>,
        #[arg(long, help = "Overwrite a non-empty output directory")]
        force: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum DirectionArg {
    Next,
    Prev,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExportFormatArg {
    Csv,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum ImportFormatArg {
    Csv,
    Json,
    Jsonl,
}

impl From<ImportFormatArg> for ImportFormat {
    fn from(value: ImportFormatArg) -> Self {
        match value {
            ImportFormatArg::Csv => ImportFormat::Csv,
            ImportFormatArg::Json => ImportFormat::Json,
            ImportFormatArg::Jsonl => ImportFormat::Jsonl,
        }
    }
}

impl From<ExportFormatArg> for ExportFormat {
    fn from(value: ExportFormatArg) -> Self {
        match value {
            ExportFormatArg::Csv => ExportFormat::Csv,
            ExportFormatArg::Json => ExportFormat::Json,
        }
    }
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
        Command::DropTable { db, table, yes } => {
            if !yes {
                return Err("refusing to drop a table without --yes".to_string());
            }
            let database = open(&db)?;
            let change = database.drop_table(&table).map_err(err)?;
            output_schema_change(&change, cli.json, cli.jsonl)
        }
        Command::RenameTable { db, from, to } => {
            let database = open(&db)?;
            let change = database.rename_table(&from, &to).map_err(err)?;
            output_schema_change(&change, cli.json, cli.jsonl)
        }
        Command::Trigger { command } => match command {
            TriggerCommand::Create {
                db,
                name,
                sql,
                file,
            } => {
                let sql = match (sql, file) {
                    (Some(sql), None) => sql,
                    (None, Some(file)) => std::fs::read_to_string(&file).map_err(err)?,
                    (None, None) => {
                        return Err("trigger create needs --sql or --file".to_string());
                    }
                    (Some(_), Some(_)) => unreachable!("clap rejects conflicting args"),
                };
                let database = open(&db)?;
                let change = database.create_trigger(&name, &sql).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            TriggerCommand::Drop { db, name, yes } => {
                if !yes {
                    return Err("refusing to drop a trigger without --yes".to_string());
                }
                let database = open(&db)?;
                let change = database.drop_trigger(&name).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            TriggerCommand::List { db, table } => {
                let database = open(&db)?;
                let triggers = database.triggers(table.as_deref()).map_err(err)?;
                output_triggers(&triggers, cli.json, cli.jsonl)
            }
        },
        Command::Vacuum { db } => {
            let database = open(&db)?;
            let result = database.vacuum().map_err(err)?;
            output_maintenance(&result, cli.json, cli.jsonl)
        }
        Command::Analyze { db } => {
            let database = open(&db)?;
            let result = database.analyze().map_err(err)?;
            output_maintenance(&result, cli.json, cli.jsonl)
        }
        Command::Pragma { db, pragma, yes } => {
            let body = pragma.trim().trim_end_matches(';').trim();
            let body = if body.to_ascii_lowercase().starts_with("pragma ") {
                body[7..].trim()
            } else {
                body
            };
            let write = body.contains('=') || yes;
            if write && !yes {
                return Err("refusing to write a PRAGMA without --yes".to_string());
            }
            let database = open(&db)?;
            let result = database.pragma(body, write).map_err(err)?;
            output_maintenance(&result, cli.json, cli.jsonl)
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
            let filter: FilterInput = serde_json::from_str(&filter).map_err(err)?;
            let values: JsonObject = serde_json::from_str(&set).map_err(err)?;
            let database = open(&db)?;
            let result = database
                .update_rows_expr(&table, &filter.into_expr(), &values)
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
            let filter: FilterInput = serde_json::from_str(&filter).map_err(err)?;
            let database = open(&db)?;
            let result = database
                .delete_rows_expr(&table, &filter.into_expr())
                .map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Sessions { db } => {
            let database = open(&db)?;
            let sessions = database.sessions().map_err(err)?;
            output_sessions(&sessions, cli.json, cli.jsonl)
        }
        Command::Stats { db } => {
            let database = open(&db)?;
            let stats = database.database_stats().map_err(err)?;
            output_stats(&stats, cli.json, cli.jsonl)
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
        Command::Exec {
            db,
            sql,
            file,
            write,
            yes,
            no_transaction,
            returning,
            limit,
        } => {
            if write && !yes {
                return Err("refusing to execute writes without --yes".to_string());
            }
            if returning && !write {
                return Err("--returning requires --write".to_string());
            }
            let sql = match (sql, file) {
                (Some(sql), None) => sql,
                (None, Some(file)) => std::fs::read_to_string(&file).map_err(err)?,
                (None, None) => {
                    return Err("exec needs a SQL string or --file".to_string());
                }
                (Some(_), Some(_)) => unreachable!("clap rejects conflicting args"),
            };
            let database = open(&db)?;
            let result = database
                .exec(&sql, write, !no_transaction, returning, limit)
                .map_err(|error| {
                    let message = error.to_string();
                    if !no_transaction
                        && (message.contains("VACUUM") || message.contains("within a transaction"))
                    {
                        format!("{message} (try --no-transaction)")
                    } else {
                        message
                    }
                })?;
            output_exec(&result, cli.json, cli.jsonl)
        }
        Command::Explain { db, sql } => {
            let database = open(&db)?;
            let plan = database.explain_query(&sql).map_err(err)?;
            output_plan(&plan, cli.json, cli.jsonl)
        }
        Command::Export {
            db,
            table,
            query,
            format,
            output,
            filter,
            sort,
        } => {
            if table.is_none() && query.is_none() {
                return Err("export needs a table or --query".to_string());
            }
            if query.is_some() && (filter.is_some() || sort.is_some()) {
                return Err("--where and --sort apply to table exports only".to_string());
            }
            let filter = match filter {
                Some(filter) => Some(serde_json::from_str::<FilterSpec>(&filter).map_err(err)?),
                None => None,
            };
            let sort = match sort {
                Some(sort) => {
                    let (column, direction) = sort
                        .split_once(':')
                        .or_else(|| sort.split_once('='))
                        .ok_or_else(|| format!("sort expects COLUMN:ASC|DESC: {sort}"))?;
                    let direction = match direction.trim().to_ascii_lowercase().as_str() {
                        "asc" => SortDirection::Asc,
                        "desc" => SortDirection::Desc,
                        _ => return Err(format!("invalid sort direction: {direction}")),
                    };
                    Some(SortSpec {
                        column: column.trim().to_string(),
                        direction,
                    })
                }
                None => None,
            };
            let database = open(&db)?;
            let format = format.into();
            if let Some(table) = table {
                run_export(output.as_deref(), |writer| {
                    database
                        .export_table(&table, filter.as_ref(), sort.as_ref(), format, writer)
                        .map_err(err)
                })?;
            } else if let Some(query) = query {
                run_export(output.as_deref(), |writer| {
                    database.export_query(&query, format, writer).map_err(err)
                })?;
            }
            Ok(())
        }
        Command::Import {
            db,
            table,
            file,
            format,
            create_table,
            no_header,
            delimiter,
            mappings,
            upsert_key,
            preview,
        } => {
            let format = match format {
                Some(format) => format.into(),
                None => infer_import_format(&file)?,
            };
            if preview {
                let preview =
                    preview_import(&file, format, !no_header, delimiter.as_deref()).map_err(err)?;
                return output_import_preview(&preview, cli.json, cli.jsonl);
            }

            let mapping = mappings
                .iter()
                .map(|spec| {
                    let (source, target) = spec
                        .split_once('=')
                        .map(|(source, target)| (source, Some(target)))
                        .unwrap_or((spec.as_str(), Some(spec.as_str())));
                    ImportColumnMapping {
                        source: source.trim().to_string(),
                        target: target.and_then(|target| {
                            let target = target.trim();
                            if target.is_empty() {
                                None
                            } else {
                                Some(target.to_string())
                            }
                        }),
                    }
                })
                .collect::<Vec<_>>();
            let request = ImportRequest {
                table,
                create_table,
                format,
                has_headers: !no_header,
                delimiter,
                mapping,
                upsert_key,
            };
            let database = open(&db)?;
            let result = database.import_file(&file, &request).map_err(err)?;
            output_import_result(&result, cli.json, cli.jsonl)
        }
        Command::D1 { command } => match command {
            D1Command::Export {
                db,
                output,
                name,
                force,
            } => {
                let database = open(&db)?;
                let result = database
                    .export_d1(&output, name.as_deref(), force)
                    .map_err(err)?;
                output_d1_export(&result, cli.json, cli.jsonl)
            }
        },
    }
}

fn open(path: &Path) -> Result<Database, String> {
    if !path.exists() {
        return Err(format!("database or project not found: {}", path.display()));
    }
    Database::open_project(path).map_err(err)
}

fn infer_import_format(path: &Path) -> Result<ImportFormat, String> {
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

fn output_triggers(triggers: &[TriggerInfo], json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_maintenance(result: &MaintenanceResult, json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_write(result: &WriteResult, json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_exec(result: &ExecResult, json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_plan(plan: &QueryPlan, json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_import_preview(preview: &ImportPreview, json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_import_result(result: &ImportResult, json: bool, jsonl: bool) -> Result<(), String> {
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

fn output_d1_export(result: &D1ExportResult, json: bool, jsonl: bool) -> Result<(), String> {
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

fn import_format_label(format: ImportFormat) -> &'static str {
    match format {
        ImportFormat::Csv => "CSV",
        ImportFormat::Json => "JSON",
        ImportFormat::Jsonl => "JSONL",
    }
}

fn json_value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(text) => truncate(text, 80),
        other => truncate(&other.to_string(), 80),
    }
}

fn output_stats(stats: &DatabaseStats, json: bool, jsonl: bool) -> Result<(), String> {
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

fn optional_bytes(bytes: Option<i64>) -> String {
    bytes
        .map(|value| format_bytes(value.max(0) as u64))
        .unwrap_or_else(|| "-".to_string())
}

fn run_export<F>(output: Option<&Path>, export: F) -> Result<(), String>
where
    F: FnOnce(&mut dyn Write) -> Result<usize, String>,
{
    if let Some(path) = output {
        let file = File::create(path).map_err(err)?;
        let mut writer = BufWriter::new(file);
        let rows = export(&mut writer)?;
        writer.flush().map_err(err)?;
        println!("exported {rows} row(s) to {}", path.display());
    } else {
        let stdout = io::stdout();
        let mut writer = stdout.lock();
        let rows = export(&mut writer)?;
        writer.flush().map_err(err)?;
        eprintln!("exported {rows} row(s)");
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

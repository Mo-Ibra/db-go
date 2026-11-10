//! Clap command-line definitions and argument conversion types.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use db_go_core::{Direction, ExportFormat, ImportFormat};

#[derive(Parser)]
#[command(name = "dbgo", version, about = "Local-first SQL workbench CLI.")]
pub(crate) struct Cli {
    /// Emit a single JSON document.
    #[arg(long, global = true, conflicts_with = "jsonl")]
    pub(crate) json: bool,

    /// Emit one JSON object per line.
    #[arg(long, global = true, conflicts_with = "json")]
    pub(crate) jsonl: bool,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
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
pub(crate) enum TriggerCommand {
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
pub(crate) enum IndexCommand {
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
pub(crate) enum ViewCommand {
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
pub(crate) enum D1Command {
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
pub(crate) enum DirectionArg {
    Next,
    Prev,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum ExportFormatArg {
    Csv,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum ImportFormatArg {
    Csv,
    Json,
    Jsonl,
}

impl From<ImportFormatArg> for ImportFormat {
    /// Convert the CLI import-format flag into the core import format.
    fn from(value: ImportFormatArg) -> Self {
        match value {
            ImportFormatArg::Csv => ImportFormat::Csv,
            ImportFormatArg::Json => ImportFormat::Json,
            ImportFormatArg::Jsonl => ImportFormat::Jsonl,
        }
    }
}

impl From<ExportFormatArg> for ExportFormat {
    /// Convert the CLI export-format flag into the core export format.
    fn from(value: ExportFormatArg) -> Self {
        match value {
            ExportFormatArg::Csv => ExportFormat::Csv,
            ExportFormatArg::Json => ExportFormat::Json,
        }
    }
}

impl From<DirectionArg> for Direction {
    /// Convert the CLI pagination-direction flag into the core direction.
    fn from(value: DirectionArg) -> Self {
        match value {
            DirectionArg::Next => Direction::Next,
            DirectionArg::Prev => Direction::Prev,
        }
    }
}

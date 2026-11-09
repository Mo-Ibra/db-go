use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use db_go_core::{
    CellValue, ColumnInfo, CountResult, Database, Direction, Page, QueryResult, TableInfo,
};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "dbgo",
    version,
    about = "Inspect SQLite databases from the terminal."
)]
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
        return Err(format!("database not found: {}", path.display()));
    }
    Database::open(path).map_err(err)
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

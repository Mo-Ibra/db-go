use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::result::Result;

use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use db_go_core::*;
use serde::Serialize;

use crate::util::err;

/// Format a byte count using binary units.
pub(crate) fn format_bytes(bytes: u64) -> String {
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

/// Format a timestamp as a coarse age such as '11m ago'.
pub(crate) fn format_age(modified_ms: i64) -> String {
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

/// Return the display label for an import format.
pub(crate) fn import_format_label(format: ImportFormat) -> &'static str {
    match format {
        ImportFormat::Csv => "CSV",
        ImportFormat::Json => "JSON",
        ImportFormat::Jsonl => "JSONL",
    }
}

/// Render a JSON value as compact text for human output.
pub(crate) fn json_value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(text) => truncate(text, 80),
        other => truncate(&other.to_string(), 80),
    }
}

/// Format an optional byte count, using '-' when unknown.
pub(crate) fn optional_bytes(bytes: Option<i64>) -> String {
    bytes
        .map(|value| format_bytes(value.max(0) as u64))
        .unwrap_or_else(|| "-".to_string())
}

/// Run an export callback against a file or stdout and report the row count.
pub(crate) fn run_export<F>(output: Option<&Path>, export: F) -> Result<(), String>
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

/// Create a preconfigured table for human-readable CLI output.
pub(crate) fn human_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Print a formatted table to stdout.
pub(crate) fn print_table(table: Table) -> Result<(), String> {
    print_line(&table.to_string())
}

/// Print one line to stdout.
pub(crate) fn print_line(text: &str) -> Result<(), String> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{text}").map_err(err)
}

/// Serialize one value as pretty JSON to stdout.
pub(crate) fn write_json<T: Serialize + ?Sized>(value: &T) -> Result<(), String> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value).map_err(err)?;
    writeln!(lock).map_err(err)
}

/// Serialize a slice of values as JSONL to stdout.
pub(crate) fn write_jsonl<T: Serialize>(items: &[T]) -> Result<(), String> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    for item in items {
        serde_json::to_writer(&mut lock, item).map_err(err)?;
        writeln!(lock).map_err(err)?;
    }
    Ok(())
}

/// Render a cell value for human-readable tables.
pub(crate) fn cell_to_string(cell: &CellValue) -> String {
    match cell {
        CellValue::Null => "NULL".to_string(),
        CellValue::Integer { value } => value.to_string(),
        CellValue::Real { value } => value.to_string(),
        CellValue::Text { value } => truncate(&value.replace(['\n', '\r', '\t'], " "), 80),
        CellValue::Blob { size, .. } => format!("BLOB({size})"),
    }
}

/// Truncate text to a maximum character count with an ellipsis.
pub(crate) fn truncate(value: &str, max_chars: usize) -> String {
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

/// Render a boolean as 'yes' or an empty string.
pub(crate) fn yes_no(value: bool) -> String {
    if value {
        "yes".to_string()
    } else {
        String::new()
    }
}

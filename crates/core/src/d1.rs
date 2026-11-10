//! Cloudflare D1 export helpers and file writers.

use super::*;

pub(crate) struct D1SqlObject {
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) table: String,
    pub(crate) sql: String,
}

pub(crate) const D1_DATA_ROWS_PER_FILE: usize = 500;
pub(crate) const D1_DATA_BYTES_PER_FILE: usize = 500_000;

pub(crate) struct D1DataWriter {
    output_dir: PathBuf,
    generated_at: String,
    file: Option<BufWriter<File>>,
    sequence: usize,
    rows_in_file: usize,
    bytes_in_file: usize,
    pub(crate) files: Vec<String>,
}

impl D1DataWriter {
    /// Create a D1 data-file writer for an output directory.
    pub(crate) fn new(output_dir: PathBuf, generated_at: String) -> Self {
        Self {
            output_dir,
            generated_at,
            file: None,
            sequence: 0,
            rows_in_file: 0,
            bytes_in_file: 0,
            files: Vec::new(),
        }
    }

    /// Append one SQL statement, rotating files when size or row limits are reached.
    pub(crate) fn write(&mut self, statement: &str) -> Result<()> {
        if self.file.is_none()
            || self.rows_in_file >= D1_DATA_ROWS_PER_FILE
            || (self.bytes_in_file + statement.len() + 1 > D1_DATA_BYTES_PER_FILE
                && self.rows_in_file > 0)
        {
            self.rotate()?;
        }
        let file = self.file.as_mut().expect("data file is open");
        writeln!(file, "{statement}")?;
        self.rows_in_file += 1;
        self.bytes_in_file += statement.len() + 1;
        Ok(())
    }

    /// Close the current data file and open the next numbered one.
    pub(crate) fn rotate(&mut self) -> Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        self.sequence += 1;
        let name = format!("data-{:04}.sql", self.sequence);
        let path = self.output_dir.join(&name);
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "-- DBGo D1 data export — {}", self.generated_at)?;
        writeln!(writer, "-- File {} of the export", self.sequence)?;
        writeln!(writer)?;
        self.file = Some(writer);
        self.rows_in_file = 0;
        self.bytes_in_file = 0;
        self.files.push(name);
        Ok(())
    }

    /// Flush and close the current data file.
    pub(crate) fn finish(&mut self) -> Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        Ok(())
    }
}

/// Order tables so parents are exported before foreign-key children.
pub(crate) fn order_d1_tables(
    mut tables: Vec<String>,
    database: &Database,
    skipped: &HashSet<String>,
    warnings: &mut Vec<String>,
) -> Result<Vec<String>> {
    tables.sort();
    let by_lower = tables
        .iter()
        .map(|table| (table.to_ascii_lowercase(), table.clone()))
        .collect::<HashMap<_, _>>();
    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for table in &tables {
        let mut set = HashSet::new();
        for foreign_key in database.foreign_keys(table)? {
            let referenced = foreign_key.referenced_table.to_ascii_lowercase();
            if referenced != table.to_ascii_lowercase()
                && !skipped.contains(&referenced)
                && by_lower.contains_key(&referenced)
            {
                set.insert(referenced);
            }
        }
        deps.insert(table.to_ascii_lowercase(), set);
    }

    let mut remaining = by_lower.keys().cloned().collect::<HashSet<_>>();
    let mut ordered = Vec::with_capacity(tables.len());
    while !remaining.is_empty() {
        let mut ready = remaining
            .iter()
            .filter(|table| {
                deps.get(*table)
                    .map(|deps| deps.iter().all(|dep| !remaining.contains(dep)))
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();
        if ready.is_empty() {
            warnings.push(
                "foreign key cycle detected; remaining tables use alphabetical order".to_string(),
            );
            let mut rest = remaining.iter().cloned().collect::<Vec<_>>();
            rest.sort();
            for table in rest {
                remaining.remove(&table);
                if let Some(name) = by_lower.get(&table) {
                    ordered.push(name.clone());
                }
            }
            break;
        }
        ready.sort();
        for table in ready {
            remaining.remove(&table);
            if let Some(name) = by_lower.get(&table) {
                ordered.push(name.clone());
            }
        }
    }
    Ok(ordered)
}

/// Choose the D1 database name from an explicit value or project config.
pub(crate) fn d1_database_name(database: &Database, provided: Option<&str>) -> String {
    let raw = provided
        .map(ToOwned::to_owned)
        .or_else(|| {
            database.project_dir().and_then(|project_dir| {
                read_project_config(project_dir)
                    .ok()
                    .map(|config| config.name)
            })
        })
        .or_else(|| {
            database
                .path()
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "dbgo-db".to_string());
    sanitize_d1_name(&raw)
}

/// Sanitize a name into a valid D1 database name.
pub(crate) fn sanitize_d1_name(value: &str) -> String {
    let mut name = value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while name.contains("--") {
        name = name.replace("--", "-");
    }
    let name = name.trim_matches('-').to_string();
    if name.is_empty() {
        "dbgo-db".to_string()
    } else {
        name.chars().take(63).collect()
    }
}

/// Render the wrangler.toml configuration for a D1 export.
pub(crate) fn d1_wrangler_toml(name: &str) -> String {
    format!(
        "# Generated by DBGo\nname = \"{name}-worker\"\ncompatibility_date = \"2024-11-01\"\n\n[[d1_databases]]\nbinding = \"DB\"\ndatabase_name = \"{name}\"\ndatabase_id = \"<YOUR_D1_DATABASE_ID>\"\n"
    )
}

/// Render the README for a D1 export bundle.
pub(crate) fn d1_readme(
    name: &str,
    source: &Path,
    generated_at: &str,
    data_files: &[String],
    tables: &[D1TableExport],
    warnings: &[String],
) -> String {
    let mut output = format!(
        "# D1 export\n\nGenerated by DBGo from `{}` at {}.\n\n",
        source.display(),
        generated_at
    );
    output.push_str("## Files\n\n- `schema.sql`\n");
    for file in data_files {
        output.push_str(&format!("- `{file}`\n"));
    }
    output.push_str("\n## Deploy\n\n```bash\n");
    output.push_str(&format!("wrangler d1 create {name}\n"));
    output.push_str("# Put the returned database_id into wrangler.toml.\n");
    output.push_str(&format!(
        "wrangler d1 execute {name} --remote --file schema.sql\n"
    ));
    for file in data_files {
        output.push_str(&format!(
            "wrangler d1 execute {name} --remote --file {file}\n"
        ));
    }
    output.push_str("```\n\n## Tables\n\n| Table | Rows |\n|---|---:|\n");
    for table in tables {
        output.push_str(&format!("| `{}` | {} |\n", table.name, table.rows));
    }
    if data_files.is_empty() {
        output.push_str("\nNo data rows were exported.\n");
    }
    if !warnings.is_empty() {
        output.push_str("\n## Warnings\n\n");
        for warning in warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output.push_str("\nDBGo internal tables (`_dbgo_*`) are excluded.\n");
    output
}

/// Render a SQLite value as a SQL literal for D1 import files.
pub(crate) fn d1_sql_literal(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(integer) => integer.to_string(),
        ValueRef::Real(real) if real.is_finite() => real.to_string(),
        ValueRef::Real(_) => "NULL".to_string(),
        ValueRef::Text(text) => {
            if text.contains(&0) {
                format!("CAST(X'{}' AS TEXT)", hex_encode(text))
            } else {
                format!("'{}'", String::from_utf8_lossy(text).replace('\'', "''"))
            }
        }
        ValueRef::Blob(blob) => format!("X'{}'", hex_encode(blob)),
    }
}

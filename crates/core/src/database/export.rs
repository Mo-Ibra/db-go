//! CSV/JSON exports and Cloudflare D1 bundle generation.

use crate::*;

impl Database {
    /// Export a read-only query result to CSV or JSON.
    pub fn export_query(
        &self,
        sql: &str,
        format: ExportFormat,
        writer: &mut dyn Write,
    ) -> Result<usize> {
        let sql = sql.trim().trim_end_matches(';');
        if sql.is_empty() {
            return Err(DbError::InvalidDesign(
                "a query is required for export".to_string(),
            ));
        }
        let mut statement = self.conn.prepare(sql)?;
        if !statement.readonly() {
            return Err(DbError::ReadOnly);
        }
        self.export_statement(&mut statement, &[], format, writer)
    }

    /// Export a table, optionally filtered and sorted, to CSV or JSON.
    pub fn export_table(
        &self,
        table: &str,
        filter: Option<&FilterSpec>,
        sort: Option<&SortSpec>,
        format: ExportFormat,
        writer: &mut dyn Write,
    ) -> Result<usize> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        let (where_sql, params) = match filter {
            Some(filter) => filter_clause(filter, &columns)?,
            None => (String::new(), Vec::new()),
        };
        let order_sql = if let Some(sort) = sort {
            if !columns.iter().any(|column| column.name == sort.column) {
                return Err(DbError::UnknownColumn(sort.column.clone()));
            }
            let direction = match sort.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            format!(" ORDER BY {} {direction}", quote_ident(&sort.column))
        } else {
            String::new()
        };
        let sql = format!("SELECT * FROM {}{where_sql}{order_sql}", quote_ident(table));
        let mut statement = self.conn.prepare(&sql)?;
        self.export_statement(&mut statement, &params, format, writer)
    }

    /// Build and run the SELECT used for a table export.
    pub(crate) fn export_statement(
        &self,
        statement: &mut rusqlite::Statement<'_>,
        params: &[rusqlite::types::Value],
        format: ExportFormat,
        writer: &mut dyn Write,
    ) -> Result<usize> {
        let columns = statement
            .column_names()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        let mut rows = statement.query(rusqlite::params_from_iter(params.iter()))?;

        match format {
            ExportFormat::Csv => {
                writeln!(
                    writer,
                    "{}",
                    columns
                        .iter()
                        .map(|column| csv_escape(column))
                        .collect::<Vec<_>>()
                        .join(",")
                )?;
                let mut count = 0;
                while let Some(row) = rows.next()? {
                    let mut fields = Vec::with_capacity(columns.len());
                    for index in 0..columns.len() {
                        fields.push(csv_value(row.get_ref(index)?));
                    }
                    writeln!(
                        writer,
                        "{}",
                        fields
                            .iter()
                            .map(|field| csv_escape(field))
                            .collect::<Vec<_>>()
                            .join(",")
                    )?;
                    count += 1;
                }
                Ok(count)
            }
            ExportFormat::Json => {
                write!(writer, "[")?;
                let mut first = true;
                let mut count = 0;
                while let Some(row) = rows.next()? {
                    if !first {
                        write!(writer, ",")?;
                    }
                    first = false;
                    let mut object = serde_json::Map::new();
                    for (index, name) in columns.iter().enumerate() {
                        object.insert(name.clone(), export_json(row.get_ref(index)?));
                    }
                    serde_json::to_writer(&mut *writer, &object)?;
                    count += 1;
                }
                writeln!(writer, "]")?;
                Ok(count)
            }
        }
    }

    /// Export schema, data, wrangler.toml, manifest, and README for Cloudflare D1.
    pub fn export_d1(
        &self,
        output_dir: &Path,
        database_name: Option<&str>,
        force: bool,
    ) -> Result<D1ExportResult> {
        if output_dir.exists() {
            if !output_dir.is_dir() {
                return Err(DbError::InvalidDesign(format!(
                    "export path must be a directory: {}",
                    output_dir.display()
                )));
            }
            if !force && std::fs::read_dir(output_dir)?.next().is_some() {
                return Err(DbError::InvalidDesign(format!(
                    "output directory is not empty: {} (use --force to overwrite)",
                    output_dir.display()
                )));
            }
        } else {
            std::fs::create_dir_all(output_dir)?;
        }

        let generated_at: String = self
            .conn
            .query_row("SELECT datetime('now')", [], |row| row.get(0))?;
        let name = d1_database_name(self, database_name);
        let mut warnings = Vec::new();

        let objects = self.d1_schema_objects()?;
        let virtual_tables = objects
            .iter()
            .filter(|object| {
                object.kind == "table"
                    && object
                        .sql
                        .trim_start()
                        .to_ascii_uppercase()
                        .starts_with("CREATE VIRTUAL TABLE")
            })
            .map(|object| object.name.clone())
            .collect::<Vec<_>>();
        if !virtual_tables.is_empty() {
            warnings.push(format!(
                "skipped virtual table(s): {}",
                virtual_tables.join(", ")
            ));
        }

        let virtual_lower = virtual_tables
            .iter()
            .map(|table| table.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        let mut skipped = virtual_lower.clone();
        for table in self.tables()? {
            let lower = table.name.to_ascii_lowercase();
            if virtual_lower
                .iter()
                .any(|virtual_name| lower.starts_with(&format!("{virtual_name}_")))
            {
                skipped.insert(lower);
                warnings.push(format!("skipped shadow table: {}", table.name));
            }
        }

        let mut table_names = self
            .tables()?
            .into_iter()
            .map(|table| table.name)
            .filter(|table| !skipped.contains(&table.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        table_names = order_d1_tables(table_names, self, &skipped, &mut warnings)?;

        let mut schema = format!(
            "-- D1 export generated by DBGo\n-- source: {}\n-- generated: {generated_at}\n\n",
            self.path.display()
        );
        let mut written_objects = 0;
        for table in &table_names {
            if let Some(object) = objects
                .iter()
                .find(|object| object.kind == "table" && object.name.eq_ignore_ascii_case(table))
            {
                schema.push_str(object.sql.trim_end_matches(';'));
                schema.push_str(";\n\n");
                written_objects += 1;
            }
        }
        for kind in ["index", "view", "trigger"] {
            for object in objects.iter().filter(|object| object.kind == kind) {
                if skipped.contains(&object.table.to_ascii_lowercase()) {
                    warnings.push(format!(
                        "skipped {} {} on skipped table {}",
                        object.kind, object.name, object.table
                    ));
                    continue;
                }
                schema.push_str(object.sql.trim_end_matches(';'));
                schema.push_str(";\n\n");
                written_objects += 1;
            }
        }
        if written_objects == 0 {
            warnings.push("no exportable schema objects found".to_string());
        }

        let schema_path = output_dir.join("schema.sql");
        std::fs::write(&schema_path, schema)?;

        let mut data_writer = D1DataWriter::new(output_dir.to_path_buf(), generated_at.clone());
        let mut tables = Vec::new();
        let mut total_rows = 0;
        for table in &table_names {
            let columns = self.d1_export_columns(table)?;
            if columns.is_empty() {
                warnings.push(format!("skipped table without exportable columns: {table}"));
                continue;
            }
            let column_sql = columns
                .iter()
                .map(|column| quote_ident(column))
                .collect::<Vec<_>>()
                .join(", ");
            let select_sql = format!("SELECT {column_sql} FROM {}", quote_ident(table));
            let mut statement = self.conn.prepare(&select_sql)?;
            let mut rows = statement.query([])?;
            let mut table_rows = 0;
            while let Some(row) = rows.next()? {
                let mut values = Vec::with_capacity(columns.len());
                for index in 0..columns.len() {
                    values.push(d1_sql_literal(row.get_ref(index)?));
                }
                let statement = format!(
                    "INSERT INTO {} ({column_sql}) VALUES ({});",
                    quote_ident(table),
                    values.join(", ")
                );
                data_writer.write(&statement)?;
                table_rows += 1;
                total_rows += 1;
            }
            tables.push(D1TableExport {
                name: table.clone(),
                rows: table_rows,
            });
        }
        data_writer.finish()?;
        let data_files = data_writer.files.clone();

        let wrangler_path = output_dir.join("wrangler.toml");
        std::fs::write(&wrangler_path, d1_wrangler_toml(&name))?;

        let readme_path = output_dir.join("README.md");
        std::fs::write(
            &readme_path,
            d1_readme(
                &name,
                &self.path,
                &generated_at,
                &data_files,
                &tables,
                &warnings,
            ),
        )?;

        let manifest_path = output_dir.join("manifest.json");
        let manifest = serde_json::json!({
            "generated_by": "DBGo",
            "generated_at": generated_at,
            "source": self.path.display().to_string(),
            "database_name": name,
            "schema_file": "schema.sql",
            "data_files": &data_files,
            "tables": tables.iter().map(|table| serde_json::json!({
                "name": table.name,
                "rows": table.rows,
            })).collect::<Vec<_>>(),
            "total_rows": total_rows,
            "warnings": &warnings,
        });
        std::fs::write(
            &manifest_path,
            format!("{}\n", serde_json::to_string_pretty(&manifest)?),
        )?;

        Ok(D1ExportResult {
            output_dir: output_dir.display().to_string(),
            schema_file: schema_path.display().to_string(),
            data_files,
            manifest_file: manifest_path.display().to_string(),
            wrangler_file: wrangler_path.display().to_string(),
            readme_file: readme_path.display().to_string(),
            tables,
            rows: total_rows,
            warnings,
        })
    }

    /// Collect tables, indexes, views, and triggers for the D1 schema file.
    pub(crate) fn d1_schema_objects(&self) -> Result<Vec<D1SqlObject>> {
        let mut statement = self.conn.prepare(
            "SELECT type, name, tbl_name, sql
             FROM sqlite_master
             WHERE sql IS NOT NULL
               AND name NOT LIKE 'sqlite_%'
               AND substr(name, 1, 6) <> '_dbgo_'
               AND substr(tbl_name, 1, 6) <> '_dbgo_'
             ORDER BY CASE type
                        WHEN 'table' THEN 0
                        WHEN 'index' THEN 1
                        WHEN 'view' THEN 2
                        WHEN 'trigger' THEN 3
                        ELSE 4
                      END,
                      name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(D1SqlObject {
                kind: row.get(0)?,
                name: row.get(1)?,
                table: row.get(2)?,
                sql: row.get(3)?,
            })
        })?;
        let mut objects = Vec::new();
        for row in rows {
            objects.push(row?);
        }
        Ok(objects)
    }

    /// Return the ordered column list used to export one D1 table.
    pub(crate) fn d1_export_columns(&self, table: &str) -> Result<Vec<String>> {
        let pragma = format!("PRAGMA table_xinfo({})", quote_ident(table));
        let mut statement = self.conn.prepare(&pragma)?;
        let mut rows = statement.query([])?;
        let mut columns = Vec::new();
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            let hidden: i64 = row.get(6)?;
            if hidden == 0 {
                columns.push(name);
            }
        }
        Ok(columns)
    }
}

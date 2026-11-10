//! File imports into new or existing tables.

use crate::*;

impl Database {
    /// Import CSV, JSON, or JSONL rows into a new or existing table with mapping and optional upsert.
    pub fn import_file(&self, path: &Path, request: &ImportRequest) -> Result<ImportResult> {
        let (source_columns, source_rows) = read_import_source(
            path,
            request.format,
            request.has_headers,
            request.delimiter.as_deref(),
        )?;
        if source_rows.is_empty() {
            return Err(DbError::InvalidDesign(
                "the import file has no data rows".to_string(),
            ));
        }
        let inferred = infer_import_columns(&source_columns, &source_rows, request.format);

        let (table_name, mapping, created_table) = if request.create_table {
            if request.upsert_key.is_some() {
                return Err(DbError::InvalidDesign(
                    "create-table imports cannot use an upsert key".to_string(),
                ));
            }
            let raw_table = request
                .table
                .clone()
                .unwrap_or_else(|| suggest_table_name(path));
            let table_name = sanitize_import_identifier(&raw_table, "imported_table");
            if self.table_exists(&table_name)? {
                return Err(DbError::InvalidDesign(format!(
                    "table already exists: {table_name}"
                )));
            }

            let requested = request
                .mapping
                .iter()
                .map(|mapping| (mapping.source.as_str(), mapping.target.as_deref()))
                .collect::<HashMap<_, _>>();
            let mut used = HashSet::new();
            let mut pairs = Vec::new();
            let mut design_columns = Vec::new();
            for (index, source) in source_columns.iter().enumerate() {
                let target = match requested.get(source.as_str()) {
                    Some(None) => continue,
                    Some(Some(target)) => *target,
                    None => inferred[index].suggested_name.as_str(),
                };
                let target = unique_import_identifier(target, &mut used, index + 1);
                design_columns.push(ColumnDesign {
                    name: target.clone(),
                    type_name: inferred[index].inferred_type.clone(),
                    primary_key: false,
                    not_null: false,
                    unique: false,
                    default_value: None,
                    references: None,
                });
                pairs.push((source.clone(), target));
            }
            if pairs.is_empty() {
                return Err(DbError::InvalidDesign(
                    "no source columns are mapped".to_string(),
                ));
            }

            let design = TableDesign {
                name: table_name.clone(),
                columns: design_columns,
                index_foreign_keys: false,
            };
            self.create_table(&design)?;
            (table_name, pairs, true)
        } else {
            let table_name = request
                .table
                .clone()
                .ok_or_else(|| DbError::InvalidDesign("a target table is required".to_string()))?;
            self.ensure_table(&table_name)?;
            let target_columns = self.columns(&table_name)?;
            let mapping =
                resolve_import_mapping(&source_columns, &target_columns, &request.mapping)?;
            (table_name, mapping, false)
        };

        let target_columns = self.columns(&table_name)?;
        let target_index = target_columns
            .iter()
            .map(|column| (column.name.to_ascii_lowercase(), column.clone()))
            .collect::<HashMap<_, _>>();
        let target_names = mapping
            .iter()
            .map(|(_, target)| target.clone())
            .collect::<Vec<_>>();

        let upsert_key = match request.upsert_key.as_deref() {
            Some(key) => target_names
                .iter()
                .find(|target| target.eq_ignore_ascii_case(key))
                .cloned()
                .ok_or_else(|| DbError::UnknownColumn(key.to_string()))
                .map(Some)?,
            None => None,
        };

        let mut converted_rows = Vec::with_capacity(source_rows.len());
        for (row_index, source_row) in source_rows.iter().enumerate() {
            let mut target_row = JsonObject::new();
            for (source, target) in &mapping {
                let column = target_index
                    .get(&target.to_ascii_lowercase())
                    .ok_or_else(|| DbError::UnknownColumn(target.clone()))?;
                let value = source_row
                    .get(source)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let value = if request.format == ImportFormat::Csv {
                    match value {
                        serde_json::Value::String(raw) => {
                            convert_csv_import_value(&raw, column, row_index + 1)?
                        }
                        serde_json::Value::Null => serde_json::Value::Null,
                        other => other,
                    }
                } else {
                    value
                };
                target_row.insert(target.clone(), value);
            }
            converted_rows.push(target_row);
        }

        if let Some(key) = &upsert_key {
            for row in &converted_rows {
                if row.get(key).is_none() || row.get(key) == Some(&serde_json::Value::Null) {
                    return Err(DbError::InvalidDesign(format!(
                        "import row is missing a non-null upsert key: {key}"
                    )));
                }
            }
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let quoted = quote_ident(&table_name);
        let column_sql = target_names
            .iter()
            .map(|name| quote_ident(name))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = vec!["?"; target_names.len()].join(", ");
        let insert_prefix = format!("INSERT INTO {quoted} ({column_sql}) VALUES ({placeholders})");
        let sql = if let Some(key) = &upsert_key {
            let updates = target_names
                .iter()
                .filter(|name| !name.eq_ignore_ascii_case(key))
                .map(|name| format!("{} = excluded.{}", quote_ident(name), quote_ident(name)))
                .collect::<Vec<_>>()
                .join(", ");
            if updates.is_empty() {
                format!(
                    "{insert_prefix} ON CONFLICT({}) DO NOTHING",
                    quote_ident(key)
                )
            } else {
                format!(
                    "{insert_prefix} ON CONFLICT({}) DO UPDATE SET {updates}",
                    quote_ident(key)
                )
            }
        } else {
            insert_prefix
        };

        let transaction = self.conn.unchecked_transaction()?;
        let mut affected = 0;
        for row in &converted_rows {
            let params = target_names
                .iter()
                .map(|name| {
                    json_to_sql(
                        row.get(name)
                            .expect("every mapped target column is present"),
                    )
                })
                .collect::<Vec<_>>();

            let (rowid, before) = if let Some(key) = &upsert_key {
                let key_value = row.get(key).expect("upsert key is present");
                let where_sql = format!(" WHERE {} = ?", quote_ident(key));
                let before = select_rows_json(
                    &transaction,
                    &quoted,
                    &where_sql,
                    vec![json_to_sql(key_value)],
                    &target_columns,
                )?
                .into_iter()
                .next();
                transaction.execute(&sql, rusqlite::params_from_iter(params))?;
                let rowid = match &before {
                    Some((rowid, _)) => *rowid,
                    None => transaction.last_insert_rowid(),
                };
                (rowid, before.map(|(_, value)| value))
            } else {
                transaction.execute(&sql, rusqlite::params_from_iter(params))?;
                (transaction.last_insert_rowid(), None)
            };

            let after = fetch_row_json(&transaction, &table_name, &target_columns, rowid)?;
            record_audit(
                &transaction,
                &session_id,
                &table_name,
                Some(rowid),
                if before.is_some() { "update" } else { "insert" },
                before.as_ref(),
                after.as_ref(),
            )?;
            affected += 1;
        }
        transaction.commit()?;

        let mapped_sources = mapping
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<HashSet<_>>();
        let ignored_columns = source_columns
            .into_iter()
            .filter(|source| !mapped_sources.contains(source))
            .collect();

        Ok(ImportResult {
            table: table_name,
            rows: affected,
            session_id,
            created_table,
            ignored_columns,
        })
    }
}

//! Audited row inserts, upserts, updates, deletes, and GUI batch saves.

use crate::*;

impl Database {
    /// List recent write sessions and their action counts from the audit log.
    pub fn sessions(&self) -> Result<Vec<SessionSummary>> {
        self.ensure_audit_table()?;
        let mut statement = self.conn.prepare(
            "SELECT session_id, COUNT(*), MIN(created_at)
             FROM _dbgo_audit
             GROUP BY session_id
             ORDER BY MIN(created_at) DESC
             LIMIT 100",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(SessionSummary {
                session_id: row.get(0)?,
                actions: row.get(1)?,
                started_at: row.get(2)?,
            })
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    /// Create the internal _dbgo_audit table if it does not exist.
    pub(crate) fn ensure_audit_table(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _dbgo_audit (
                id INTEGER PRIMARY KEY,
                session_id TEXT NOT NULL,
                table_name TEXT NOT NULL,
                rowid INTEGER,
                action TEXT NOT NULL,
                before_json TEXT,
                after_json TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_dbgo_audit_session ON _dbgo_audit(session_id);",
        )?;
        Ok(())
    }

    /// Insert JSON rows into a table in one audited transaction.
    pub fn insert_rows(&self, table: &str, rows: &[JsonObject]) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        if rows.is_empty() {
            return Ok(WriteResult {
                session_id: new_session_id(),
                affected: 0,
                backup_path: None,
            });
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let quoted = quote_ident(table);
        let transaction = self.conn.unchecked_transaction()?;
        let mut affected = 0;

        for row in rows {
            validate_json_columns(row, &columns)?;
            let keys: Vec<&String> = row.keys().collect();
            if keys.is_empty() {
                return Err(DbError::InvalidDesign(
                    "insert row has no columns".to_string(),
                ));
            }

            let column_sql = keys
                .iter()
                .map(|key| quote_ident(key.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = vec!["?"; keys.len()].join(", ");
            let sql = format!("INSERT INTO {quoted} ({column_sql}) VALUES ({placeholders})");
            let params = keys
                .iter()
                .map(|key| json_to_sql(row.get(key.as_str()).expect("row key exists")))
                .collect::<Vec<_>>();

            transaction.execute(&sql, rusqlite::params_from_iter(params))?;
            let rowid = transaction.last_insert_rowid();
            let after = fetch_row_json(&transaction, table, &columns, rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(rowid),
                "insert",
                None,
                after.as_ref(),
            )?;
            affected += 1;
        }

        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    /// Insert or update JSON rows using a unique key column in one audited transaction.
    pub fn upsert_rows(&self, table: &str, key: &str, rows: &[JsonObject]) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        if !columns.iter().any(|column| column.name == key) {
            return Err(DbError::UnknownColumn(key.to_string()));
        }
        if rows.is_empty() {
            return Ok(WriteResult {
                session_id: new_session_id(),
                affected: 0,
                backup_path: None,
            });
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let quoted = quote_ident(table);
        let key_ident = quote_ident(key);
        let transaction = self.conn.unchecked_transaction()?;
        let mut affected = 0;

        for row in rows {
            validate_json_columns(row, &columns)?;
            if !row.contains_key(key) {
                return Err(DbError::InvalidDesign(format!(
                    "upsert row is missing key column: {key}"
                )));
            }

            let before = {
                let sql = format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} WHERE {key_ident} = ?1 LIMIT 1"
                );
                let mut statement = transaction.prepare(&sql)?;
                let mut found =
                    statement.query([json_to_sql(row.get(key).expect("key exists"))])?;
                if let Some(found_row) = found.next()? {
                    let rowid = found_row.get::<_, i64>(0)?;
                    Some((rowid, row_to_json(&columns, found_row)?))
                } else {
                    None
                }
            };

            let keys: Vec<&String> = row.keys().collect();
            let column_sql = keys
                .iter()
                .map(|column| quote_ident(column.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = vec!["?"; keys.len()].join(", ");

            let mut update_columns: Vec<&String> = Vec::new();
            for column in &keys {
                if column.as_str() != key {
                    update_columns.push(*column);
                }
            }

            let conflict = if update_columns.is_empty() {
                format!("ON CONFLICT({key_ident}) DO NOTHING")
            } else {
                let updates = update_columns
                    .iter()
                    .map(|column| {
                        format!(
                            "{} = excluded.{}",
                            quote_ident(column.as_str()),
                            quote_ident(column.as_str())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("ON CONFLICT({key_ident}) DO UPDATE SET {updates}")
            };

            let sql =
                format!("INSERT INTO {quoted} ({column_sql}) VALUES ({placeholders}) {conflict}");
            let params = keys
                .iter()
                .map(|column| json_to_sql(row.get(column.as_str()).expect("row key exists")))
                .collect::<Vec<_>>();
            transaction.execute(&sql, rusqlite::params_from_iter(params))?;

            let (rowid, action) = if let Some((rowid, _)) = &before {
                (*rowid, "update")
            } else {
                (transaction.last_insert_rowid(), "insert")
            };
            let after = fetch_row_json(&transaction, table, &columns, rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(rowid),
                action,
                before.as_ref().map(|(_, value)| value),
                after.as_ref(),
            )?;
            affected += 1;
        }

        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    /// Update rows matching a legacy single-condition filter.
    pub fn update_rows(
        &self,
        table: &str,
        filter: &FilterSpec,
        values: &JsonObject,
    ) -> Result<WriteResult> {
        self.update_rows_expr(table, &FilterExpr::from(filter.clone()), values)
    }

    /// Update rows matching an AND/OR/NOT filter tree.
    pub fn update_rows_expr(
        &self,
        table: &str,
        filter: &FilterExpr,
        values: &JsonObject,
    ) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        if values.is_empty() {
            return Err(DbError::InvalidDesign("no values to update".to_string()));
        }

        let mut set_parts = Vec::new();
        let mut set_params = Vec::new();
        for (name, value) in values {
            validate_json_column(name, &columns)?;
            if columns
                .iter()
                .any(|column| column.name == *name && column.primary_key)
            {
                return Err(DbError::InvalidDesign(
                    "updating primary key columns is not supported".to_string(),
                ));
            }
            set_parts.push(format!("{} = ?", quote_ident(name)));
            set_params.push(json_to_sql(value));
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let (condition, where_params) = filter_expr_clause(filter, &columns)?;
        let where_sql = format!(" WHERE {condition}");
        let quoted = quote_ident(table);
        let transaction = self.conn.unchecked_transaction()?;
        let matching = select_rows_json(&transaction, &quoted, &where_sql, where_params, &columns)?;

        let set_sql = set_parts.join(", ");
        for (rowid, before) in &matching {
            let mut params = set_params.clone();
            params.push(rusqlite::types::Value::Integer(*rowid));
            transaction.execute(
                &format!("UPDATE {quoted} SET {set_sql} WHERE rowid = ?"),
                rusqlite::params_from_iter(params),
            )?;
            let after = fetch_row_json(&transaction, table, &columns, *rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(*rowid),
                "update",
                Some(before),
                after.as_ref(),
            )?;
        }

        let affected = matching.len();
        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    /// Delete rows matching a legacy single-condition filter.
    pub fn delete_rows(&self, table: &str, filter: &FilterSpec) -> Result<WriteResult> {
        self.delete_rows_expr(table, &FilterExpr::from(filter.clone()))
    }

    /// Delete rows matching an AND/OR/NOT filter tree.
    pub fn delete_rows_expr(&self, table: &str, filter: &FilterExpr) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let (condition, where_params) = filter_expr_clause(filter, &columns)?;
        let where_sql = format!(" WHERE {condition}");
        let quoted = quote_ident(table);
        let transaction = self.conn.unchecked_transaction()?;
        let matching = select_rows_json(&transaction, &quoted, &where_sql, where_params, &columns)?;

        for (rowid, before) in &matching {
            transaction.execute(&format!("DELETE FROM {quoted} WHERE rowid = ?1"), [*rowid])?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(*rowid),
                "delete",
                Some(before),
                None,
            )?;
        }

        let affected = matching.len();
        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }

    /// Apply a batch of GUI row inserts, updates, and deletes in one audited transaction.
    pub fn save_changes(
        &self,
        table: &str,
        updates: &[RowUpdate],
        inserts: &[JsonObject],
        deletes: &[i64],
    ) -> Result<WriteResult> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        if updates.is_empty() && inserts.is_empty() && deletes.is_empty() {
            return Ok(WriteResult {
                session_id: new_session_id(),
                affected: 0,
                backup_path: None,
            });
        }

        for update in updates {
            validate_update_values(&update.values, &columns)?;
        }
        for insert in inserts {
            validate_json_columns(insert, &columns)?;
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let quoted = quote_ident(table);
        let transaction = self.conn.unchecked_transaction()?;
        let mut affected = 0;

        // Deletes run first so a replacement row can reuse a key freed in the
        // same save. Updates come next to free unique values before inserts.
        for rowid in deletes {
            let before = fetch_row_json(&transaction, table, &columns, *rowid)?
                .ok_or_else(|| DbError::InvalidDesign(format!("row not found: {rowid}")))?;
            transaction.execute(&format!("DELETE FROM {quoted} WHERE rowid = ?1"), [*rowid])?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(*rowid),
                "delete",
                Some(&before),
                None,
            )?;
            affected += 1;
        }

        for update in updates {
            let before =
                fetch_row_json(&transaction, table, &columns, update.rowid)?.ok_or_else(|| {
                    DbError::InvalidDesign(format!("row not found: {}", update.rowid))
                })?;
            if update.values.is_empty() {
                continue;
            }

            let mut set_parts = Vec::new();
            let mut params = Vec::new();
            for (name, value) in &update.values {
                set_parts.push(format!("{} = ?", quote_ident(name)));
                params.push(json_to_sql(value));
            }
            params.push(rusqlite::types::Value::Integer(update.rowid));

            transaction.execute(
                &format!(
                    "UPDATE {quoted} SET {} WHERE rowid = ?",
                    set_parts.join(", ")
                ),
                rusqlite::params_from_iter(params),
            )?;
            let after = fetch_row_json(&transaction, table, &columns, update.rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(update.rowid),
                "update",
                Some(&before),
                after.as_ref(),
            )?;
            affected += 1;
        }

        for row in inserts {
            let keys: Vec<&String> = row.keys().collect();
            if keys.is_empty() {
                return Err(DbError::InvalidDesign(
                    "insert row has no columns".to_string(),
                ));
            }
            let column_sql = keys
                .iter()
                .map(|key| quote_ident(key.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = vec!["?"; keys.len()].join(", ");
            let sql = format!("INSERT INTO {quoted} ({column_sql}) VALUES ({placeholders})");
            let params = keys
                .iter()
                .map(|key| json_to_sql(row.get(key.as_str()).expect("row key exists")))
                .collect::<Vec<_>>();

            transaction.execute(&sql, rusqlite::params_from_iter(params))?;
            let rowid = transaction.last_insert_rowid();
            let after = fetch_row_json(&transaction, table, &columns, rowid)?;
            record_audit(
                &transaction,
                &session_id,
                table,
                Some(rowid),
                "insert",
                None,
                after.as_ref(),
            )?;
            affected += 1;
        }

        transaction.commit()?;
        Ok(WriteResult {
            session_id,
            affected,
            backup_path: None,
        })
    }
}

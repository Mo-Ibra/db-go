//! Arbitrary SQL execution, triggers, and database maintenance.

use crate::*;

impl Database {
    /// Execute arbitrary SQL with read-only safety and optional transaction/audit handling.
    pub fn exec(
        &self,
        sql: &str,
        allow_write: bool,
        transactional: bool,
        returning: bool,
        limit: u32,
    ) -> Result<ExecResult> {
        let sql = sql.trim();
        if sql.is_empty() {
            return Err(DbError::InvalidDesign(
                "a SQL statement is required".to_string(),
            ));
        }
        let limit = limit.clamp(1, 10_000) as usize;

        if !allow_write {
            let result = self.query(sql, limit as u32)?;
            return Ok(ExecResult {
                sql: sql.to_string(),
                columns: result.columns,
                rows: result.rows,
                truncated: result.truncated,
                affected: 0,
                session_id: None,
                transactional: false,
                read_only: true,
            });
        }

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let after = serde_json::json!({
            "sql": sql,
            "returning": returning,
        });

        if returning {
            let (columns, rows, truncated, affected) = if transactional {
                let transaction = self.conn.unchecked_transaction()?;
                let result = exec_returning(&transaction, sql, limit)?;
                record_audit(
                    &transaction,
                    &session_id,
                    "sql",
                    None,
                    "exec",
                    None,
                    Some(&after),
                )?;
                transaction.commit()?;
                result
            } else {
                let result = exec_returning(&self.conn, sql, limit)?;
                record_audit(
                    &self.conn,
                    &session_id,
                    "sql",
                    None,
                    "exec",
                    None,
                    Some(&after),
                )?;
                result
            };
            return Ok(ExecResult {
                sql: sql.to_string(),
                columns,
                rows,
                truncated,
                affected,
                session_id: Some(session_id),
                transactional,
                read_only: false,
            });
        }

        let affected = if transactional {
            let transaction = self.conn.unchecked_transaction()?;
            let before = self.conn.total_changes();
            transaction.execute_batch(sql)?;
            let affected = self.conn.total_changes().saturating_sub(before) as usize;
            record_audit(
                &transaction,
                &session_id,
                "sql",
                None,
                "exec",
                None,
                Some(&after),
            )?;
            transaction.commit()?;
            affected
        } else {
            let before = self.conn.total_changes();
            self.conn.execute_batch(sql)?;
            let affected = self.conn.total_changes().saturating_sub(before) as usize;
            record_audit(
                &self.conn,
                &session_id,
                "sql",
                None,
                "exec",
                None,
                Some(&after),
            )?;
            affected
        };

        Ok(ExecResult {
            sql: sql.to_string(),
            columns: Vec::new(),
            rows: Vec::new(),
            truncated: false,
            affected,
            session_id: Some(session_id),
            transactional,
            read_only: false,
        })
    }

    /// Drop a table and record the schema change.
    pub fn drop_table(&self, table: &str) -> Result<SchemaChange> {
        self.ensure_table(table)?;
        if table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "internal tables cannot be dropped".to_string(),
            ));
        }
        let before = self.table_ddl(table)?;
        self.run_schema_change(
            &format!("drop_table_{table}"),
            vec![format!("DROP TABLE {}", quote_ident(table))],
            "drop_table",
            table,
            Some(serde_json::json!({ "ddl": before })),
            None,
        )
    }

    /// Rename a table and record the schema change.
    pub fn rename_table(&self, from: &str, to: &str) -> Result<SchemaChange> {
        self.ensure_table(from)?;
        validate_ident(to)?;
        if to.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "names starting with _dbgo_ are reserved".to_string(),
            ));
        }
        if self.any_object_exists(to)? {
            return Err(DbError::InvalidDesign(format!(
                "object already exists: {to}"
            )));
        }
        let before = self.table_ddl(from)?;
        self.run_schema_change(
            &format!("rename_table_{from}_to_{to}"),
            vec![format!(
                "ALTER TABLE {} RENAME TO {}",
                quote_ident(from),
                quote_ident(to)
            )],
            "rename_table",
            from,
            Some(serde_json::json!({ "ddl": before })),
            Some(serde_json::json!({ "new_name": to })),
        )
    }

    /// List triggers, optionally limited to one table.
    pub fn triggers(&self, table: Option<&str>) -> Result<Vec<TriggerInfo>> {
        let mut sql = "SELECT name, tbl_name, sql
                       FROM sqlite_master
                       WHERE type = 'trigger'
                         AND sql IS NOT NULL
                         AND name NOT LIKE 'sqlite_%'
                         AND substr(name, 1, 6) <> '_dbgo_'"
            .to_string();
        if table.is_some() {
            sql.push_str(" AND tbl_name = ?1 COLLATE NOCASE");
        }
        sql.push_str(" ORDER BY name");
        let mut statement = self.conn.prepare(&sql)?;
        let rows = if let Some(table) = table {
            statement
                .query_map([table], |row| {
                    Ok(TriggerInfo {
                        name: row.get(0)?,
                        table: row.get(1)?,
                        sql: row.get(2)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            statement
                .query_map([], |row| {
                    Ok(TriggerInfo {
                        name: row.get(0)?,
                        table: row.get(1)?,
                        sql: row.get(2)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    /// Create a trigger from SQL and record the schema change.
    pub fn create_trigger(&self, name: &str, sql: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(format!(
                "reserved trigger name: {name}"
            )));
        }
        let exists: i64 = self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'trigger' AND name = ?1 COLLATE NOCASE
             )",
            [name],
            |row| row.get(0),
        )?;
        if exists != 0 {
            return Err(DbError::InvalidDesign(format!(
                "trigger already exists: {name}"
            )));
        }
        let sql = sql.trim().trim_end_matches(';').trim();
        if !sql.to_ascii_uppercase().starts_with("CREATE TRIGGER") {
            return Err(DbError::InvalidDesign(
                "trigger SQL must start with CREATE TRIGGER".to_string(),
            ));
        }
        self.run_schema_change(
            &format!("trigger_{name}"),
            vec![sql.to_string()],
            "create_trigger",
            name,
            None,
            Some(serde_json::json!({ "ddl": sql })),
        )
    }

    /// Drop a trigger and record the schema change.
    pub fn drop_trigger(&self, name: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(format!(
                "reserved trigger name: {name}"
            )));
        }
        let ddl = self
            .conn
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'trigger' AND name = ?1 COLLATE NOCASE",
                [name],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .ok_or_else(|| DbError::InvalidDesign(format!("trigger not found: {name}")))?;
        self.run_schema_change(
            &format!("drop_trigger_{name}"),
            vec![format!("DROP TRIGGER {}", quote_ident(name))],
            "drop_trigger",
            name,
            Some(serde_json::json!({ "ddl": ddl })),
            None,
        )
    }

    /// Run VACUUM and record a maintenance audit entry.
    pub fn vacuum(&self) -> Result<MaintenanceResult> {
        self.conn.execute_batch("VACUUM;")?;
        self.record_maintenance("vacuum", serde_json::json!({}), Vec::new(), Vec::new())
    }

    /// Run ANALYZE and record a maintenance audit entry.
    pub fn analyze(&self) -> Result<MaintenanceResult> {
        self.conn.execute_batch("ANALYZE;")?;
        self.record_maintenance("analyze", serde_json::json!({}), Vec::new(), Vec::new())
    }

    /// Read or write a SQLite PRAGMA, optionally auditing the write.
    pub fn pragma(&self, pragma: &str, write: bool) -> Result<MaintenanceResult> {
        let body = pragma.trim().trim_end_matches(';').trim();
        let body = if body.to_ascii_lowercase().starts_with("pragma ") {
            body[7..].trim()
        } else {
            body
        };
        if body.is_empty() {
            return Err(DbError::InvalidDesign(
                "a PRAGMA name is required".to_string(),
            ));
        }
        let sql = format!("PRAGMA {body}");
        let mut statement = self.conn.prepare(&sql)?;
        let columns = statement
            .columns()
            .map(|column| ColumnInfo {
                name: column.name().to_string(),
                type_name: column.decl_type().unwrap_or("").to_string(),
                not_null: false,
                primary_key: false,
            })
            .collect::<Vec<_>>();
        let mut rows = statement.query([])?;
        let mut output = Vec::new();
        while let Some(row) = rows.next()? {
            let mut cells = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                cells.push(cell_from_value_ref(row.get_ref(index)?));
            }
            output.push(Row { rowid: None, cells });
        }
        if write {
            self.record_maintenance(
                "pragma",
                serde_json::json!({ "pragma": body }),
                columns,
                output,
            )
        } else {
            Ok(MaintenanceResult {
                action: "pragma".to_string(),
                session_id: None,
                columns,
                rows: output,
            })
        }
    }

    /// Insert an audit entry for a non-schema maintenance action.
    pub(crate) fn record_maintenance(
        &self,
        action: &str,
        details: serde_json::Value,
        columns: Vec<ColumnInfo>,
        rows: Vec<Row>,
    ) -> Result<MaintenanceResult> {
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        record_audit(
            &self.conn,
            &session_id,
            "maintenance",
            None,
            action,
            None,
            Some(&details),
        )?;
        Ok(MaintenanceResult {
            action: action.to_string(),
            session_id: Some(session_id),
            columns,
            rows,
        })
    }
}

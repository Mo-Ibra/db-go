//! Migration discovery, application, and schema-change bookkeeping.

use crate::*;

impl Database {
    /// List applied and pending migration files in application order.
    pub fn migrations(&self) -> Result<Vec<MigrationInfo>> {
        self.ensure_migrations_table()?;
        let applied = {
            let mut statement = self
                .conn
                .prepare("SELECT name, sql, applied_at FROM _dbgo_migrations ORDER BY id")?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };

        let mut used = vec![false; applied.len()];
        let mut out = Vec::new();
        if let Some(project_dir) = &self.project_dir {
            let directory = project_dir.join("migrations");
            if directory.is_dir() {
                let mut files = std::fs::read_dir(&directory)?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("sql"))
                    })
                    .collect::<Vec<_>>();
                files.sort();

                for path in files {
                    let name = path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or_default()
                        .to_string();
                    let content = std::fs::read_to_string(&path)?;
                    let matched = applied.iter().position(|(applied_name, applied_sql, _)| {
                        applied_name == &name || applied_sql.trim() == content.trim()
                    });
                    let (is_applied, applied_at) = match matched {
                        Some(index) => {
                            used[index] = true;
                            (true, Some(applied[index].2.clone()))
                        }
                        None => (false, None),
                    };
                    out.push(MigrationInfo {
                        name,
                        applied: is_applied,
                        applied_at,
                        path: Some(path.display().to_string()),
                    });
                }
            }
        }

        for (index, (name, _, applied_at)) in applied.into_iter().enumerate() {
            if !used[index] {
                out.push(MigrationInfo {
                    name,
                    applied: true,
                    applied_at: Some(applied_at),
                    path: None,
                });
            }
        }
        Ok(out)
    }

    /// Apply all pending SQL migration files in filename order.
    pub fn apply_migrations(&self) -> Result<MigrateResult> {
        let pending = self
            .migrations()?
            .into_iter()
            .filter(|migration| !migration.applied)
            .filter_map(|migration| migration.path.map(|path| (migration.name, path)))
            .collect::<Vec<_>>();

        let mut applied = Vec::new();
        for (name, path) in pending {
            let sql = std::fs::read_to_string(&path)?;
            if sql.trim().is_empty() {
                return Err(DbError::InvalidDesign(format!(
                    "migration is empty: {path}"
                )));
            }
            applied.push(self.apply_migration_sql(&name, &sql)?);
        }
        Ok(MigrateResult { applied })
    }

    /// Apply one migration SQL file in a transaction and record it.
    pub(crate) fn apply_migration_sql(&self, name: &str, sql: &str) -> Result<AppliedMigration> {
        self.ensure_migrations_table()?;
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO _dbgo_migrations (name, sql) VALUES (?1, ?2)",
            rusqlite::params![name, sql],
        )?;
        record_audit(
            &transaction,
            &session_id,
            "migrations",
            None,
            "migration",
            None,
            Some(&serde_json::json!({ "name": name })),
        )?;
        transaction.commit()?;

        if let Err(error) = self.write_schema_md() {
            eprintln!("[dbgo] warning: could not update schema.md: {error}");
        }

        Ok(AppliedMigration {
            name: name.to_string(),
            session_id,
            backup_path: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    /// Run a schema change transaction, record a migration, and refresh schema.md.
    pub(crate) fn run_schema_change(
        &self,
        slug: &str,
        statements: Vec<String>,
        action: &str,
        object_name: &str,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
    ) -> Result<SchemaChange> {
        if statements.is_empty() {
            return Err(DbError::InvalidDesign(
                "no schema statements to apply".to_string(),
            ));
        }

        self.ensure_migrations_table()?;
        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let sql = format!(
            "{};",
            statements
                .iter()
                .map(|statement| statement.trim().trim_end_matches(';'))
                .collect::<Vec<_>>()
                .join(";\n")
        );

        let transaction = self.conn.unchecked_transaction()?;
        let migration_id: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM _dbgo_migrations",
            [],
            |row| row.get(0),
        )?;
        let migration_name = format!("{migration_id:04}_{}", sanitize_slug(slug));
        transaction.execute_batch(&sql)?;
        transaction.execute(
            "INSERT INTO _dbgo_migrations (id, name, sql) VALUES (?1, ?2, ?3)",
            rusqlite::params![migration_id, migration_name, sql],
        )?;
        record_audit(
            &transaction,
            &session_id,
            object_name,
            None,
            action,
            before.as_ref(),
            after.as_ref(),
        )?;
        transaction.commit()?;

        let migration_path = if let Some(project_dir) = &self.project_dir {
            match write_migration_named(project_dir, &migration_name, action, object_name, &sql) {
                Ok(path) => Some(path.display().to_string()),
                Err(error) => {
                    eprintln!("[dbgo] warning: could not write migration file: {error}");
                    None
                }
            }
        } else {
            None
        };

        if let Err(error) = self.write_schema_md() {
            eprintln!("[dbgo] warning: could not update schema.md: {error}");
        }

        Ok(SchemaChange {
            session_id,
            backup_path: None,
            migration_id,
            migration_name,
            migration_path,
            statements,
        })
    }

    /// Look up a SQLite object's SQL and table name by kind and name.
    pub(crate) fn object_record(
        &self,
        kind: &str,
        name: &str,
    ) -> Result<Option<(String, Option<String>)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT tbl_name, sql FROM sqlite_master
                 WHERE type = ?1 AND name = ?2 COLLATE NOCASE",
                [kind, name],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?)
    }

    /// Return true when any table, index, view, or trigger already uses a name.
    pub(crate) fn any_object_exists(&self, name: &str) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE name = ?1 COLLATE NOCASE
                  AND type IN ('table', 'view', 'index', 'trigger')
             )",
            [name],
            |row| row.get::<_, i64>(0),
        )? != 0)
    }

    /// Validate a foreign-key reference against an existing table and column.
    pub(crate) fn validate_foreign_key(
        &self,
        design: &TableDesign,
        column: &ColumnDesign,
        references: &ForeignKeyDesign,
    ) -> Result<()> {
        validate_ident(&references.table)?;
        validate_ident(&references.column)?;
        if references.table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "cannot reference internal tables".to_string(),
            ));
        }

        if references.table.eq_ignore_ascii_case(design.name.trim()) {
            if !design
                .columns
                .iter()
                .any(|candidate| candidate.name.eq_ignore_ascii_case(&references.column))
            {
                return Err(DbError::InvalidDesign(format!(
                    "{} references missing column {}.{}",
                    column.name, references.table, references.column
                )));
            }
        } else {
            if !self.table_exists(&references.table)? {
                return Err(DbError::InvalidDesign(format!(
                    "referenced table does not exist: {}",
                    references.table
                )));
            }
            let target_columns = self.columns(&references.table)?;
            if !target_columns
                .iter()
                .any(|candidate| candidate.name.eq_ignore_ascii_case(&references.column))
            {
                return Err(DbError::InvalidDesign(format!(
                    "{} references missing column {}.{}",
                    column.name, references.table, references.column
                )));
            }
        }

        let on_delete = normalize_on_delete(references.on_delete.as_deref())?;
        if on_delete == "SET NULL" && column.not_null {
            return Err(DbError::InvalidDesign(format!(
                "{} is NOT NULL, so ON DELETE SET NULL is not allowed",
                column.name
            )));
        }
        Ok(())
    }

    /// Return true when a table with the given name exists.
    pub(crate) fn table_exists(&self, table: &str) -> Result<bool> {
        self.conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master
                    WHERE type = 'table' AND name = ?1 COLLATE NOCASE
                 )",
                [table],
                |row| row.get(0),
            )
            .map_err(DbError::from)
    }

    /// Return an error unless the named table exists.
    pub(crate) fn ensure_table(&self, table: &str) -> Result<()> {
        if self.table_exists(table)? {
            Ok(())
        } else {
            Err(DbError::TableNotFound(table.to_string()))
        }
    }
}

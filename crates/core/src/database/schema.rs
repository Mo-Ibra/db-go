//! Tables, columns, indexes, views, and schema alteration.

use crate::*;

impl Database {
    /// Create a table from a design and record the schema change.
    pub fn create_table(&self, design: &TableDesign) -> Result<TableInfo> {
        let table_name = design.name.trim();
        validate_ident(table_name)?;
        if table_name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "names starting with _dbgo_ are reserved".to_string(),
            ));
        }
        if design.columns.is_empty() {
            return Err(DbError::InvalidDesign(
                "a table needs at least one column".to_string(),
            ));
        }
        if self.table_exists(table_name)? {
            return Err(DbError::InvalidDesign(format!(
                "table already exists: {table_name}"
            )));
        }

        let mut seen = HashSet::new();
        let mut primary_keys = 0;
        for column in &design.columns {
            validate_ident(&column.name)?;
            if !seen.insert(column.name.to_ascii_lowercase()) {
                return Err(DbError::InvalidDesign(format!(
                    "duplicate column: {}",
                    column.name
                )));
            }
            normalize_type(&column.type_name)?;
            if column.primary_key {
                primary_keys += 1;
            }
            if let Some(references) = &column.references {
                self.validate_foreign_key(design, column, references)?;
            }
        }
        if primary_keys > 1 {
            return Err(DbError::InvalidDesign(
                "only one primary key column is supported".to_string(),
            ));
        }

        let create_sql = render_create_table(design)?;
        let mut statements = vec![create_sql];
        if design.index_foreign_keys {
            for column in &design.columns {
                if column.references.is_some() {
                    statements.push(render_index_for_column(table_name, &column.name));
                }
            }
        }
        let migration_sql = format!("{};", statements.join(";\n"));

        self.run_schema_change(
            &format!("create_table_{table_name}"),
            statements,
            "create_table",
            table_name,
            None,
            Some(serde_json::json!({ "ddl": migration_sql })),
        )?;

        Ok(TableInfo {
            name: table_name.to_string(),
            kind: "table".to_string(),
        })
    }

    /// Return the original CREATE TABLE SQL for a table.
    pub fn table_ddl(&self, table: &str) -> Result<String> {
        self.ensure_table(table)?;
        Ok(self
            .conn
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'table' AND name = ?1 COLLATE NOCASE",
                [table],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .unwrap_or_default())
    }

    /// List foreign keys declared on a table.
    pub fn foreign_keys(&self, table: &str) -> Result<Vec<ForeignKeyInfo>> {
        self.ensure_table(table)?;
        let pragma = format!("PRAGMA foreign_key_list({})", quote_ident(table));
        let mut statement = self.conn.prepare(&pragma)?;
        let mut rows = statement.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let referenced_table: String = row.get(2)?;
            let column: String = row.get(3)?;
            let referenced_column: Option<String> = row.get(4)?;
            let referenced_column = match referenced_column {
                Some(column) if !column.is_empty() => column,
                _ => self
                    .primary_key_column(&referenced_table)
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
            };
            out.push(ForeignKeyInfo {
                column,
                referenced_table,
                referenced_column,
                on_update: row.get(5)?,
                on_delete: row.get(6)?,
            });
        }
        Ok(out)
    }

    /// List indexes, optionally limited to one table.
    pub fn indexes(&self, table: Option<&str>) -> Result<Vec<IndexInfo>> {
        if let Some(table) = table {
            return self.indexes_for_table(table);
        }

        let mut statement = self.conn.prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND substr(name, 1, 6) <> '_dbgo_'
             ORDER BY name",
        )?;
        let names = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for name in names {
            out.extend(self.indexes_for_table(&name)?);
        }
        Ok(out)
    }

    /// List indexes for one table with their column metadata.
    pub(crate) fn indexes_for_table(&self, table: &str) -> Result<Vec<IndexInfo>> {
        self.ensure_table(table)?;
        let pragma = format!("PRAGMA index_list({})", quote_ident(table));
        let mut statement = self.conn.prepare(&pragma)?;
        let mut rows = statement.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            let unique: bool = row.get::<_, i64>(2)? != 0;
            let origin: String = row.get(3)?;
            let partial: bool = row.get::<_, i64>(4)? != 0;
            let sql: Option<String> = self
                .conn
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1 COLLATE NOCASE",
                    [&name],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();

            let info_pragma = format!("PRAGMA index_info({})", quote_ident(&name));
            let mut info_statement = self.conn.prepare(&info_pragma)?;
            let columns = info_statement
                .query_map([], |row| {
                    Ok(row
                        .get::<_, Option<String>>(2)?
                        .unwrap_or_else(|| "(expression)".to_string()))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            out.push(IndexInfo {
                name,
                table: table.to_string(),
                unique,
                origin,
                partial,
                columns,
                sql,
            });
        }
        Ok(out)
    }

    /// Return detailed metadata for one table: DDL, columns, indexes, and foreign keys.
    pub fn table_details(&self, table: &str) -> Result<TableDetails> {
        Ok(TableDetails {
            name: table.to_string(),
            kind: "table".to_string(),
            ddl: self.table_ddl(table)?,
            columns: self.columns(table)?,
            indexes: self.indexes(Some(table))?,
            foreign_keys: self.foreign_keys(table)?,
        })
    }

    /// Plan an ALTER TABLE operation, including a table rebuild when SQLite requires it.
    pub(crate) fn plan_alter_table(
        &self,
        table: &str,
        operations: &[AlterTableOp],
    ) -> Result<(Vec<String>, Vec<ColumnInfo>)> {
        self.ensure_table(table)?;
        if table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "internal tables cannot be altered".to_string(),
            ));
        }
        if operations.is_empty() {
            return Err(DbError::InvalidDesign(
                "no alter-table operations were provided".to_string(),
            ));
        }

        let mut columns = self.columns(table)?;
        let mut statements = Vec::new();

        for operation in operations {
            match operation {
                AlterTableOp::AddColumn { column } => {
                    validate_ident(&column.name)?;
                    let type_name = normalize_type(&column.type_name)?;
                    if columns
                        .iter()
                        .any(|existing| existing.name.eq_ignore_ascii_case(&column.name))
                    {
                        return Err(DbError::InvalidDesign(format!(
                            "column already exists: {}",
                            column.name
                        )));
                    }
                    if column.primary_key {
                        return Err(DbError::InvalidDesign(
                            "cannot add a PRIMARY KEY column to an existing table".to_string(),
                        ));
                    }
                    if column.unique {
                        return Err(DbError::InvalidDesign(
                            "cannot add a UNIQUE column; create a unique index instead".to_string(),
                        ));
                    }
                    if column.not_null && column.default_value.is_none() {
                        return Err(DbError::InvalidDesign(format!(
                            "{} is NOT NULL, so it needs a default value",
                            column.name
                        )));
                    }
                    if let Some(references) = &column.references {
                        if column.not_null {
                            return Err(DbError::InvalidDesign(format!(
                                "{} has a foreign key, so it cannot be NOT NULL when added",
                                column.name
                            )));
                        }
                        self.validate_alter_reference(table, column, references, &columns)?;
                        let default_is_null = match column.default_value.as_deref() {
                            None => true,
                            Some(value) => value.trim().eq_ignore_ascii_case("NULL"),
                        };
                        if !default_is_null {
                            return Err(DbError::InvalidDesign(format!(
                                "{} has a foreign key, so its default must be NULL",
                                column.name
                            )));
                        }
                    }

                    statements.push(format!(
                        "ALTER TABLE {} ADD COLUMN {}",
                        quote_ident(table),
                        render_column_definition(column)?
                    ));
                    columns.push(ColumnInfo {
                        name: column.name.clone(),
                        type_name,
                        not_null: column.not_null,
                        primary_key: false,
                    });
                }
                AlterTableOp::RenameColumn { from, to } => {
                    validate_ident(from)?;
                    validate_ident(to)?;
                    if from.eq_ignore_ascii_case(to) {
                        return Err(DbError::InvalidDesign(
                            "rename source and target are the same".to_string(),
                        ));
                    }
                    if !columns
                        .iter()
                        .any(|column| column.name.eq_ignore_ascii_case(from))
                    {
                        return Err(DbError::UnknownColumn(from.clone()));
                    }
                    if columns
                        .iter()
                        .any(|column| column.name.eq_ignore_ascii_case(to))
                    {
                        return Err(DbError::InvalidDesign(format!(
                            "column already exists: {to}"
                        )));
                    }
                    statements.push(format!(
                        "ALTER TABLE {} RENAME COLUMN {} TO {}",
                        quote_ident(table),
                        quote_ident(from),
                        quote_ident(to)
                    ));
                    if let Some(column) = columns
                        .iter_mut()
                        .find(|column| column.name.eq_ignore_ascii_case(from))
                    {
                        column.name = to.clone();
                    }
                }
                AlterTableOp::DropColumn { name } => {
                    validate_ident(name)?;
                    let Some(column) = columns
                        .iter()
                        .find(|column| column.name.eq_ignore_ascii_case(name))
                    else {
                        return Err(DbError::UnknownColumn(name.clone()));
                    };
                    if column.primary_key {
                        return Err(DbError::InvalidDesign(
                            "cannot drop a primary key column".to_string(),
                        ));
                    }
                    if columns.len() <= 1 {
                        return Err(DbError::InvalidDesign(
                            "cannot drop the last column in a table".to_string(),
                        ));
                    }
                    if let Some(index) = self.indexes_for_table(table)?.into_iter().find(|index| {
                        index
                            .columns
                            .iter()
                            .any(|column| column.eq_ignore_ascii_case(name))
                    }) {
                        return Err(DbError::InvalidDesign(format!(
                            "column {name} is used by index {}; drop the index first",
                            index.name
                        )));
                    }
                    statements.push(format!(
                        "ALTER TABLE {} DROP COLUMN {}",
                        quote_ident(table),
                        quote_ident(name)
                    ));
                    columns.retain(|column| !column.name.eq_ignore_ascii_case(name));
                }
            }
        }

        Ok((statements, columns))
    }

    /// Return the SQL that alter_table would execute without applying it.
    pub fn preview_alter_table(
        &self,
        table: &str,
        operations: &[AlterTableOp],
    ) -> Result<Vec<String>> {
        Ok(self.plan_alter_table(table, operations)?.0)
    }

    /// Apply add, rename, or drop column operations and record the schema change.
    pub fn alter_table(&self, table: &str, operations: &[AlterTableOp]) -> Result<SchemaChange> {
        let before_ddl = self.table_ddl(table)?;
        let (statements, _) = self.plan_alter_table(table, operations)?;
        self.run_schema_change(
            &format!("alter_{table}"),
            statements,
            "alter_table",
            table,
            Some(serde_json::json!({ "ddl": before_ddl })),
            Some(serde_json::json!({ "operations": operations })),
        )
    }

    /// Validate that altering a table will not break foreign-key references.
    pub(crate) fn validate_alter_reference(
        &self,
        table: &str,
        column: &ColumnDesign,
        references: &ForeignKeyDesign,
        columns: &[ColumnInfo],
    ) -> Result<()> {
        validate_ident(&references.table)?;
        validate_ident(&references.column)?;
        if references.table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "cannot reference internal tables".to_string(),
            ));
        }

        let target_columns = if references.table.eq_ignore_ascii_case(table) {
            columns.to_vec()
        } else {
            if !self.table_exists(&references.table)? {
                return Err(DbError::InvalidDesign(format!(
                    "referenced table does not exist: {}",
                    references.table
                )));
            }
            self.columns(&references.table)?
        };
        if !target_columns
            .iter()
            .any(|target| target.name.eq_ignore_ascii_case(&references.column))
        {
            return Err(DbError::InvalidDesign(format!(
                "{} references missing column {}.{}",
                column.name, references.table, references.column
            )));
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

    /// Create an index and record the schema change.
    pub fn create_index(
        &self,
        table: &str,
        name: &str,
        columns: &[String],
        unique: bool,
    ) -> Result<SchemaChange> {
        self.ensure_table(table)?;
        if table.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(
                "internal tables cannot be indexed".to_string(),
            ));
        }
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_")
            || name.to_ascii_lowercase().starts_with("sqlite_")
        {
            return Err(DbError::InvalidDesign(format!(
                "reserved index name: {name}"
            )));
        }
        if columns.is_empty() {
            return Err(DbError::InvalidDesign(
                "an index needs at least one column".to_string(),
            ));
        }
        let table_columns = self.columns(table)?;
        for column in columns {
            validate_ident(column)?;
            if !table_columns
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(column))
            {
                return Err(DbError::UnknownColumn(column.clone()));
            }
        }
        if self.any_object_exists(name)? {
            return Err(DbError::InvalidDesign(format!(
                "object already exists: {name}"
            )));
        }

        let column_sql = columns
            .iter()
            .map(|column| quote_ident(column))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "CREATE {}INDEX {} ON {} ({column_sql})",
            if unique { "UNIQUE " } else { "" },
            quote_ident(name),
            quote_ident(table)
        );
        self.run_schema_change(
            &format!("index_{name}"),
            vec![sql.clone()],
            "create_index",
            table,
            None,
            Some(serde_json::json!({
                "index": name,
                "table": table,
                "unique": unique,
                "columns": columns,
                "ddl": sql,
            })),
        )
    }

    /// Drop an index and record the schema change.
    pub fn drop_index(&self, name: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_")
            || name.to_ascii_lowercase().starts_with("sqlite_")
        {
            return Err(DbError::InvalidDesign(format!(
                "reserved index name: {name}"
            )));
        }
        let Some((table, Some(sql))) = self.object_record("index", name)? else {
            return Err(DbError::InvalidDesign(format!(
                "index not found or automatic: {name}"
            )));
        };
        self.run_schema_change(
            &format!("drop_index_{name}"),
            vec![format!("DROP INDEX {}", quote_ident(name))],
            "drop_index",
            &table,
            Some(serde_json::json!({ "index": name, "ddl": sql })),
            None,
        )
    }

    /// List user views and their SQL.
    pub fn views(&self) -> Result<Vec<ViewInfo>> {
        let mut statement = self.conn.prepare(
            "SELECT name, sql FROM sqlite_master
             WHERE type = 'view'
               AND substr(name, 1, 6) <> '_dbgo_'
             ORDER BY name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ViewInfo {
                name: row.get(0)?,
                sql: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Create a view from a SELECT statement and record the schema change.
    pub fn create_view(&self, name: &str, select_sql: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(format!(
                "reserved view name: {name}"
            )));
        }
        if self.any_object_exists(name)? {
            return Err(DbError::InvalidDesign(format!(
                "object already exists: {name}"
            )));
        }
        let select_sql = select_sql.trim().trim_end_matches(';');
        if select_sql.is_empty() {
            return Err(DbError::InvalidDesign(
                "a view needs a SELECT statement".to_string(),
            ));
        }
        {
            let statement = self.conn.prepare(select_sql)?;
            if !statement.readonly() {
                return Err(DbError::ReadOnly);
            }
        }

        let sql = format!("CREATE VIEW {} AS {}", quote_ident(name), select_sql);
        self.run_schema_change(
            &format!("view_{name}"),
            vec![sql.clone()],
            "create_view",
            name,
            None,
            Some(serde_json::json!({ "view": name, "ddl": sql })),
        )
    }

    /// Drop a view and record the schema change.
    pub fn drop_view(&self, name: &str) -> Result<SchemaChange> {
        validate_ident(name)?;
        if name.to_ascii_lowercase().starts_with("_dbgo_") {
            return Err(DbError::InvalidDesign(format!(
                "reserved view name: {name}"
            )));
        }
        let Some((_, Some(sql))) = self.object_record("view", name)? else {
            return Err(DbError::InvalidDesign(format!("view not found: {name}")));
        };
        self.run_schema_change(
            &format!("drop_view_{name}"),
            vec![format!("DROP VIEW {}", quote_ident(name))],
            "drop_view",
            name,
            Some(serde_json::json!({ "view": name, "ddl": sql })),
            None,
        )
    }
}

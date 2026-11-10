//! Schema inspection, pagination, queries, query plans, and statistics.

use crate::*;

impl Database {
    /// List user tables in the database.
    pub fn tables(&self) -> Result<Vec<TableInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, type
             FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND substr(name, 1, 6) <> '_dbgo_'
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TableInfo {
                name: row.get(0)?,
                kind: row.get(1)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Return a compact table-to-column-names map used for editor autocomplete.
    pub fn schema_map(&self) -> Result<BTreeMap<String, Vec<String>>> {
        let mut map = BTreeMap::new();
        for table in self.tables()? {
            let columns = self.columns(&table.name)?;
            map.insert(
                table.name,
                columns.into_iter().map(|column| column.name).collect(),
            );
        }
        Ok(map)
    }

    /// Build the full schema graph: tables, columns, foreign keys, and relationships.
    pub fn schema_graph(&self) -> Result<SchemaGraph> {
        let tables = self.tables()?;
        let mut nodes = Vec::with_capacity(tables.len());
        let mut relationships = Vec::new();

        for table in &tables {
            let columns = self.columns(&table.name)?;
            let mut foreign_key_columns = HashSet::new();
            let pragma = format!("PRAGMA foreign_key_list({})", quote_ident(&table.name));
            let mut statement = self.conn.prepare(&pragma)?;
            let mut rows = statement.query([])?;

            while let Some(row) = rows.next()? {
                let to_table: String = row.get(2)?;
                let from_column: String = row.get(3)?;
                let to_column: Option<String> = row.get(4)?;
                let on_update: String = row.get(5)?;
                let on_delete: String = row.get(6)?;
                let to_column = match to_column {
                    Some(column) if !column.is_empty() => column,
                    _ => self
                        .primary_key_column(&to_table)
                        .ok()
                        .flatten()
                        .unwrap_or_default(),
                };
                if to_column.is_empty() {
                    continue;
                }

                foreign_key_columns.insert(from_column.clone());
                relationships.push(Relationship {
                    id: format!("{}:{}->{}:{}", table.name, from_column, to_table, to_column),
                    from_table: table.name.clone(),
                    from_column,
                    to_table,
                    to_column,
                    on_delete,
                    on_update,
                });
            }

            let primary_key = columns
                .iter()
                .filter(|column| column.primary_key)
                .map(|column| column.name.clone())
                .collect();

            let columns = columns
                .into_iter()
                .map(|column| SchemaColumn {
                    name: column.name.clone(),
                    type_name: column.type_name,
                    not_null: column.not_null,
                    primary_key: column.primary_key,
                    foreign_key: foreign_key_columns.contains(&column.name),
                })
                .collect();

            nodes.push(SchemaTable {
                name: table.name.clone(),
                columns,
                primary_key,
            });
        }

        Ok(SchemaGraph {
            tables: nodes,
            relationships,
        })
    }

    /// Return the primary-key column name for a table, if it has one.
    pub(crate) fn primary_key_column(&self, table: &str) -> Result<Option<String>> {
        let columns = self.columns(table)?;
        Ok(columns
            .into_iter()
            .find(|column| column.primary_key)
            .map(|column| column.name))
    }

    /// Return column metadata for one table.
    pub fn columns(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        self.ensure_table(table)?;
        let sql = format!("PRAGMA table_info({})", quote_ident(table));
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(ColumnInfo {
                name: row.get(1)?,
                type_name: row.get(2)?,
                not_null: row.get::<_, i64>(3)? != 0,
                primary_key: row.get::<_, i64>(5)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Read one keyset-paginated page of rows ordered by rowid.
    pub fn page(
        &self,
        table: &str,
        cursor: Option<i64>,
        direction: Direction,
        limit: u32,
    ) -> Result<Page> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        let limit = limit.clamp(1, 5_000) as i64;
        let quoted = quote_ident(table);

        let (sql, params) = match (direction, cursor) {
            (Direction::Next, Some(c)) => (
                format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} \
                     WHERE rowid > ?1 ORDER BY rowid ASC LIMIT ?2"
                ),
                vec![c, limit],
            ),
            (Direction::Next, None) => (
                format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} \
                     ORDER BY rowid ASC LIMIT ?1"
                ),
                vec![limit],
            ),
            (Direction::Prev, Some(c)) => (
                format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} \
                     WHERE rowid < ?1 ORDER BY rowid DESC LIMIT ?2"
                ),
                vec![c, limit],
            ),
            (Direction::Prev, None) => (
                format!(
                    "SELECT rowid AS __dbgo_rowid, * FROM {quoted} \
                     ORDER BY rowid ASC LIMIT ?1"
                ),
                vec![limit],
            ),
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
        let mut out = Vec::new();

        while let Some(row) = rows.next()? {
            let rowid = row.get::<_, i64>(0).ok();
            let mut cells = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                let value = row.get_ref(index + 1)?;
                cells.push(cell_from_value_ref(value));
            }
            out.push(Row { rowid, cells });
        }

        if direction == Direction::Prev {
            out.reverse();
        }

        let prev_cursor = out.first().and_then(|row| row.rowid);
        let next_cursor = out.last().and_then(|row| row.rowid);

        let has_more = match next_cursor {
            Some(last) => {
                let probe = format!("SELECT 1 FROM {quoted} WHERE rowid > ?1 LIMIT 1");
                let mut stmt = self.conn.prepare(&probe)?;
                let mut rows = stmt.query([last])?;
                rows.next()?.is_some()
            }
            None => false,
        };

        Ok(Page {
            columns,
            rows: out,
            next_cursor,
            prev_cursor,
            has_more,
        })
    }

    /// Read one sorted and filtered page of rows using offset pagination.
    pub fn table_page(
        &self,
        table: &str,
        filter: Option<&FilterSpec>,
        sort: Option<&SortSpec>,
        offset: i64,
        limit: u32,
    ) -> Result<Page> {
        self.ensure_table(table)?;
        let columns = self.columns(table)?;
        let quoted = quote_ident(table);

        let (where_sql, mut params) = match filter {
            Some(filter) => filter_clause(filter, &columns)?,
            None => (String::new(), Vec::new()),
        };

        let order_sql = match sort {
            Some(sort) => {
                if !columns.iter().any(|column| column.name == sort.column) {
                    return Err(DbError::UnknownColumn(sort.column.clone()));
                }
                let column = quote_ident(&sort.column);
                let direction = match sort.direction {
                    SortDirection::Asc => "ASC",
                    SortDirection::Desc => "DESC",
                };
                format!(" ORDER BY {column} {direction}, rowid {direction}")
            }
            None => " ORDER BY rowid ASC".to_string(),
        };

        let limit = limit.clamp(1, 5_000) as i64;
        let offset = offset.max(0);
        let sql = format!(
            "SELECT rowid AS __dbgo_rowid, * FROM {quoted}{where_sql}{order_sql} LIMIT ? OFFSET ?"
        );

        params.push(rusqlite::types::Value::Integer(limit + 1));
        params.push(rusqlite::types::Value::Integer(offset));

        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
        let mut out = Vec::new();
        let mut has_more = false;

        while let Some(row) = rows.next()? {
            if out.len() == limit as usize {
                has_more = true;
                break;
            }
            let rowid = row.get::<_, i64>(0).ok();
            let mut cells = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                cells.push(cell_from_value_ref(row.get_ref(index + 1)?));
            }
            out.push(Row { rowid, cells });
        }

        let prev_cursor = out.first().and_then(|row| row.rowid);
        let next_cursor = out.last().and_then(|row| row.rowid);

        Ok(Page {
            columns,
            rows: out,
            next_cursor,
            prev_cursor,
            has_more,
        })
    }

    /// Read SQLite's data_version value used to detect external database changes.
    pub fn data_version(&self) -> Result<i64> {
        self.conn
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .map_err(DbError::from)
    }

    /// Execute one read-only SQL query and return a limited result set.
    pub fn query(&self, sql: &str, limit: u32) -> Result<QueryResult> {
        let limit = limit.clamp(1, 10_000) as usize;
        let mut stmt = self.conn.prepare(sql)?;

        if !stmt.readonly() {
            return Err(DbError::ReadOnly);
        }

        let columns: Vec<ColumnInfo> = stmt
            .columns()
            .map(|column| ColumnInfo {
                name: column.name().to_string(),
                type_name: column.decl_type().unwrap_or("").to_string(),
                not_null: false,
                primary_key: false,
            })
            .collect();

        if columns.is_empty() {
            return Err(DbError::NoColumns);
        }

        let column_count = columns.len();
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        let mut truncated = false;

        while let Some(row) = rows.next()? {
            if out.len() == limit {
                truncated = true;
                break;
            }
            let mut cells = Vec::with_capacity(column_count);
            for index in 0..column_count {
                cells.push(cell_from_value_ref(row.get_ref(index)?));
            }
            out.push(Row { rowid: None, cells });
        }

        Ok(QueryResult {
            columns,
            rows: out,
            truncated,
        })
    }

    /// Return SQLite's query plan with full-scan and index-use flags.
    pub fn explain_query(&self, sql: &str) -> Result<QueryPlan> {
        let sql = sql.trim().trim_end_matches(';').trim();
        if sql.is_empty() {
            return Err(DbError::InvalidDesign(
                "a query is required for EXPLAIN".to_string(),
            ));
        }

        let explain = format!("EXPLAIN QUERY PLAN {sql}");
        let mut statement = self.conn.prepare(&explain)?;
        let mut rows = statement.query([])?;
        let mut steps = Vec::new();
        while let Some(row) = rows.next()? {
            let detail: String = row.get(3)?;
            let upper = detail.to_ascii_uppercase();
            let full_scan = upper.starts_with("SCAN ")
                && !upper.contains(" USING ")
                && !upper.contains("SUBQUERY")
                && !upper.contains("CONSTANT ROW");
            let uses_index = upper.contains("USING INDEX")
                || upper.contains("USING COVERING INDEX")
                || upper.contains("USING INTEGER PRIMARY KEY");
            steps.push(QueryPlanStep {
                id: row.get(0)?,
                parent: row.get(1)?,
                detail,
                full_scan,
                uses_index,
            });
        }

        let full_scans = steps.iter().filter(|step| step.full_scan).count();
        let indexes_used = steps.iter().filter(|step| step.uses_index).count();
        Ok(QueryPlan {
            sql: sql.to_string(),
            steps,
            full_scans,
            indexes_used,
        })
    }

    /// Collect page, table, and index size statistics for the database.
    pub fn database_stats(&self) -> Result<DatabaseStats> {
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let free_pages: i64 = self
            .conn
            .query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
        let dbstat = self.dbstat_sizes()?;
        let estimates = self.row_count_estimates()?;
        let mut tables = Vec::new();
        for table in self.tables()? {
            tables.push(self.stats_for_table(&table.name, dbstat.as_ref(), &estimates)?);
        }
        Ok(DatabaseStats {
            path: self.path.display().to_string(),
            page_size,
            page_count,
            free_bytes: free_pages * page_size,
            total_bytes: page_count * page_size,
            dbstat: dbstat.is_some(),
            tables,
        })
    }

    /// Collect size statistics for one table and its indexes.
    pub fn table_stats(&self, table: &str) -> Result<TableStats> {
        self.ensure_table(table)?;
        let indexes = self.indexes(Some(table))?;
        let mut names = vec![table.to_string()];
        names.extend(indexes.into_iter().map(|index| index.name));
        let dbstat = self.dbstat_sizes_for(&names)?;
        let estimates = self.row_count_estimates()?;
        self.stats_for_table(table, dbstat.as_ref(), &estimates)
    }

    /// Build size statistics for one table using optional dbstat data.
    pub(crate) fn stats_for_table(
        &self,
        table: &str,
        dbstat: Option<&HashMap<String, i64>>,
        estimates: &HashMap<String, i64>,
    ) -> Result<TableStats> {
        let (row_count, row_count_capped, row_count_estimated) =
            if let Some(estimate) = estimates.get(table) {
                (*estimate, false, true)
            } else {
                let count = self.count_rows(table, 1_000_000)?;
                (count.count, count.capped, false)
            };

        let indexes = self
            .indexes(Some(table))?
            .into_iter()
            .map(|index| IndexStats {
                size_bytes: dbstat.map(|sizes| *sizes.get(&index.name).unwrap_or(&0)),
                name: index.name,
                unique: index.unique,
                columns: index.columns,
            })
            .collect::<Vec<_>>();
        let size_bytes = dbstat.map(|sizes| *sizes.get(table).unwrap_or(&0));
        let index_bytes = if size_bytes.is_some() {
            Some(
                indexes
                    .iter()
                    .filter_map(|index| index.size_bytes)
                    .sum::<i64>(),
            )
        } else {
            None
        };

        Ok(TableStats {
            name: table.to_string(),
            row_count,
            row_count_capped,
            row_count_estimated,
            size_bytes,
            index_bytes,
            indexes,
        })
    }

    /// Estimate row counts for all tables from SQLite planner statistics.
    pub(crate) fn row_count_estimates(&self) -> Result<HashMap<String, i64>> {
        let exists: i64 = self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'sqlite_stat1'
             )",
            [],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Ok(HashMap::new());
        }

        let mut statement = self.conn.prepare("SELECT tbl, stat FROM sqlite_stat1")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut estimates = HashMap::new();
        for row in rows {
            let (table, stat) = row?;
            if let Some(estimate) = stat
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<i64>().ok())
            {
                estimates.entry(table).or_insert(estimate);
            }
        }
        Ok(estimates)
    }

    /// Read per-object sizes from SQLite's dbstat virtual table when available.
    pub(crate) fn dbstat_sizes(&self) -> Result<Option<HashMap<String, i64>>> {
        let mut statement = match self
            .conn
            .prepare("SELECT name, SUM(pgsize) FROM dbstat GROUP BY name")
        {
            Ok(statement) => statement,
            Err(_) => return Ok(None),
        };
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut sizes = HashMap::new();
        for row in rows {
            let (name, size) = row?;
            sizes.insert(name, size);
        }
        Ok(Some(sizes))
    }

    /// Read dbstat sizes for a selected set of object names.
    pub(crate) fn dbstat_sizes_for(
        &self,
        names: &[String],
    ) -> Result<Option<HashMap<String, i64>>> {
        if names.is_empty() {
            return Ok(None);
        }
        let placeholders = vec!["?"; names.len()].join(", ");
        let sql = format!(
            "SELECT name, SUM(pgsize) FROM dbstat WHERE name IN ({placeholders}) GROUP BY name"
        );
        let mut statement = match self.conn.prepare(&sql) {
            Ok(statement) => statement,
            Err(_) => return Ok(None),
        };
        let rows = statement.query_map(rusqlite::params_from_iter(names.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut sizes = HashMap::new();
        for row in rows {
            let (name, size) = row?;
            sizes.insert(name, size);
        }
        Ok(Some(sizes))
    }

    /// Count rows in a table up to a cap and report whether the cap was reached.
    pub fn count_rows(&self, table: &str, cap: i64) -> Result<CountResult> {
        self.ensure_table(table)?;
        let quoted = quote_ident(table);
        let sql = format!("SELECT COUNT(*) FROM (SELECT 1 FROM {quoted} LIMIT ?1)");
        let count: i64 = self.conn.query_row(&sql, [cap + 1], |row| row.get(0))?;
        let capped = count > cap;
        Ok(CountResult {
            count: if capped { cap } else { count },
            capped,
        })
    }
}

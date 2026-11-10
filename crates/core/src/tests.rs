use super::*;

fn memory_db() -> Database {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE companies (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                domain TEXT,
                score REAL,
                logo BLOB
             );
             INSERT INTO companies (name, domain, score, logo)
             VALUES ('Acme', 'acme.test', 9.5, x'0102'),
                    ('Globex', 'globex.test', 8.0, NULL),
                    ('Initech', NULL, NULL, NULL);",
    )
    .unwrap();
    Database {
        path: PathBuf::from(":memory:"),
        conn,
        project_dir: None,
    }
}

#[test]
fn lists_tables() {
    let db = memory_db();
    let tables = db.tables().unwrap();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].name, "companies");
}

#[test]
fn pages_with_keyset() {
    let db = memory_db();
    let first = db.page("companies", None, Direction::Next, 2).unwrap();
    assert_eq!(first.rows.len(), 2);
    assert!(first.has_more);
    assert_eq!(first.rows[0].rowid, Some(1));

    let second = db
        .page("companies", first.next_cursor, Direction::Next, 2)
        .unwrap();
    assert_eq!(second.rows.len(), 1);
    assert!(!second.has_more);
    assert_eq!(second.rows[0].rowid, Some(3));

    let back = db
        .page("companies", second.prev_cursor, Direction::Prev, 2)
        .unwrap();
    assert_eq!(back.rows.len(), 2);
    assert_eq!(back.rows[0].rowid, Some(1));
}

#[test]
fn counts_rows() {
    let db = memory_db();
    let count = db.count_rows("companies", 1_000_000).unwrap();
    assert_eq!(count.count, 3);
    assert!(!count.capped);
}

#[test]
fn runs_read_only_query() {
    let db = memory_db();
    let result = db
        .query("SELECT id, name, score FROM companies ORDER BY id", 10)
        .unwrap();
    assert_eq!(result.columns.len(), 3);
    assert_eq!(result.columns[0].name, "id");
    assert_eq!(result.rows.len(), 3);
    assert!(!result.truncated);
    assert!(matches!(result.rows[0].cells[1], CellValue::Text { .. }));
}

#[test]
fn truncates_query_results() {
    let db = memory_db();
    let result = db.query("SELECT * FROM companies", 2).unwrap();
    assert_eq!(result.rows.len(), 2);
    assert!(result.truncated);
}

#[test]
fn rejects_write_statements() {
    let db = memory_db();
    let error = db.query("DELETE FROM companies", 10).unwrap_err();
    assert!(matches!(error, DbError::ReadOnly));
}

#[test]
fn builds_schema_map() {
    let db = memory_db();
    let map = db.schema_map().unwrap();
    let columns: Vec<&str> = map["companies"].iter().map(String::as_str).collect();
    assert_eq!(columns, vec!["id", "name", "domain", "score", "logo"]);
}

#[test]
fn builds_schema_graph() {
    let db = memory_db();
    db.conn
        .execute_batch(
            "CREATE TABLE links (
                    id INTEGER PRIMARY KEY,
                    company_id INTEGER NOT NULL REFERENCES companies(id) ON DELETE CASCADE
                );",
        )
        .unwrap();

    let graph = db.schema_graph().unwrap();
    assert_eq!(graph.tables.len(), 2);

    let links = graph
        .tables
        .iter()
        .find(|table| table.name == "links")
        .unwrap();
    let company_id = links
        .columns
        .iter()
        .find(|column| column.name == "company_id")
        .unwrap();
    assert!(company_id.foreign_key);
    assert_eq!(links.primary_key, vec!["id".to_string()]);

    assert_eq!(graph.relationships.len(), 1);
    let relationship = &graph.relationships[0];
    assert_eq!(relationship.from_table, "links");
    assert_eq!(relationship.from_column, "company_id");
    assert_eq!(relationship.to_table, "companies");
    assert_eq!(relationship.to_column, "id");
    assert_eq!(relationship.on_delete, "CASCADE");
}

#[test]
fn sorts_table_page() {
    let db = memory_db();
    let asc = SortSpec {
        column: "name".into(),
        direction: SortDirection::Asc,
    };
    let page = db.table_page("companies", None, Some(&asc), 0, 10).unwrap();
    let names: Vec<&str> = page
        .rows
        .iter()
        .map(|row| match &row.cells[1] {
            CellValue::Text { value } => value.as_str(),
            _ => "",
        })
        .collect();
    assert_eq!(names, vec!["Acme", "Globex", "Initech"]);

    let desc = SortSpec {
        column: "name".into(),
        direction: SortDirection::Desc,
    };
    let page = db
        .table_page("companies", None, Some(&desc), 0, 10)
        .unwrap();
    let names: Vec<&str> = page
        .rows
        .iter()
        .map(|row| match &row.cells[1] {
            CellValue::Text { value } => value.as_str(),
            _ => "",
        })
        .collect();
    assert_eq!(names, vec!["Initech", "Globex", "Acme"]);
}

#[test]
fn filters_table_page() {
    let db = memory_db();
    let contains = FilterSpec {
        column: "name".into(),
        op: FilterOp::Contains,
        value: Some("glob".into()),
    };
    let page = db
        .table_page("companies", Some(&contains), None, 0, 10)
        .unwrap();
    assert_eq!(page.rows.len(), 1);

    let is_null = FilterSpec {
        column: "domain".into(),
        op: FilterOp::IsNull,
        value: None,
    };
    let page = db
        .table_page("companies", Some(&is_null), None, 0, 10)
        .unwrap();
    assert_eq!(page.rows.len(), 1);
}

#[test]
fn rejects_unknown_sort_column() {
    let db = memory_db();
    let sort = SortSpec {
        column: "missing".into(),
        direction: SortDirection::Asc,
    };
    assert!(matches!(
        db.table_page("companies", None, Some(&sort), 0, 10),
        Err(DbError::UnknownColumn(_))
    ));
}

fn column(name: &str, type_name: &str) -> ColumnDesign {
    ColumnDesign {
        name: name.into(),
        type_name: type_name.into(),
        primary_key: false,
        not_null: false,
        unique: false,
        default_value: None,
        references: None,
    }
}

#[test]
fn creates_database_with_migrations_table() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.db");
    let (db, backup) = Database::create(&path, false).unwrap();
    assert!(backup.is_none());
    assert!(db.tables().unwrap().is_empty());

    let migrations: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM _dbgo_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(migrations, 0);

    assert!(matches!(
        Database::create(&path, false),
        Err(DbError::DatabaseExists(_))
    ));
}

#[test]
fn creates_table_with_foreign_key_and_index() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.db");
    let (db, _) = Database::create(&path, false).unwrap();

    let companies = TableDesign {
        name: "companies".into(),
        columns: vec![
            ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            },
            ColumnDesign {
                not_null: true,
                ..column("name", "TEXT")
            },
        ],
        index_foreign_keys: true,
    };
    db.create_table(&companies).unwrap();

    let links = TableDesign {
        name: "links".into(),
        columns: vec![
            ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            },
            ColumnDesign {
                not_null: true,
                references: Some(ForeignKeyDesign {
                    table: "companies".into(),
                    column: "id".into(),
                    on_delete: Some("CASCADE".into()),
                }),
                ..column("company_id", "INTEGER")
            },
        ],
        index_foreign_keys: true,
    };
    db.create_table(&links).unwrap();

    assert_eq!(db.tables().unwrap().len(), 2);

    let (target_table, from_column, to_column): (String, String, String) = db
        .conn
        .query_row("PRAGMA foreign_key_list('links')", [], |row| {
            Ok((row.get(2)?, row.get(3)?, row.get(4)?))
        })
        .unwrap();
    assert_eq!(target_table, "companies");
    assert_eq!(from_column, "company_id");
    assert_eq!(to_column, "id");

    let indexes: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_links_company_id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(indexes, 1);

    let migrations: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM _dbgo_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(migrations, 2);
}

#[test]
fn rejects_duplicate_column_names() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.db");
    let (db, _) = Database::create(&path, false).unwrap();
    let design = TableDesign {
        name: "things".into(),
        columns: vec![column("name", "TEXT"), column("NAME", "TEXT")],
        index_foreign_keys: true,
    };
    assert!(matches!(
        db.create_table(&design),
        Err(DbError::InvalidDesign(_))
    ));
}

#[test]
fn backs_up_existing_database_before_replacing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.db");
    let (db, _) = Database::create(&path, false).unwrap();
    db.create_table(&TableDesign {
        name: "keep".into(),
        columns: vec![ColumnDesign {
            primary_key: true,
            not_null: true,
            ..column("id", "INTEGER")
        }],
        index_foreign_keys: true,
    })
    .unwrap();
    drop(db);

    let (fresh, backup) = Database::create(&path, true).unwrap();
    assert!(backup.is_some());
    assert!(backup.as_ref().unwrap().exists());
    assert!(fresh.tables().unwrap().is_empty());
}

#[test]
fn creates_project_with_agent_files() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("database");
    let (db, backup) = Database::create_project(&project, false).unwrap();
    assert!(backup.is_none());
    assert!(project.join("dbgo.json").exists());
    assert!(project.join("AGENTS.md").exists());
    assert!(project.join("schema.md").exists());
    assert!(project.join("backups").is_dir());
    assert!(project.join("migrations").is_dir());
    assert_eq!(db.project_dir().unwrap(), project.as_path());
    assert!(db.tables().unwrap().is_empty());

    let reopened = Database::open_project(&project).unwrap();
    assert_eq!(reopened.project_dir().unwrap(), project.as_path());
    assert!(reopened.tables().unwrap().is_empty());
}

#[test]
fn create_table_updates_project_files() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("database");
    let (db, _) = Database::create_project(&project, false).unwrap();
    db.create_table(&TableDesign {
        name: "companies".into(),
        columns: vec![ColumnDesign {
            primary_key: true,
            not_null: true,
            ..column("id", "INTEGER")
        }],
        index_foreign_keys: true,
    })
    .unwrap();

    let schema = std::fs::read_to_string(project.join("schema.md")).unwrap();
    assert!(schema.contains("## companies"));
    assert!(schema.contains("| id | INTEGER | PK, NOT NULL |"));

    let migrations: Vec<_> = std::fs::read_dir(project.join("migrations"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .collect();
    assert_eq!(migrations.len(), 1);
    let migration = std::fs::read_to_string(migrations[0].path()).unwrap();
    assert!(migration.contains("CREATE TABLE \"companies\""));
}

#[test]
fn backup_writes_into_project_backups() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("database");
    let (db, _) = Database::create_project(&project, false).unwrap();
    let backup = db.backup().unwrap();
    assert!(backup.starts_with(project.join("backups")));
    assert!(backup.exists());
}

fn temp_companies_db() -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let (db, _) = Database::create(&path, false).unwrap();
    db.create_table(&TableDesign {
        name: "companies".into(),
        columns: vec![
            ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            },
            ColumnDesign {
                not_null: true,
                unique: true,
                ..column("domain", "TEXT")
            },
            ColumnDesign {
                ..column("city", "TEXT")
            },
        ],
        index_foreign_keys: true,
    })
    .unwrap();
    (dir, db)
}

fn json_row(values: &[(&str, serde_json::Value)]) -> JsonObject {
    values
        .iter()
        .map(|(key, value)| (key.to_string(), value.clone()))
        .collect()
}

fn city_of(db: &Database, domain: &str) -> String {
    let result = db
        .query(
            &format!("SELECT city FROM companies WHERE domain = '{domain}'"),
            10,
        )
        .unwrap();
    match &result.rows[0].cells[0] {
        CellValue::Text { value } => value.clone(),
        other => panic!("expected text, got {other:?}"),
    }
}

#[test]
fn inserts_rows_and_audits() {
    let (_dir, db) = temp_companies_db();
    let result = db
        .insert_rows(
            "companies",
            &[json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ])],
        )
        .unwrap();

    assert_eq!(result.affected, 1);
    assert!(result.backup_path.is_none());
    assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
    assert_eq!(city_of(&db, "acme.test"), "Berlin");

    let sessions = db.sessions().unwrap();
    assert_eq!(sessions.len(), 2);
    let inserted = sessions
        .iter()
        .find(|session| session.session_id == result.session_id)
        .expect("insert session is listed");
    assert_eq!(inserted.actions, 1);
}

#[test]
fn upserts_rows_on_unique_key() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Berlin")),
        ])],
    )
    .unwrap();

    let result = db
        .upsert_rows(
            "companies",
            "domain",
            &[json_row(&[
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Munich")),
            ])],
        )
        .unwrap();

    assert_eq!(result.affected, 1);
    assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
    assert_eq!(city_of(&db, "acme.test"), "Munich");
}

#[test]
fn updates_rows_by_filter() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Berlin")),
        ])],
    )
    .unwrap();

    let filter = FilterSpec {
        column: "domain".into(),
        op: FilterOp::Eq,
        value: Some("acme.test".into()),
    };
    let result = db
        .update_rows(
            "companies",
            &filter,
            &json_row(&[("city", serde_json::json!("Paris"))]),
        )
        .unwrap();

    assert_eq!(result.affected, 1);
    assert_eq!(city_of(&db, "acme.test"), "Paris");
}

#[test]
fn deletes_rows_by_filter() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Berlin")),
        ])],
    )
    .unwrap();

    let filter = FilterSpec {
        column: "domain".into(),
        op: FilterOp::Eq,
        value: Some("acme.test".into()),
    };
    let result = db.delete_rows("companies", &filter).unwrap();
    assert_eq!(result.affected, 1);
    assert_eq!(db.count_rows("companies", 100).unwrap().count, 0);
}

#[test]
fn rejects_primary_key_updates() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
        ])],
    )
    .unwrap();

    let filter = FilterSpec {
        column: "domain".into(),
        op: FilterOp::Eq,
        value: Some("acme.test".into()),
    };
    let result = db.update_rows(
        "companies",
        &filter,
        &json_row(&[("id", serde_json::json!(2))]),
    );
    assert!(matches!(result, Err(DbError::InvalidDesign(_))));
}

#[test]
fn saves_batch_of_updates_inserts_and_deletes_in_one_session() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[
            json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ]),
            json_row(&[
                ("id", serde_json::json!(2)),
                ("domain", serde_json::json!("globex.test")),
                ("city", serde_json::json!("Austin")),
            ]),
        ],
    )
    .unwrap();

    let rowids = |db: &Database| -> Vec<i64> {
        db.query("SELECT rowid FROM companies ORDER BY id", 10)
            .unwrap()
            .rows
            .iter()
            .map(|row| match row.cells[0] {
                CellValue::Integer { value } => value,
                ref other => panic!("expected integer rowid, got {other:?}"),
            })
            .collect()
    };
    let ids = rowids(&db);

    let result = db
        .save_changes(
            "companies",
            &[RowUpdate {
                rowid: ids[0],
                values: json_row(&[("city", serde_json::json!("Munich"))]),
            }],
            &[json_row(&[
                ("id", serde_json::json!(3)),
                ("domain", serde_json::json!("initech.test")),
                ("city", serde_json::json!("Tokyo")),
            ])],
            &[ids[1]],
        )
        .unwrap();

    assert_eq!(result.affected, 3);
    assert!(result.backup_path.is_none());
    assert_eq!(db.count_rows("companies", 100).unwrap().count, 2);
    assert_eq!(city_of(&db, "acme.test"), "Munich");
    assert_eq!(city_of(&db, "initech.test"), "Tokyo");

    let sessions = db.sessions().unwrap();
    assert_eq!(sessions.len(), 3);
    let saved = sessions
        .iter()
        .find(|session| session.session_id == result.session_id)
        .expect("saved session is listed");
    assert_eq!(saved.actions, 3);
}

#[test]
fn save_changes_can_replace_a_deleted_key_in_one_batch() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Berlin")),
        ])],
    )
    .unwrap();
    let rowid = match db.query("SELECT rowid FROM companies", 10).unwrap().rows[0].cells[0] {
        CellValue::Integer { value } => value,
        ref other => panic!("expected integer rowid, got {other:?}"),
    };

    db.save_changes(
        "companies",
        &[],
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme-new.test")),
            ("city", serde_json::json!("Paris")),
        ])],
        &[rowid],
    )
    .unwrap();

    assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
    assert_eq!(city_of(&db, "acme-new.test"), "Paris");
}

#[test]
fn save_changes_rejects_primary_key_updates() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
        ])],
    )
    .unwrap();
    let rowid = match db.query("SELECT rowid FROM companies", 10).unwrap().rows[0].cells[0] {
        CellValue::Integer { value } => value,
        ref other => panic!("expected integer rowid, got {other:?}"),
    };

    let result = db.save_changes(
        "companies",
        &[RowUpdate {
            rowid,
            values: json_row(&[("id", serde_json::json!(2))]),
        }],
        &[],
        &[],
    );
    assert!(matches!(result, Err(DbError::InvalidDesign(_))));
}

#[test]
fn save_changes_rolls_back_the_whole_batch() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Berlin")),
        ])],
    )
    .unwrap();
    let ids = db
        .query("SELECT rowid FROM companies", 10)
        .unwrap()
        .rows
        .iter()
        .map(|row| match row.cells[0] {
            CellValue::Integer { value } => value,
            ref other => panic!("expected integer rowid, got {other:?}"),
        })
        .collect::<Vec<_>>();

    let result = db.save_changes(
        "companies",
        &[RowUpdate {
            rowid: ids[0],
            values: json_row(&[("city", serde_json::json!("Paris"))]),
        }],
        &[],
        &[999_999],
    );

    assert!(result.is_err());
    assert_eq!(city_of(&db, "acme.test"), "Berlin");
}

#[test]
fn alters_table_add_rename_and_drop_columns() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Berlin")),
        ])],
    )
    .unwrap();

    let operations = vec![
        AlterTableOp::AddColumn {
            column: ColumnDesign {
                name: "country".into(),
                type_name: "TEXT".into(),
                primary_key: false,
                not_null: true,
                unique: false,
                default_value: Some("US".into()),
                references: None,
            },
        },
        AlterTableOp::RenameColumn {
            from: "city".into(),
            to: "location".into(),
        },
        AlterTableOp::DropColumn {
            name: "country".into(),
        },
    ];

    let preview = db.preview_alter_table("companies", &operations).unwrap();
    assert_eq!(preview.len(), 3);
    assert!(db
        .columns("companies")
        .unwrap()
        .iter()
        .any(|c| c.name == "city"));

    let change = db.alter_table("companies", &operations).unwrap();
    assert_eq!(change.statements.len(), 3);
    assert!(change.backup_path.is_none());

    let columns = db.columns("companies").unwrap();
    let names = columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>();
    assert_eq!(names, vec!["id", "domain", "location"]);
    let result = db
        .query(
            "SELECT location FROM companies WHERE domain = 'acme.test'",
            10,
        )
        .unwrap();
    assert!(matches!(
        result.rows[0].cells[0],
        CellValue::Text { ref value } if value == "Berlin"
    ));
}

#[test]
fn rejects_unsafe_alter_operations() {
    let (_dir, db) = temp_companies_db();
    let not_null = db.preview_alter_table(
        "companies",
        &[AlterTableOp::AddColumn {
            column: ColumnDesign {
                name: "country".into(),
                type_name: "TEXT".into(),
                primary_key: false,
                not_null: true,
                unique: false,
                default_value: None,
                references: None,
            },
        }],
    );
    assert!(matches!(not_null, Err(DbError::InvalidDesign(_))));

    let drop_indexed = db.preview_alter_table(
        "companies",
        &[AlterTableOp::DropColumn {
            name: "domain".into(),
        }],
    );
    assert!(matches!(drop_indexed, Err(DbError::InvalidDesign(_))));
}

#[test]
fn creates_and_drops_indexes() {
    let (_dir, db) = temp_companies_db();
    let change = db
        .create_index(
            "companies",
            "idx_companies_city",
            &["city".to_string()],
            false,
        )
        .unwrap();
    assert!(change.migration_name.contains("idx_companies_city"));
    let indexes = db.indexes(Some("companies")).unwrap();
    let created = indexes
        .iter()
        .find(|index| index.name == "idx_companies_city")
        .expect("index is listed");
    assert_eq!(created.columns, vec!["city"]);

    let duplicate = db.create_index(
        "companies",
        "idx_companies_city",
        &["city".to_string()],
        false,
    );
    assert!(duplicate.is_err());

    db.drop_index("idx_companies_city").unwrap();
    assert!(!db
        .indexes(Some("companies"))
        .unwrap()
        .iter()
        .any(|index| index.name == "idx_companies_city"));
}

#[test]
fn creates_and_drops_views() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Berlin")),
        ])],
    )
    .unwrap();

    db.create_view(
        "berlin_companies",
        "SELECT * FROM companies WHERE city = 'Berlin'",
    )
    .unwrap();
    assert_eq!(db.views().unwrap().len(), 1);
    let result = db.query("SELECT domain FROM berlin_companies", 10).unwrap();
    assert_eq!(result.rows.len(), 1);

    db.drop_view("berlin_companies").unwrap();
    assert!(db.views().unwrap().is_empty());
}

#[test]
fn lists_and_applies_pending_migrations() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("database");
    let (db, _) = Database::create_project(&project, false).unwrap();
    db.create_table(&TableDesign {
        name: "notes".into(),
        columns: vec![
            ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            },
            ColumnDesign {
                not_null: true,
                ..column("body", "TEXT")
            },
        ],
        index_foreign_keys: true,
    })
    .unwrap();

    let migrations = db.migrations().unwrap();
    assert!(migrations.iter().any(|m| m.applied));

    std::fs::write(
        project.join("migrations").join("0002_manual.sql"),
        "CREATE TABLE tags (id INTEGER PRIMARY KEY, label TEXT NOT NULL);\n",
    )
    .unwrap();

    let migrations = db.migrations().unwrap();
    let pending = migrations
        .iter()
        .find(|migration| migration.name == "0002_manual")
        .expect("manual migration is listed");
    assert!(!pending.applied);

    let result = db.apply_migrations().unwrap();
    assert_eq!(result.applied.len(), 1);
    assert_eq!(result.applied[0].name, "0002_manual");
    assert!(db.table_exists("tags").unwrap());

    let migrations = db.migrations().unwrap();
    assert!(
        migrations
            .iter()
            .find(|migration| migration.name == "0002_manual")
            .unwrap()
            .applied
    );
    assert!(db.apply_migrations().unwrap().applied.is_empty());
}

#[test]
fn recognizes_old_create_table_migrations_by_sql() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("legacy");
    let (db, _) = Database::create_project(&project, false).unwrap();
    let sql = "CREATE TABLE \"legacy\" (\"id\" INTEGER PRIMARY KEY);";
    db.conn
        .execute(
            "INSERT INTO _dbgo_migrations (name, sql) VALUES (?1, ?2)",
            rusqlite::params!["create_table:legacy", sql],
        )
        .unwrap();
    std::fs::write(
        project.join("migrations").join("0001_create_legacy.sql"),
        format!("{sql}\n"),
    )
    .unwrap();

    let migrations = db.migrations().unwrap();
    let file = migrations
        .iter()
        .find(|migration| migration.name == "0001_create_legacy")
        .expect("legacy file is listed");
    assert!(file.applied);
}

#[test]
fn lists_backups_and_restores() {
    let (_dir, mut db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Berlin")),
        ])],
    )
    .unwrap();
    let backup = db.backup().unwrap();

    db.update_rows(
        "companies",
        &FilterSpec {
            column: "domain".into(),
            op: FilterOp::Eq,
            value: Some("acme.test".into()),
        },
        &json_row(&[("city", serde_json::json!("Paris"))]),
    )
    .unwrap();
    assert_eq!(city_of(&db, "acme.test"), "Paris");

    let listed = db.backups().unwrap();
    let listed_backup = listed
        .iter()
        .find(|info| info.path == backup.canonicalize().unwrap().display().to_string())
        .expect("explicit backup is listed");
    assert!(listed_backup.size_bytes > 0);

    let result = db.restore_backup(&backup.display().to_string()).unwrap();
    assert!(result.safety_backup.is_none());
    assert_eq!(result.tables, 1);
    assert_eq!(city_of(&db, "acme.test"), "Berlin");

    let after = db.backups().unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(
        after[0].path,
        backup.canonicalize().unwrap().display().to_string()
    );
    let sessions = db.sessions().unwrap();
    assert!(sessions
        .iter()
        .any(|session| session.session_id == result.session_id));
}

#[test]
fn refuses_to_restore_a_file_outside_the_backup_directory() {
    let (_dir, mut db) = temp_companies_db();
    let result = db.restore_backup("/etc/passwd");
    assert!(result.is_err());
}

#[test]
fn adopts_bare_database_with_agent_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("companies.db");
    let (mut db, _) = Database::create(&path, false).unwrap();
    db.create_table(&TableDesign {
        name: "companies".into(),
        columns: vec![
            ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            },
            ColumnDesign {
                not_null: true,
                unique: true,
                ..column("domain", "TEXT")
            },
        ],
        index_foreign_keys: true,
    })
    .unwrap();

    let info = db.ensure_agent_files().unwrap();
    assert!(info.created);
    assert_eq!(info.database, "companies.db");
    assert_eq!(db.project_dir(), Some(dir.path()));

    for name in ["dbgo.json", "AGENTS.md", "schema.md"] {
        assert!(dir.path().join(name).is_file(), "{name} should exist");
    }
    assert!(dir.path().join("migrations").is_dir());
    assert!(dir.path().join("backups").is_dir());

    let agents = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(agents.contains("dbgo alter-table"));
    assert!(agents.contains("dbgo backups"));
    assert!(agents.contains("dbgo restore"));
    let schema = std::fs::read_to_string(dir.path().join("schema.md")).unwrap();
    assert!(schema.contains("## companies"));
    let config = read_project_config(dir.path()).unwrap();
    assert_eq!(config.database, "companies.db");

    let migration_files = std::fs::read_dir(dir.path().join("migrations"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    assert!(!migration_files.is_empty());
    assert!(migration_files.iter().any(|entry| {
        std::fs::read_to_string(entry.path())
            .map(|sql| sql.contains("CREATE TABLE"))
            .unwrap_or(false)
    }));

    let backup = db.backup().unwrap();
    assert!(backup.starts_with(dir.path().join("backups")));
}

#[test]
fn adoption_preserves_existing_agent_and_schema_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("AGENTS.md"),
        "# My project\n\nCustom instructions.\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("schema.md"),
        "# My schema\n\nHand-written notes.\n",
    )
    .unwrap();
    let path = dir.path().join("companies.db");
    let (mut db, _) = Database::create(&path, false).unwrap();
    db.create_table(&TableDesign {
        name: "companies".into(),
        columns: vec![ColumnDesign {
            primary_key: true,
            not_null: true,
            ..column("id", "INTEGER")
        }],
        index_foreign_keys: true,
    })
    .unwrap();

    db.ensure_agent_files().unwrap();
    db.ensure_agent_files().unwrap();

    let agents = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(agents.starts_with("# My project"));
    assert!(agents.contains("Custom instructions."));
    assert_eq!(agents.matches("<!-- dbgo:agent-guide:start -->").count(), 1);
    assert!(agents.contains("dbgo alter-table"));

    let schema = std::fs::read_to_string(dir.path().join("schema.md")).unwrap();
    assert!(schema.starts_with("# My schema"));
    assert!(schema.contains("Hand-written notes."));
    assert_eq!(schema.matches("<!-- dbgo:schema:start -->").count(), 1);
    assert!(schema.contains("## companies"));
}

#[test]
fn refuses_adoption_when_project_file_points_elsewhere() {
    let dir = tempfile::tempdir().unwrap();
    write_project_config(
        dir.path(),
        &ProjectConfig {
            name: "other".into(),
            database: "other.db".into(),
            version: 1,
            created_at: now_secs(),
        },
    )
    .unwrap();
    let path = dir.path().join("companies.db");
    let (mut db, _) = Database::create(&path, false).unwrap();

    assert!(db.ensure_agent_files().is_err());
    assert!(!dir.path().join("AGENTS.md").exists());
}

#[test]
fn reports_table_and_database_stats() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[
            json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ]),
            json_row(&[
                ("id", serde_json::json!(2)),
                ("domain", serde_json::json!("globex.test")),
                ("city", serde_json::json!("Austin")),
            ]),
        ],
    )
    .unwrap();

    let table = db.table_stats("companies").unwrap();
    assert_eq!(table.row_count, 2);
    assert!(!table.row_count_estimated);
    assert!(!table.indexes.is_empty());
    assert!(table.size_bytes.is_some());
    assert!(table.size_bytes.unwrap() > 0);

    let stats = db.database_stats().unwrap();
    assert!(stats.dbstat);
    assert!(stats.total_bytes > 0);
    assert!(stats
        .tables
        .iter()
        .any(|candidate| candidate.name == "companies"));
}

#[test]
fn exports_table_and_query_results() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("domain", serde_json::json!("acme.test")),
            ("city", serde_json::json!("Ber, \"lin\"\nline")),
        ])],
    )
    .unwrap();

    let mut csv = Vec::new();
    let rows = db
        .export_table("companies", None, None, ExportFormat::Csv, &mut csv)
        .unwrap();
    assert_eq!(rows, 1);
    let csv = String::from_utf8(csv).unwrap();
    assert!(csv.starts_with("id,domain,city\n"));
    assert!(csv.contains("\"Ber, \"\"lin\"\"\nline\""));

    let mut json = Vec::new();
    let rows = db
        .export_query(
            "SELECT domain, city FROM companies",
            ExportFormat::Json,
            &mut json,
        )
        .unwrap();
    assert_eq!(rows, 1);
    let parsed: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["domain"], "acme.test");

    let mut rejected = Vec::new();
    assert!(matches!(
        db.export_query("DELETE FROM companies", ExportFormat::Csv, &mut rejected),
        Err(DbError::ReadOnly)
    ));
}

#[test]
fn previews_and_imports_csv() {
    let (_dir, db) = temp_companies_db();
    let files = tempfile::tempdir().unwrap();
    let path = files.path().join("rows.csv");
    std::fs::write(
        &path,
        "id,domain,city\n1,acme.test,Berlin\n2,globex.test,Austin\n",
    )
    .unwrap();

    let preview = preview_import(&path, ImportFormat::Csv, true, None).unwrap();
    assert_eq!(preview.total_rows, 2);
    assert_eq!(preview.columns.len(), 3);
    assert_eq!(preview.columns[0].inferred_type, "INTEGER");
    assert_eq!(preview.columns[2].inferred_type, "TEXT");

    let result = db
        .import_file(
            &path,
            &ImportRequest {
                table: Some("companies".into()),
                create_table: false,
                format: ImportFormat::Csv,
                has_headers: true,
                delimiter: None,
                mapping: Vec::new(),
                upsert_key: None,
            },
        )
        .unwrap();

    assert_eq!(result.rows, 2);
    assert!(!result.created_table);
    assert!(result.ignored_columns.is_empty());
    assert_eq!(db.count_rows("companies", 100).unwrap().count, 2);
    assert_eq!(city_of(&db, "acme.test"), "Berlin");
}

#[test]
fn imports_json_and_creates_a_table() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.db");
    let (db, _) = Database::create(&path, false).unwrap();
    let json = dir.path().join("people.json");
    std::fs::write(
        &json,
        r#"[{"id":1,"name":"Ada","score":9.5},{"id":2,"name":"Grace","score":8}]"#,
    )
    .unwrap();

    let result = db
        .import_file(
            &json,
            &ImportRequest {
                table: Some("people".into()),
                create_table: true,
                format: ImportFormat::Json,
                has_headers: true,
                delimiter: None,
                mapping: Vec::new(),
                upsert_key: None,
            },
        )
        .unwrap();

    assert!(result.created_table);
    assert_eq!(result.table, "people");
    assert_eq!(result.rows, 2);
    assert!(db.table_exists("people").unwrap());
    let columns = db.columns("people").unwrap();
    assert!(columns
        .iter()
        .any(|column| column.name == "id" && column.type_name == "INTEGER"));
    assert!(columns
        .iter()
        .any(|column| column.name == "name" && column.type_name == "TEXT"));
    assert!(columns
        .iter()
        .any(|column| column.name == "score" && column.type_name == "REAL"));
}

#[test]
fn import_sanitizes_csv_headers_and_supports_upsert() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.db");
    let (db, _) = Database::create(&path, false).unwrap();
    let csv = dir.path().join("people.csv");
    std::fs::write(&csv, "id,First Name\n1,Ada\n").unwrap();

    let result = db
        .import_file(
            &csv,
            &ImportRequest {
                table: None,
                create_table: true,
                format: ImportFormat::Csv,
                has_headers: true,
                delimiter: None,
                mapping: Vec::new(),
                upsert_key: None,
            },
        )
        .unwrap();
    assert_eq!(result.table, "people");
    let columns = db.columns("people").unwrap();
    assert!(columns.iter().any(|column| column.name == "First_Name"));
}

#[test]
fn import_rejects_invalid_csv_values() {
    let (_dir, db) = temp_companies_db();
    let files = tempfile::tempdir().unwrap();
    let path = files.path().join("bad.csv");
    std::fs::write(&path, "id,domain\nnot-a-number,acme.test\n").unwrap();

    let result = db.import_file(
        &path,
        &ImportRequest {
            table: Some("companies".into()),
            create_table: false,
            format: ImportFormat::Csv,
            has_headers: true,
            delimiter: None,
            mapping: Vec::new(),
            upsert_key: None,
        },
    );
    assert!(result.is_err());
    assert_eq!(db.count_rows("companies", 100).unwrap().count, 0);
}

#[test]
fn import_upserts_using_a_key() {
    let (_dir, db) = temp_companies_db();
    let files = tempfile::tempdir().unwrap();
    let first = files.path().join("first.csv");
    std::fs::write(&first, "id,domain,city\n1,acme.test,Berlin\n").unwrap();
    db.import_file(
        &first,
        &ImportRequest {
            table: Some("companies".into()),
            create_table: false,
            format: ImportFormat::Csv,
            has_headers: true,
            delimiter: None,
            mapping: Vec::new(),
            upsert_key: None,
        },
    )
    .unwrap();

    let second = files.path().join("second.csv");
    std::fs::write(&second, "domain,city\nacme.test,Paris\n").unwrap();
    let result = db
        .import_file(
            &second,
            &ImportRequest {
                table: Some("companies".into()),
                create_table: false,
                format: ImportFormat::Csv,
                has_headers: true,
                delimiter: None,
                mapping: Vec::new(),
                upsert_key: Some("domain".into()),
            },
        )
        .unwrap();

    assert_eq!(result.rows, 1);
    assert_eq!(db.count_rows("companies", 100).unwrap().count, 1);
    assert_eq!(city_of(&db, "acme.test"), "Paris");
}

#[test]
fn import_mapping_overrides_without_disabling_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let (db, _) = Database::create(&db_path, false).unwrap();
    db.create_table(&TableDesign {
        name: "people".into(),
        columns: vec![
            ColumnDesign {
                ..column("name", "TEXT")
            },
            ColumnDesign {
                unique: true,
                ..column("domain", "TEXT")
            },
            ColumnDesign {
                ..column("city", "TEXT")
            },
        ],
        index_foreign_keys: true,
    })
    .unwrap();

    let files = tempfile::tempdir().unwrap();
    let path = files.path().join("mapped.csv");
    std::fs::write(&path, "domain,Full Name,city\nacme.test,Acme,Berlin\n").unwrap();

    let result = db
        .import_file(
            &path,
            &ImportRequest {
                table: Some("people".into()),
                create_table: false,
                format: ImportFormat::Csv,
                has_headers: true,
                delimiter: None,
                mapping: vec![
                    ImportColumnMapping {
                        source: "Full Name".into(),
                        target: Some("name".into()),
                    },
                    ImportColumnMapping {
                        source: "city".into(),
                        target: None,
                    },
                ],
                upsert_key: None,
            },
        )
        .unwrap();

    assert_eq!(result.rows, 1);
    assert_eq!(result.ignored_columns, vec!["city".to_string()]);
    let row = db
        .query(
            "SELECT name, city FROM people WHERE domain = 'acme.test'",
            1,
        )
        .unwrap();
    assert!(matches!(
        &row.rows[0].cells[0],
        CellValue::Text { value } if value == "Acme"
    ));
    assert!(matches!(row.rows[0].cells[1], CellValue::Null));
}

#[test]
fn import_preserves_leading_zero_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.db");
    let (db, _) = Database::create(&path, false).unwrap();
    let csv = dir.path().join("zips.csv");
    std::fs::write(&csv, "zip\n00123\n00456\n").unwrap();

    db.import_file(
        &csv,
        &ImportRequest {
            table: None,
            create_table: true,
            format: ImportFormat::Csv,
            has_headers: true,
            delimiter: None,
            mapping: Vec::new(),
            upsert_key: None,
        },
    )
    .unwrap();

    let columns = db.columns("zips").unwrap();
    assert!(columns
        .iter()
        .any(|column| column.name == "zip" && column.type_name == "TEXT"));
    let result = db.query("SELECT zip FROM zips ORDER BY zip", 10).unwrap();
    assert!(matches!(
        result.rows[0].cells[0],
        CellValue::Text { ref value } if value == "00123"
    ));
}

#[test]
fn exports_a_d1_bundle_in_foreign_key_order() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("database");
    let (db, _) = Database::create_project(&project, false).unwrap();

    db.create_table(&TableDesign {
        name: "authors".into(),
        columns: vec![
            ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            },
            ColumnDesign {
                not_null: true,
                ..column("name", "TEXT")
            },
        ],
        index_foreign_keys: true,
    })
    .unwrap();
    db.create_table(&TableDesign {
        name: "books".into(),
        columns: vec![
            ColumnDesign {
                primary_key: true,
                not_null: true,
                ..column("id", "INTEGER")
            },
            ColumnDesign {
                not_null: true,
                references: Some(ForeignKeyDesign {
                    table: "authors".into(),
                    column: "id".into(),
                    on_delete: Some("CASCADE".into()),
                }),
                ..column("author_id", "INTEGER")
            },
            ColumnDesign {
                ..column("title", "TEXT")
            },
        ],
        index_foreign_keys: true,
    })
    .unwrap();

    db.insert_rows(
        "authors",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("name", serde_json::json!("O'Brien")),
        ])],
    )
    .unwrap();
    db.insert_rows(
        "books",
        &[json_row(&[
            ("id", serde_json::json!(1)),
            ("author_id", serde_json::json!(1)),
            ("title", serde_json::json!("A book")),
        ])],
    )
    .unwrap();
    db.conn
        .execute(
            "INSERT INTO books (id, author_id, title) VALUES (2, 1, ?1)",
            rusqlite::params![&b"book\x00blob"[..]],
        )
        .unwrap();
    db.create_view("book_titles", "SELECT title FROM books")
        .unwrap();

    let output = dir.path().join("d1");
    let result = db.export_d1(&output, Some("my-db"), false).unwrap();
    assert_eq!(result.rows, 3);
    assert_eq!(result.tables.len(), 2);
    assert!(result.tables.iter().any(|table| table.name == "authors"));
    assert!(result.tables.iter().any(|table| table.name == "books"));
    assert!(!result.data_files.is_empty());

    let schema = std::fs::read_to_string(&result.schema_file).unwrap();
    assert!(schema.contains("CREATE TABLE \"authors\""));
    assert!(schema.contains("CREATE TABLE \"books\""));
    assert!(schema.contains("CREATE INDEX"));
    assert!(schema.contains("CREATE VIEW \"book_titles\""));
    assert!(!schema.contains("_dbgo_"));
    assert!(!schema.contains("sqlite_"));

    let mut data = String::new();
    for file in &result.data_files {
        data.push_str(&std::fs::read_to_string(output.join(file)).unwrap());
    }
    let authors_position = data.find("INSERT INTO \"authors\"").unwrap();
    let books_position = data.find("INSERT INTO \"books\"").unwrap();
    assert!(authors_position < books_position);
    assert!(data.contains("O''Brien"));
    assert!(data.contains("X'626f6f6b00626c6f62'"));

    let wrangler = std::fs::read_to_string(&result.wrangler_file).unwrap();
    assert!(wrangler.contains("database_name = \"my-db\""));
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&result.manifest_file).unwrap()).unwrap();
    assert_eq!(manifest["total_rows"], 3);
    assert_eq!(manifest["database_name"], "my-db");
}

#[test]
fn d1_export_refuses_a_non_empty_directory_without_force() {
    let (_dir, db) = temp_companies_db();
    let output = tempfile::tempdir().unwrap();
    std::fs::write(output.path().join("keep.txt"), "keep").unwrap();

    assert!(db.export_d1(output.path(), None, false).is_err());
    db.export_d1(output.path(), None, true).unwrap();
    assert!(output.path().join("schema.sql").is_file());
    assert!(output.path().join("keep.txt").is_file());
}

#[test]
fn exec_runs_reads_writes_returning_and_maintenance() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[
            json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ]),
            json_row(&[
                ("id", serde_json::json!(2)),
                ("domain", serde_json::json!("globex.test")),
                ("city", serde_json::json!("Austin")),
            ]),
        ],
    )
    .unwrap();

    let read = db
        .exec(
            "SELECT city FROM companies ORDER BY id",
            false,
            true,
            false,
            10,
        )
        .unwrap();
    assert!(read.read_only);
    assert_eq!(read.rows.len(), 2);
    assert!(read.session_id.is_none());

    assert!(matches!(
        db.exec("DELETE FROM companies", false, true, false, 10),
        Err(DbError::ReadOnly)
    ));

    let update = db
        .exec(
            "UPDATE companies SET city = 'Paris' WHERE id = 1",
            true,
            true,
            false,
            10,
        )
        .unwrap();
    assert_eq!(update.affected, 1);
    assert!(update.session_id.is_some());
    assert_eq!(city_of(&db, "acme.test"), "Paris");

    let returning = db
        .exec(
            "UPDATE companies SET city = 'Rome' WHERE id = 1 RETURNING id, city",
            true,
            true,
            true,
            10,
        )
        .unwrap();
    assert_eq!(returning.columns.len(), 2);
    assert_eq!(returning.rows.len(), 1);
    assert_eq!(returning.affected, 1);

    let multi = db
            .exec(
                "UPDATE companies SET city = city WHERE id = 1; UPDATE companies SET city = city WHERE id = 2;",
                true,
                true,
                false,
                10,
            )
            .unwrap();
    assert_eq!(multi.affected, 2);

    assert!(db.exec("VACUUM", true, true, false, 10).is_err());
    db.exec("VACUUM", true, false, false, 10).unwrap();
}

#[test]
fn filters_support_and_or_not_expressions() {
    let (_dir, db) = temp_companies_db();
    db.insert_rows(
        "companies",
        &[
            json_row(&[
                ("id", serde_json::json!(1)),
                ("domain", serde_json::json!("acme.test")),
                ("city", serde_json::json!("Berlin")),
            ]),
            json_row(&[
                ("id", serde_json::json!(2)),
                ("domain", serde_json::json!("globex.test")),
                ("city", serde_json::json!("Austin")),
            ]),
            json_row(&[
                ("id", serde_json::json!(3)),
                ("domain", serde_json::json!("initech.test")),
                ("city", serde_json::json!("Paris")),
            ]),
        ],
    )
    .unwrap();

    let filter = FilterExpr::And {
        filters: vec![
            FilterExpr::Condition {
                column: "id".into(),
                op: FilterOp::Gte,
                value: Some("2".into()),
            },
            FilterExpr::Or {
                filters: vec![
                    FilterExpr::Condition {
                        column: "city".into(),
                        op: FilterOp::Eq,
                        value: Some("Berlin".into()),
                    },
                    FilterExpr::Condition {
                        column: "city".into(),
                        op: FilterOp::Eq,
                        value: Some("Paris".into()),
                    },
                ],
            },
        ],
    };
    let result = db
        .update_rows_expr(
            "companies",
            &filter,
            &json_row(&[("city", serde_json::json!("Matched"))]),
        )
        .unwrap();
    assert_eq!(result.affected, 1);
    assert_eq!(city_of(&db, "initech.test"), "Matched");
    assert_eq!(city_of(&db, "globex.test"), "Austin");

    let filter = FilterExpr::Not {
        filter: Box::new(FilterExpr::Condition {
            column: "id".into(),
            op: FilterOp::Eq,
            value: Some("3".into()),
        }),
    };
    let result = db.delete_rows_expr("companies", &filter).unwrap();
    assert_eq!(result.affected, 2);
    assert_eq!(db.count_rows("companies", 10).unwrap().count, 1);
}

#[test]
fn filter_values_accept_json_scalars() {
    let input: FilterInput =
        serde_json::from_str(r#"{"column":"id","op":"eq","value":1}"#).unwrap();
    match input.into_expr() {
        FilterExpr::Condition { value, .. } => assert_eq!(value.as_deref(), Some("1")),
        other => panic!("unexpected filter: {other:?}"),
    }

    let input: FilterInput = serde_json::from_str(
        r#"{"type":"not","filter":{"type":"condition","column":"active","op":"eq","value":true}}"#,
    )
    .unwrap();
    match input.into_expr() {
        FilterExpr::Not { filter } => match *filter {
            FilterExpr::Condition { value, .. } => assert_eq!(value.as_deref(), Some("1")),
            other => panic!("unexpected filter: {other:?}"),
        },
        other => panic!("unexpected filter: {other:?}"),
    }

    let input: FilterInput =
        serde_json::from_str(r#"{"column":"city","op":"is_null","value":null}"#).unwrap();
    match input.into_expr() {
        FilterExpr::Condition { value, .. } => assert_eq!(value, None),
        other => panic!("unexpected filter: {other:?}"),
    }

    assert!(
        serde_json::from_str::<FilterInput>(r#"{"column":"id","op":"eq","value":[1]}"#).is_err()
    );
}

#[test]
fn drops_renames_and_manages_triggers() {
    let (_dir, db) = temp_companies_db();
    let trigger = db
        .create_trigger(
            "companies_touch",
            "CREATE TRIGGER companies_touch AFTER UPDATE ON companies
                 BEGIN UPDATE companies SET city = city WHERE id = NEW.id; END",
        )
        .unwrap();
    assert!(trigger.migration_name.contains("trigger_companies_touch"));
    assert_eq!(db.triggers(None).unwrap().len(), 1);

    db.rename_table("companies", "companies2").unwrap();
    assert!(db.table_exists("companies2").unwrap());
    assert!(!db.table_exists("companies").unwrap());
    assert_eq!(db.triggers(Some("companies2")).unwrap().len(), 1);

    db.drop_trigger("companies_touch").unwrap();
    assert!(db.triggers(None).unwrap().is_empty());
    db.drop_table("companies2").unwrap();
    assert!(!db.table_exists("companies2").unwrap());
}

#[test]
fn runs_vacuum_analyze_and_pragmas() {
    let (_dir, db) = temp_companies_db();
    let write = db.pragma("user_version=7", true).unwrap();
    assert!(write.session_id.is_some());
    let read = db.pragma("user_version", false).unwrap();
    assert!(read.session_id.is_none());
    assert!(matches!(
        read.rows[0].cells[0],
        CellValue::Integer { value: 7 }
    ));
    assert!(db.analyze().unwrap().session_id.is_some());
    assert!(db.vacuum().unwrap().session_id.is_some());
}

#[test]
fn explains_query_plans_and_flags_full_scans() {
    let (_dir, db) = temp_companies_db();
    let indexed = db
        .explain_query("SELECT * FROM companies WHERE domain = 'acme.test'")
        .unwrap();
    assert!(!indexed.steps.is_empty());
    assert!(indexed.indexes_used >= 1);
    assert_eq!(indexed.full_scans, 0);

    let full_scan = db.explain_query("SELECT * FROM companies").unwrap();
    assert!(full_scan.full_scans >= 1);
}

#[test]
fn data_version_changes_from_another_connection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("version.db");
    let (db, _) = Database::create(&path, false).unwrap();
    let before = db.data_version().unwrap();

    let other = Database::open(&path).unwrap();
    other
        .conn
        .execute_batch("CREATE TABLE external_change (id INTEGER PRIMARY KEY);")
        .unwrap();

    let after = db.data_version().unwrap();
    assert_ne!(before, after);
}

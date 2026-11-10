//! Opening/creating databases, project files, backups, and restores.

use crate::*;

impl Database {
    /// Open a database file and apply DBGo's connection pragmas.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let project_dir = detect_project_dir(&path);
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-64000;
             PRAGMA mmap_size=268435456;
             PRAGMA busy_timeout=5000;
             PRAGMA foreign_keys=ON;",
        )?;
        Ok(Self {
            path,
            conn,
            project_dir,
        })
    }

    /// Return the database file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Open a DBGo project directory or a bare database path.
    pub fn open_project(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(DbError::NotAProject(format!(
                "path not found: {}",
                path.display()
            )));
        }
        if path.is_dir() {
            let config = read_project_config(path)?;
            let database_path = path.join(&config.database);
            if !database_path.exists() {
                return Err(DbError::NotAProject(format!(
                    "database file missing: {}",
                    database_path.display()
                )));
            }
            let mut database = Self::open(&database_path)?;
            database.project_dir = Some(path.to_path_buf());
            Ok(database)
        } else {
            Self::open(path)
        }
    }

    /// Return the project directory when this database belongs to a DBGo project.
    pub fn project_dir(&self) -> Option<&Path> {
        self.project_dir.as_deref()
    }

    /// Create a timestamped backup in the project's backups directory or beside the database.
    pub fn backup(&self) -> Result<PathBuf> {
        if let Some(project_dir) = &self.project_dir {
            let backups = project_dir.join("backups");
            std::fs::create_dir_all(&backups)?;
            let file_name = self
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("db.sqlite");
            let backup = backups.join(format!("{file_name}.bak-{}", now_millis()));
            backup_database_to(&self.path, &backup)?;
            Ok(backup)
        } else {
            backup_database(&self.path)
        }
    }

    /// Return the directory where backups for this database belong.
    pub(crate) fn backup_directory(&self) -> Option<PathBuf> {
        if let Some(project_dir) = &self.project_dir {
            return Some(project_dir.join("backups"));
        }
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        Some(if parent.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            parent.to_path_buf()
        })
    }

    /// Return the filename prefix used for this database's backups.
    pub(crate) fn backup_prefix(&self) -> String {
        format!(
            "{}.bak-",
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("db.sqlite")
        )
    }

    /// List available backup snapshots, newest first.
    pub fn backups(&self) -> Result<Vec<BackupInfo>> {
        let Some(directory) = self.backup_directory() else {
            return Ok(Vec::new());
        };
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let prefix = self.backup_prefix();
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.starts_with(&prefix) {
                continue;
            }
            let name = name.to_string();
            let metadata = entry.metadata()?;
            let modified_ms = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as i64)
                .unwrap_or(0);
            let canonical = std::fs::canonicalize(&path).unwrap_or(path);
            backups.push(BackupInfo {
                name,
                path: canonical.display().to_string(),
                size_bytes: metadata.len(),
                modified_ms,
            });
        }
        backups.sort_by(|left, right| {
            right
                .modified_ms
                .cmp(&left.modified_ms)
                .then_with(|| right.name.cmp(&left.name))
        });
        Ok(backups)
    }

    /// Resolve a backup filename or path and reject paths outside the backup directory.
    pub(crate) fn resolve_backup(&self, backup: &str) -> Result<PathBuf> {
        let directory = self.backup_directory().ok_or_else(|| {
            DbError::InvalidDesign("database has no backup directory".to_string())
        })?;
        let raw = PathBuf::from(backup);
        let candidate = if raw.is_absolute() {
            raw
        } else {
            let mut candidates = vec![directory.join(&raw)];
            if let Ok(current) = std::env::current_dir() {
                candidates.push(current.join(&raw));
            }
            if let Some(project_dir) = &self.project_dir {
                candidates.push(project_dir.join(&raw));
            }
            candidates
                .into_iter()
                .find(|candidate| candidate.exists())
                .ok_or_else(|| {
                    DbError::InvalidDesign(format!("backup not found: {}", raw.display()))
                })?
        };

        let candidate = candidate.canonicalize().map_err(|_| {
            DbError::InvalidDesign(format!("backup not found: {}", candidate.display()))
        })?;
        let allowed = directory.canonicalize().map_err(|_| {
            DbError::InvalidDesign(format!(
                "backup directory not found: {}",
                directory.display()
            ))
        })?;
        if !candidate.starts_with(&allowed) {
            return Err(DbError::InvalidDesign(format!(
                "backup must be inside {}",
                allowed.display()
            )));
        }
        if let Ok(live) = self.path.canonicalize() {
            if live == candidate {
                return Err(DbError::InvalidDesign(
                    "cannot restore the live database over itself".to_string(),
                ));
            }
        }
        let name = candidate
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !name.starts_with(&self.backup_prefix()) {
            return Err(DbError::InvalidDesign(format!(
                "not a DBGo backup for this database: {name}"
            )));
        }
        Ok(candidate)
    }

    /// Restore a validated backup into the live database.
    pub fn restore_backup(&mut self, backup: &str) -> Result<RestoreResult> {
        let source = self.resolve_backup(backup)?;
        {
            let check = Connection::open(&source)?;
            let integrity: String = check.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
            if integrity != "ok" {
                return Err(DbError::InvalidDesign(format!(
                    "backup failed quick_check: {integrity}"
                )));
            }
        }

        self.conn.restore(
            rusqlite::MAIN_DB,
            &source,
            None::<fn(rusqlite::backup::Progress)>,
        )?;
        self.conn
            .execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;

        self.ensure_audit_table()?;
        let session_id = new_session_id();
        let before = serde_json::json!({ "database": self.path.display().to_string() });
        let after = serde_json::json!({
            "restored_from": source.display().to_string(),
        });
        record_audit(
            &self.conn,
            &session_id,
            "backups",
            None,
            "restore",
            Some(&before),
            Some(&after),
        )?;

        if let Err(error) = self.write_schema_md() {
            eprintln!("[dbgo] warning: could not update schema.md: {error}");
        }
        let tables = self.tables()?.len();

        Ok(RestoreResult {
            restored_from: source.display().to_string(),
            safety_backup: None,
            session_id,
            tables,
        })
    }

    /// Generate the current schema.md content from the live schema.
    pub(crate) fn schema_document(&self) -> Result<String> {
        let graph = self.schema_graph()?;
        let mut markdown = schema_markdown(&graph);

        let mut index_lines = String::new();
        for table in &graph.tables {
            for index in self.indexes(Some(&table.name))? {
                if index.origin == "pk" {
                    continue;
                }
                index_lines.push_str(&format!(
                    "- `{}`: `{}`{} on {}\n",
                    md_escape(&table.name),
                    md_escape(&index.name),
                    if index.unique { " (unique)" } else { "" },
                    index
                        .columns
                        .iter()
                        .map(|column| format!("`{}`", md_escape(column)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        if !index_lines.is_empty() {
            markdown.push_str("## Indexes\n\n");
            markdown.push_str(&index_lines);
            markdown.push('\n');
        }
        Ok(markdown)
    }

    /// Write the current schema snapshot to schema.md.
    pub(crate) fn write_schema_md(&self) -> Result<()> {
        let Some(project_dir) = &self.project_dir else {
            return Ok(());
        };
        let document = self.schema_document()?;
        let path = project_dir.join("schema.md");
        let existing = std::fs::read_to_string(&path).ok();
        let merged = merge_marked_document(
            existing.as_deref(),
            &document,
            "dbgo:schema",
            looks_like_dbgo_schema,
        );
        std::fs::write(path, merged)?;
        Ok(())
    }

    /// Create or refresh AGENTS.md and schema.md beside the database.
    pub fn ensure_agent_files(&mut self) -> Result<AgentFilesInfo> {
        let (project_dir, config, created) = match &self.project_dir {
            Some(project_dir) => {
                let config = read_project_config(project_dir)?;
                (project_dir.clone(), config, false)
            }
            None => {
                let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
                let project_dir = if parent.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    parent.to_path_buf()
                };
                let config_path = project_dir.join("dbgo.json");
                if config_path.exists() {
                    let config = read_project_config(&project_dir)?;
                    let config_database = project_dir.join(&config.database);
                    let matches = config_database == self.path
                        || config_database.canonicalize().ok() == self.path.canonicalize().ok();
                    if !matches {
                        return Err(DbError::InvalidDesign(format!(
                            "{} already points to another database: {}",
                            config_path.display(),
                            config.database
                        )));
                    }
                    (project_dir, config, false)
                } else {
                    let database = self
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("db.sqlite")
                        .to_string();
                    let name = project_dir
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| !name.is_empty())
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            self.path
                                .file_stem()
                                .and_then(|stem| stem.to_str())
                                .map(ToOwned::to_owned)
                        })
                        .unwrap_or_else(|| "database".to_string());
                    (
                        project_dir,
                        ProjectConfig {
                            name,
                            database,
                            version: 1,
                            created_at: now_secs(),
                        },
                        true,
                    )
                }
            }
        };

        std::fs::create_dir_all(project_dir.join("migrations"))?;
        std::fs::create_dir_all(project_dir.join("backups"))?;
        if self.table_exists("_dbgo_migrations")? {
            self.materialize_migrations(&project_dir)?;
        } else {
            self.ensure_migrations_table()?;
        }

        let agents_path = project_dir.join("AGENTS.md");
        let existing_agents = std::fs::read_to_string(&agents_path).ok();
        let agents = merge_marked_document(
            existing_agents.as_deref(),
            &agents_markdown(&config),
            "dbgo:agent-guide",
            looks_like_dbgo_agents,
        );
        std::fs::write(&agents_path, agents)?;

        let schema_path = project_dir.join("schema.md");
        let existing_schema = std::fs::read_to_string(&schema_path).ok();
        let schema = merge_marked_document(
            existing_schema.as_deref(),
            &self.schema_document()?,
            "dbgo:schema",
            looks_like_dbgo_schema,
        );
        std::fs::write(&schema_path, schema)?;

        let config_path = project_dir.join("dbgo.json");
        if created {
            write_project_config(&project_dir, &config)?;
        }
        self.project_dir = Some(project_dir.clone());

        Ok(AgentFilesInfo {
            directory: project_dir.display().to_string(),
            database: config.database,
            agents_path: agents_path.display().to_string(),
            schema_path: schema_path.display().to_string(),
            config_path: config_path.display().to_string(),
            created,
        })
    }

    /// Create a new project directory with a database and agent files.
    pub fn create_project(
        path: impl AsRef<Path>,
        overwrite: bool,
    ) -> Result<(Self, Option<PathBuf>)> {
        let project_dir = path.as_ref().to_path_buf();
        if project_dir.exists() && !project_dir.is_dir() {
            return Err(DbError::InvalidDesign(format!(
                "project path must be a directory: {}",
                project_dir.display()
            )));
        }

        let config_path = project_dir.join("dbgo.json");
        let database_path = project_dir.join("db.sqlite");
        let exists = config_path.exists() || database_path.exists();
        if exists && !overwrite {
            return Err(DbError::ProjectExists(project_dir.display().to_string()));
        }

        std::fs::create_dir_all(&project_dir)?;
        let backups_dir = project_dir.join("backups");
        let migrations_dir = project_dir.join("migrations");
        std::fs::create_dir_all(&backups_dir)?;
        std::fs::create_dir_all(&migrations_dir)?;

        let mut backup = None;
        if database_path.exists() {
            let backup_path = backups_dir.join(format!("db.sqlite.bak-{}", now_millis()));
            backup_database_to(&database_path, &backup_path)?;
            std::fs::remove_file(&database_path)?;
            remove_sidecar(&database_path, "-wal");
            remove_sidecar(&database_path, "-shm");
            backup = Some(backup_path);
        }

        let mut database = Self::open(&database_path)?;
        database.ensure_migrations_table()?;
        database.project_dir = Some(project_dir.clone());

        let config = ProjectConfig {
            name: project_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("database")
                .to_string(),
            database: "db.sqlite".to_string(),
            version: 1,
            created_at: now_secs(),
        };
        write_project_files(&project_dir, &config)?;

        Ok((database, backup))
    }

    /// Create a new bare database file, optionally backing up and replacing an existing one.
    pub fn create(path: impl AsRef<Path>, overwrite: bool) -> Result<(Self, Option<PathBuf>)> {
        let path = path.as_ref().to_path_buf();
        let backup = if path.exists() {
            if !overwrite {
                return Err(DbError::DatabaseExists(path.display().to_string()));
            }
            Some(backup_database(&path)?)
        } else {
            None
        };

        if backup.is_some() {
            std::fs::remove_file(&path)?;
            remove_sidecar(&path, "-wal");
            remove_sidecar(&path, "-shm");
        }

        let database = Self::open(&path)?;
        database.ensure_migrations_table()?;
        Ok((database, backup))
    }

    /// Create the internal _dbgo_migrations table if it does not exist.
    pub(crate) fn ensure_migrations_table(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _dbgo_migrations (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                sql TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        Ok(())
    }

    /// Regenerate the project's AGENTS.md guide.
    pub fn refresh_agent_guide(&mut self) -> Result<AgentFilesInfo> {
        self.ensure_agent_files()
    }

    /// Write recorded migrations back into the migrations directory as SQL files.
    pub(crate) fn materialize_migrations(&self, project_dir: &Path) -> Result<()> {
        let migrations_dir = project_dir.join("migrations");
        std::fs::create_dir_all(&migrations_dir)?;
        let mut statement = self
            .conn
            .prepare("SELECT name, sql FROM _dbgo_migrations ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (name, sql) = row?;
            let path = migrations_dir.join(format!("{}.sql", sanitize_slug(&name)));
            if !path.exists() {
                std::fs::write(path, format!("{sql}\n"))?;
            }
        }
        Ok(())
    }
}

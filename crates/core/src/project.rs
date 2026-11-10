//! Project configuration, paths, backups, and migration-file helpers.

use super::*;

/// Create a timestamped backup beside a bare database file.
pub(crate) fn backup_database(path: &Path) -> Result<PathBuf> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("database");
    let backup = path.with_file_name(format!("{file_name}.bak-{}", now_millis()));
    backup_database_to(path, &backup)?;
    Ok(backup)
}

/// Copy a database into a destination file using SQLite's VACUUM INTO.
pub(crate) fn backup_database_to(source: &Path, destination: &Path) -> Result<()> {
    let connection = Connection::open(source)?;
    connection.execute("VACUUM INTO ?1", [destination.to_string_lossy().as_ref()])?;
    Ok(())
}

/// Return the current Unix time in milliseconds.
pub(crate) fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Return the current Unix time in seconds.
pub(crate) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Find the DBGo project directory that owns a database path, if any.
pub(crate) fn detect_project_dir(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let config_path = parent.join("dbgo.json");
    let content = std::fs::read_to_string(config_path).ok()?;
    let config: ProjectConfig = serde_json::from_str(&content).ok()?;
    if parent.join(&config.database) == path {
        Some(parent.to_path_buf())
    } else {
        None
    }
}

/// Read and parse dbgo.json from a project directory.
pub(crate) fn read_project_config(project_dir: &Path) -> Result<ProjectConfig> {
    let path = project_dir.join("dbgo.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|_| DbError::NotAProject(project_dir.display().to_string()))?;
    serde_json::from_str(&content)
        .map_err(|error| DbError::InvalidDesign(format!("invalid dbgo.json: {error}")))
}

/// Write dbgo.json for a project directory.
pub(crate) fn write_project_config(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    let config_json = serde_json::to_string_pretty(config)?;
    std::fs::write(project_dir.join("dbgo.json"), config_json)?;
    Ok(())
}

/// Create the standard DBGo project files: config, AGENTS.md, and schema.md.
pub(crate) fn write_project_files(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    write_project_config(project_dir, config)?;
    std::fs::write(project_dir.join("AGENTS.md"), agents_markdown(config))?;
    std::fs::write(
        project_dir.join("schema.md"),
        "# Schema\n\nNo tables yet.\n",
    )?;
    Ok(())
}

/// Convert arbitrary text into a safe lowercase filename slug.
pub(crate) fn sanitize_slug(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    slug.trim_matches('_').to_string()
}

/// Write one named SQL migration file into the project migrations directory.
pub(crate) fn write_migration_named(
    project_dir: &Path,
    name: &str,
    action: &str,
    object_name: &str,
    sql: &str,
) -> Result<PathBuf> {
    let migrations = project_dir.join("migrations");
    std::fs::create_dir_all(&migrations)?;
    let path = migrations.join(format!("{name}.sql"));
    std::fs::write(&path, format!("-- {action} {object_name}\n{sql}\n"))?;
    Ok(path)
}

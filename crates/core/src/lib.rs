//! DBGo core: SQLite access, schema management, import/export, and project files.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension};

mod agent;
mod d1;
mod database;
mod db_utils;
mod exec_utils;
mod filters;
mod import_data;
mod models;
mod project;
mod schema_doc;
mod sql;

pub use import_data::preview_import;
pub use models::*;

use agent::*;
use d1::*;
use db_utils::*;
use exec_utils::*;
use filters::*;
use import_data::{
    convert_csv_import_value, infer_import_columns, read_import_source, resolve_import_mapping,
    sanitize_import_identifier, suggest_table_name, unique_import_identifier,
};
use project::*;
use schema_doc::*;
use sql::*;

pub struct Database {
    path: PathBuf,
    conn: Connection,
    project_dir: Option<PathBuf>,
}

#[cfg(test)]
mod tests;

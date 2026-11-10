//! CLI command dispatch into the core library.

use db_go_core::*;
use std::result::Result;

use crate::args::*;
use crate::output::data::*;
use crate::output::format::*;
use crate::output::project::*;
use crate::output::schema::*;
use crate::util::*;

/// Dispatch one parsed CLI command to the core library and render its output.
pub(crate) fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Init {
            db,
            force,
            single_file,
        } => {
            if single_file {
                let (_database, backup) = Database::create(&db, force).map_err(err)?;
                if let Some(backup) = backup {
                    eprintln!("dbgo: existing database backed up to {}", backup.display());
                }
                println!("created database {}", db.display());
            } else {
                let (database, backup) = Database::create_project(&db, force).map_err(err)?;
                if let Some(backup) = backup {
                    eprintln!("dbgo: existing database backed up to {}", backup.display());
                }
                println!("created project {}", db.display());
                println!("database: {}", database.path().display());
                println!("agent guide: {}", db.join("AGENTS.md").display());
            }
            Ok(())
        }
        Command::Graph { db } => {
            let database = open(&db)?;
            let graph = database.schema_graph().map_err(err)?;
            output_graph(&graph, cli.json, cli.jsonl)
        }
        Command::CreateTable { db, file } => {
            let content = std::fs::read_to_string(&file).map_err(err)?;
            let design: TableDesign = serde_json::from_str(&content).map_err(err)?;
            let database = open(&db)?;
            let table = database.create_table(&design).map_err(err)?;
            println!("created table {}", table.name);
            Ok(())
        }
        Command::AlterTable {
            db,
            table,
            file,
            add_columns,
            rename_columns,
            drop_columns,
            sql_only,
            yes,
        } => {
            let operations = build_alter_operations(
                file.as_deref(),
                &add_columns,
                &rename_columns,
                &drop_columns,
            )?;
            if operations.is_empty() {
                return Err("no alter-table operations were provided".to_string());
            }
            if !sql_only
                && !yes
                && operations
                    .iter()
                    .any(|operation| matches!(operation, AlterTableOp::DropColumn { .. }))
            {
                return Err("refusing to drop a column without --yes".to_string());
            }

            let database = open(&db)?;
            if sql_only {
                let statements = database
                    .preview_alter_table(&table, &operations)
                    .map_err(err)?;
                print_line(&statements.join(";\n"))
            } else {
                let change = database.alter_table(&table, &operations).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
        }
        Command::DropTable { db, table, yes } => {
            if !yes {
                return Err("refusing to drop a table without --yes".to_string());
            }
            let database = open(&db)?;
            let change = database.drop_table(&table).map_err(err)?;
            output_schema_change(&change, cli.json, cli.jsonl)
        }
        Command::RenameTable { db, from, to } => {
            let database = open(&db)?;
            let change = database.rename_table(&from, &to).map_err(err)?;
            output_schema_change(&change, cli.json, cli.jsonl)
        }
        Command::Trigger { command } => match command {
            TriggerCommand::Create {
                db,
                name,
                sql,
                file,
            } => {
                let sql = match (sql, file) {
                    (Some(sql), None) => sql,
                    (None, Some(file)) => std::fs::read_to_string(&file).map_err(err)?,
                    (None, None) => {
                        return Err("trigger create needs --sql or --file".to_string());
                    }
                    (Some(_), Some(_)) => unreachable!("clap rejects conflicting args"),
                };
                let database = open(&db)?;
                let change = database.create_trigger(&name, &sql).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            TriggerCommand::Drop { db, name, yes } => {
                if !yes {
                    return Err("refusing to drop a trigger without --yes".to_string());
                }
                let database = open(&db)?;
                let change = database.drop_trigger(&name).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            TriggerCommand::List { db, table } => {
                let database = open(&db)?;
                let triggers = database.triggers(table.as_deref()).map_err(err)?;
                output_triggers(&triggers, cli.json, cli.jsonl)
            }
        },
        Command::Vacuum { db } => {
            let database = open(&db)?;
            let result = database.vacuum().map_err(err)?;
            output_maintenance(&result, cli.json, cli.jsonl)
        }
        Command::Analyze { db } => {
            let database = open(&db)?;
            let result = database.analyze().map_err(err)?;
            output_maintenance(&result, cli.json, cli.jsonl)
        }
        Command::Pragma { db, pragma, yes } => {
            let body = pragma.trim().trim_end_matches(';').trim();
            let body = if body.to_ascii_lowercase().starts_with("pragma ") {
                body[7..].trim()
            } else {
                body
            };
            let write = body.contains('=') || yes;
            if write && !yes {
                return Err("refusing to write a PRAGMA without --yes".to_string());
            }
            let database = open(&db)?;
            let result = database.pragma(body, write).map_err(err)?;
            output_maintenance(&result, cli.json, cli.jsonl)
        }
        Command::Index { command } => match command {
            IndexCommand::Create {
                db,
                table,
                name,
                columns,
                unique,
            } => {
                let columns = columns
                    .split(',')
                    .map(str::trim)
                    .filter(|column| !column.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                let database = open(&db)?;
                let change = database
                    .create_index(&table, &name, &columns, unique)
                    .map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            IndexCommand::Drop { db, name, yes } => {
                if !yes {
                    return Err("refusing to drop an index without --yes".to_string());
                }
                let database = open(&db)?;
                let change = database.drop_index(&name).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            IndexCommand::List { db, table } => {
                let database = open(&db)?;
                let indexes = database.indexes(table.as_deref()).map_err(err)?;
                output_indexes(&indexes, cli.json, cli.jsonl)
            }
        },
        Command::View { command } => match command {
            ViewCommand::Create {
                db,
                name,
                sql,
                file,
            } => {
                let select_sql = match (sql, file) {
                    (Some(sql), None) => sql,
                    (None, Some(file)) => std::fs::read_to_string(&file).map_err(err)?,
                    (None, None) => {
                        return Err("view create needs --sql or --file".to_string());
                    }
                    (Some(_), Some(_)) => unreachable!("clap rejects conflicting args"),
                };
                let database = open(&db)?;
                let change = database.create_view(&name, &select_sql).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            ViewCommand::Drop { db, name, yes } => {
                if !yes {
                    return Err("refusing to drop a view without --yes".to_string());
                }
                let database = open(&db)?;
                let change = database.drop_view(&name).map_err(err)?;
                output_schema_change(&change, cli.json, cli.jsonl)
            }
            ViewCommand::List { db } => {
                let database = open(&db)?;
                let views = database.views().map_err(err)?;
                output_views(&views, cli.json, cli.jsonl)
            }
        },
        Command::Migrations { db } => {
            let database = open(&db)?;
            let migrations = database.migrations().map_err(err)?;
            output_migrations(&migrations, cli.json, cli.jsonl)
        }
        Command::Migrate { db } => {
            let database = open(&db)?;
            let result = database.apply_migrations().map_err(err)?;
            output_migrate(&result, cli.json, cli.jsonl)
        }
        Command::Backup { db } => {
            let database = open(&db)?;
            let backup = database.backup().map_err(err)?;
            println!("{}", backup.display());
            Ok(())
        }
        Command::Backups { db } => {
            let database = open(&db)?;
            let backups = database.backups().map_err(err)?;
            output_backups(&backups, cli.json, cli.jsonl)
        }
        Command::Restore {
            db,
            backup,
            latest,
            yes,
        } => {
            if !yes {
                return Err("refusing to restore without --yes".to_string());
            }
            let mut database = open(&db)?;
            let backup = if latest {
                database
                    .backups()
                    .map_err(err)?
                    .into_iter()
                    .next()
                    .map(|info| info.path)
                    .ok_or_else(|| "no backups found".to_string())?
            } else {
                backup.ok_or_else(|| "restore needs --backup <path> or --latest".to_string())?
            };
            let result = database.restore_backup(&backup).map_err(err)?;
            output_restore(&result, cli.json, cli.jsonl)
        }
        Command::Insert { db, table, file } => {
            let rows = read_jsonl(&file)?;
            let database = open(&db)?;
            let result = database.insert_rows(&table, &rows).map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Upsert {
            db,
            table,
            key,
            file,
        } => {
            let rows = read_jsonl(&file)?;
            let database = open(&db)?;
            let result = database.upsert_rows(&table, &key, &rows).map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Update {
            db,
            table,
            filter,
            set,
            yes,
        } => {
            if !yes {
                return Err("refusing to update without --yes".to_string());
            }
            let filter: FilterInput = serde_json::from_str(&filter).map_err(err)?;
            let values: JsonObject = serde_json::from_str(&set).map_err(err)?;
            let database = open(&db)?;
            let result = database
                .update_rows_expr(&table, &filter.into_expr(), &values)
                .map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Delete {
            db,
            table,
            filter,
            yes,
        } => {
            if !yes {
                return Err("refusing to delete without --yes".to_string());
            }
            let filter: FilterInput = serde_json::from_str(&filter).map_err(err)?;
            let database = open(&db)?;
            let result = database
                .delete_rows_expr(&table, &filter.into_expr())
                .map_err(err)?;
            output_write(&result, cli.json, cli.jsonl)
        }
        Command::Sessions { db } => {
            let database = open(&db)?;
            let sessions = database.sessions().map_err(err)?;
            output_sessions(&sessions, cli.json, cli.jsonl)
        }
        Command::Stats { db } => {
            let database = open(&db)?;
            let stats = database.database_stats().map_err(err)?;
            output_stats(&stats, cli.json, cli.jsonl)
        }
        Command::AgentGuide { db } => {
            let mut database = open(&db)?;
            let info = database.refresh_agent_guide().map_err(err)?;
            println!(
                "{} agent files in {}",
                if info.created { "created" } else { "refreshed" },
                info.directory
            );
            println!("agent guide: {}", info.agents_path);
            println!("schema: {}", info.schema_path);
            Ok(())
        }
        Command::Tables { db } => {
            let database = open(&db)?;
            let tables = database.tables().map_err(err)?;
            output_tables(&tables, cli.json, cli.jsonl)
        }
        Command::Schema { db, table } => {
            let database = open(&db)?;
            let columns = database.columns(&table).map_err(err)?;
            output_columns(&columns, cli.json, cli.jsonl)
        }
        Command::Count { db, table, cap } => {
            let database = open(&db)?;
            let count = database.count_rows(&table, cap.max(1)).map_err(err)?;
            output_count(&count, cli.json, cli.jsonl)
        }
        Command::Page {
            db,
            table,
            limit,
            cursor,
            direction,
        } => {
            let database = open(&db)?;
            let page = database
                .page(&table, cursor, direction.into(), limit)
                .map_err(err)?;
            output_page(&page, cli.json, cli.jsonl)
        }
        Command::Query { db, sql, limit } => {
            let database = open(&db)?;
            let result = database.query(&sql, limit).map_err(err)?;
            output_query(&result, cli.json, cli.jsonl)
        }
        Command::Exec {
            db,
            sql,
            file,
            write,
            yes,
            no_transaction,
            returning,
            limit,
        } => {
            if write && !yes {
                return Err("refusing to execute writes without --yes".to_string());
            }
            if returning && !write {
                return Err("--returning requires --write".to_string());
            }
            let sql = match (sql, file) {
                (Some(sql), None) => sql,
                (None, Some(file)) => std::fs::read_to_string(&file).map_err(err)?,
                (None, None) => {
                    return Err("exec needs a SQL string or --file".to_string());
                }
                (Some(_), Some(_)) => unreachable!("clap rejects conflicting args"),
            };
            let database = open(&db)?;
            let result = database
                .exec(&sql, write, !no_transaction, returning, limit)
                .map_err(|error| {
                    let message = error.to_string();
                    if !no_transaction
                        && (message.contains("VACUUM") || message.contains("within a transaction"))
                    {
                        format!("{message} (try --no-transaction)")
                    } else {
                        message
                    }
                })?;
            output_exec(&result, cli.json, cli.jsonl)
        }
        Command::Explain { db, sql } => {
            let database = open(&db)?;
            let plan = database.explain_query(&sql).map_err(err)?;
            output_plan(&plan, cli.json, cli.jsonl)
        }
        Command::Export {
            db,
            table,
            query,
            format,
            output,
            filter,
            sort,
        } => {
            if table.is_none() && query.is_none() {
                return Err("export needs a table or --query".to_string());
            }
            if query.is_some() && (filter.is_some() || sort.is_some()) {
                return Err("--where and --sort apply to table exports only".to_string());
            }
            let filter = match filter {
                Some(filter) => Some(serde_json::from_str::<FilterSpec>(&filter).map_err(err)?),
                None => None,
            };
            let sort = match sort {
                Some(sort) => {
                    let (column, direction) = sort
                        .split_once(':')
                        .or_else(|| sort.split_once('='))
                        .ok_or_else(|| format!("sort expects COLUMN:ASC|DESC: {sort}"))?;
                    let direction = match direction.trim().to_ascii_lowercase().as_str() {
                        "asc" => SortDirection::Asc,
                        "desc" => SortDirection::Desc,
                        _ => return Err(format!("invalid sort direction: {direction}")),
                    };
                    Some(SortSpec {
                        column: column.trim().to_string(),
                        direction,
                    })
                }
                None => None,
            };
            let database = open(&db)?;
            let format = format.into();
            if let Some(table) = table {
                run_export(output.as_deref(), |writer| {
                    database
                        .export_table(&table, filter.as_ref(), sort.as_ref(), format, writer)
                        .map_err(err)
                })?;
            } else if let Some(query) = query {
                run_export(output.as_deref(), |writer| {
                    database.export_query(&query, format, writer).map_err(err)
                })?;
            }
            Ok(())
        }
        Command::Import {
            db,
            table,
            file,
            format,
            create_table,
            no_header,
            delimiter,
            mappings,
            upsert_key,
            preview,
        } => {
            let format = match format {
                Some(format) => format.into(),
                None => infer_import_format(&file)?,
            };
            if preview {
                let preview =
                    preview_import(&file, format, !no_header, delimiter.as_deref()).map_err(err)?;
                return output_import_preview(&preview, cli.json, cli.jsonl);
            }

            let mapping = mappings
                .iter()
                .map(|spec| {
                    let (source, target) = spec
                        .split_once('=')
                        .map(|(source, target)| (source, Some(target)))
                        .unwrap_or((spec.as_str(), Some(spec.as_str())));
                    ImportColumnMapping {
                        source: source.trim().to_string(),
                        target: target.and_then(|target| {
                            let target = target.trim();
                            if target.is_empty() {
                                None
                            } else {
                                Some(target.to_string())
                            }
                        }),
                    }
                })
                .collect::<Vec<_>>();
            let request = ImportRequest {
                table,
                create_table,
                format,
                has_headers: !no_header,
                delimiter,
                mapping,
                upsert_key,
            };
            let database = open(&db)?;
            let result = database.import_file(&file, &request).map_err(err)?;
            output_import_result(&result, cli.json, cli.jsonl)
        }
        Command::D1 { command } => match command {
            D1Command::Export {
                db,
                output,
                name,
                force,
            } => {
                let database = open(&db)?;
                let result = database
                    .export_d1(&output, name.as_deref(), force)
                    .map_err(err)?;
                output_d1_export(&result, cli.json, cli.jsonl)
            }
        },
    }
}

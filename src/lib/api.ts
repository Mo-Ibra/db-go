import { invoke } from "@tauri-apps/api/core";
import type {
  AlterTableOp,
  BackupInfo,
  ColumnInfo,
  CountResult,
  DatabaseStats,
  DbInfo,
  ExportFormat,
  ExportResponse,
  FilterSpec,
  ForeignKeyInfo,
  ImportFormat,
  ImportPreview,
  ImportRequest,
  ImportResult,
  JsonObject,
  MigrationInfo,
  MigrateResult,
  Page,
  QueryPlan,
  QueryResult,
  RestoreResult,
  RowUpdate,
  SchemaChange,
  SchemaGraph,
  SchemaMap,
  SortSpec,
  TableDesign,
  TableDetails,
  TableInfo,
  TableStats,
  WriteResult,
} from "../types";

export const api = {
  pathExists: (path: string) => invoke<boolean>("path_exists", { path }),
  initialDatabase: () => invoke<string | null>("initial_database"),
  openDatabase: (path: string) => invoke<DbInfo>("open_database", { path }),
  createDatabase: (path: string, overwrite: boolean) =>
    invoke<DbInfo>("create_database", { path, overwrite }),
  createProject: (path: string, overwrite: boolean) =>
    invoke<DbInfo>("create_project", { path, overwrite }),
  createTable: (design: TableDesign) => invoke<TableInfo>("create_table", { design }),
  tableDetails: (table: string) => invoke<TableDetails>("table_details", { table }),
  previewAlterTable: (table: string, operations: AlterTableOp[]) =>
    invoke<string[]>("preview_alter_table", { table, operations }),
  alterTable: (table: string, operations: AlterTableOp[]) =>
    invoke<SchemaChange>("alter_table", { table, operations }),
  createIndex: (table: string, name: string, columns: string[], unique: boolean) =>
    invoke<SchemaChange>("create_index", { table, name, columns, unique }),
  dropIndex: (name: string) => invoke<SchemaChange>("drop_index", { name }),
  listMigrations: () => invoke<MigrationInfo[]>("list_migrations"),
  applyMigrations: () => invoke<MigrateResult>("apply_migrations"),
  listBackups: () => invoke<BackupInfo[]>("list_backups"),
  createBackup: () => invoke<string>("create_backup"),
  restoreBackup: (backup: string) => invoke<RestoreResult>("restore_backup", { backup }),
  saveChanges: (
    table: string,
    updates: RowUpdate[],
    inserts: JsonObject[],
    deletes: number[],
  ) => invoke<WriteResult>("save_changes", { table, updates, inserts, deletes }),
  closeDatabase: () => invoke<void>("close_database"),
  listTables: () => invoke<TableInfo[]>("list_tables"),
  describeTable: (table: string) => invoke<ColumnInfo[]>("describe_table", { table }),
  schemaMap: () => invoke<SchemaMap>("schema_map"),
  schemaGraph: () => invoke<SchemaGraph>("schema_graph"),
  dataVersion: () => invoke<number>("data_version"),
  loadPage: (table: string, cursor: number | null, direction: "next" | "prev", limit: number) =>
    invoke<Page>("load_page", { table, cursor, direction, limit }),
  foreignKeys: (table: string) => invoke<ForeignKeyInfo[]>("foreign_keys", { table }),
  explainQuery: (sql: string) => invoke<QueryPlan>("explain_query", { sql }),
  databaseStats: () => invoke<DatabaseStats>("database_stats"),
  tableStats: (table: string) => invoke<TableStats>("table_stats", { table }),
  exportData: (args: {
    table: string | null;
    query: string | null;
    filter: FilterSpec | null;
    sort: SortSpec | null;
    format: ExportFormat;
    path: string;
  }) => invoke<ExportResponse>("export_data", args),
  previewImport: (
    path: string,
    format: ImportFormat,
    hasHeaders: boolean,
    delimiter: string | null,
  ) =>
    invoke<ImportPreview>("preview_import", {
      path,
      format,
      hasHeaders,
      delimiter,
    }),
  importFile: (path: string, request: ImportRequest) =>
    invoke<ImportResult>("import_file", { path, request }),
  loadTablePage: (
    table: string,
    filter: FilterSpec | null,
    sort: SortSpec | null,
    offset: number,
    limit: number,
  ) => invoke<Page>("load_table_page", { table, filter, sort, offset, limit }),
  runQuery: (sql: string, limit: number) => invoke<QueryResult>("run_query", { sql, limit }),
  countRows: (table: string) => invoke<CountResult>("count_rows", { table }),
};

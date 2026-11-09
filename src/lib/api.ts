import { invoke } from "@tauri-apps/api/core";
import type {
  ColumnInfo,
  CountResult,
  DbInfo,
  FilterSpec,
  Page,
  QueryResult,
  SchemaMap,
  SortSpec,
  TableInfo,
} from "../types";

export const api = {
  initialDatabase: () => invoke<string | null>("initial_database"),
  openDatabase: (path: string) => invoke<DbInfo>("open_database", { path }),
  closeDatabase: () => invoke<void>("close_database"),
  listTables: () => invoke<TableInfo[]>("list_tables"),
  describeTable: (table: string) => invoke<ColumnInfo[]>("describe_table", { table }),
  schemaMap: () => invoke<SchemaMap>("schema_map"),
  loadPage: (table: string, cursor: number | null, direction: "next" | "prev", limit: number) =>
    invoke<Page>("load_page", { table, cursor, direction, limit }),
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

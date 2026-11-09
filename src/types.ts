export type TableInfo = {
  name: string;
  kind: string;
};

export type ColumnInfo = {
  name: string;
  type_name: string;
  not_null: boolean;
  primary_key: boolean;
};

export type CellValue =
  | { kind: "null" }
  | { kind: "integer"; value: number }
  | { kind: "real"; value: number }
  | { kind: "text"; value: string }
  | { kind: "blob"; value: string; size: number };

export type Row = {
  rowid: number | null;
  cells: CellValue[];
};

export type Page = {
  columns: ColumnInfo[];
  rows: Row[];
  next_cursor: number | null;
  prev_cursor: number | null;
  has_more: boolean;
};

export type CountResult = {
  count: number;
  capped: boolean;
};

export type QueryResult = {
  columns: ColumnInfo[];
  rows: Row[];
  truncated: boolean;
};

export type SchemaMap = Record<string, string[]>;

export type SchemaColumn = {
  name: string;
  type_name: string;
  not_null: boolean;
  primary_key: boolean;
  foreign_key: boolean;
};

export type SchemaTable = {
  name: string;
  columns: SchemaColumn[];
  primary_key: string[];
};

export type Relationship = {
  id: string;
  from_table: string;
  from_column: string;
  to_table: string;
  to_column: string;
  on_delete: string;
  on_update: string;
};

export type SchemaGraph = {
  tables: SchemaTable[];
  relationships: Relationship[];
};

export type SortDirection = "asc" | "desc";

export type SortSpec = {
  column: string;
  direction: SortDirection;
};

export type FilterOp =
  | "contains"
  | "eq"
  | "neq"
  | "gt"
  | "gte"
  | "lt"
  | "lte"
  | "is_null"
  | "not_null";

export type FilterSpec = {
  column: string;
  op: FilterOp;
  value: string | null;
};

export type ForeignKeyDesign = {
  table: string;
  column: string;
  on_delete: string | null;
};

export type ColumnDesign = {
  name: string;
  type_name: string;
  primary_key: boolean;
  not_null: boolean;
  unique: boolean;
  default_value: string | null;
  references: ForeignKeyDesign | null;
};

export type TableDesign = {
  name: string;
  columns: ColumnDesign[];
  index_foreign_keys: boolean;
};

export type DbInfo = {
  path: string;
  tables: TableInfo[];
  backup_path?: string | null;
};

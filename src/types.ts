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

export type DbInfo = {
  path: string;
  tables: TableInfo[];
};

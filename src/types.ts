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

export type DbInfo = {
  path: string;
  tables: TableInfo[];
};

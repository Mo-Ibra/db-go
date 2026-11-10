import type { ColumnInfo, JsonObject, Row } from "@/types";
import { cellToValue, valueToCell } from "@/lib/cellValues";

export type PendingInsert = {
  key: string;
  values: JsonObject;
};

export type PendingUpdate = {
  rowid: number;
  values: JsonObject;
  before: JsonObject;
};

export type RowEditorState =
  | { mode: "insert"; row: Row | null }
  | { mode: "update"; row: Row };

export function makePendingKey(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function isBlobColumn(column: ColumnInfo): boolean {
  return column.type_name.toUpperCase().includes("BLOB");
}

export function rowToValues(columns: ColumnInfo[], row: Row): JsonObject {
  const values: JsonObject = {};
  columns.forEach((column, index) => {
    if (isBlobColumn(column)) return;
    values[column.name] = cellToValue(row.cells[index]);
  });
  return values;
}

export function valuesEqual(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function applyValuesToRow(
  columns: ColumnInfo[],
  row: Row,
  values: JsonObject,
): Row {
  return {
    ...row,
    cells: columns.map((column, index) => {
      if (column.primary_key || isBlobColumn(column) || !(column.name in values)) {
        return row.cells[index];
      }
      return valueToCell(values[column.name]);
    }),
  };
}

export function pendingInsertToRow(
  columns: ColumnInfo[],
  insert: PendingInsert,
): Row {
  return {
    rowid: null,
    pendingKey: insert.key,
    cells: columns.map((column) => valueToCell(insert.values[column.name])),
  };
}

export function mergePendingRows(
  pageRows: Row[],
  columns: ColumnInfo[],
  updates: PendingUpdate[],
  deletes: number[],
  inserts: PendingInsert[],
): Row[] {
  const updateByRowid = new Map(updates.map((update) => [update.rowid, update.values]));
  const deleted = new Set(deletes);
  const rows = pageRows
    .filter((row) => row.rowid === null || !deleted.has(row.rowid))
    .map((row) => {
      if (row.rowid === null) return row;
      const values = updateByRowid.get(row.rowid);
      return values ? applyValuesToRow(columns, row, values) : row;
    });
  return [
    ...rows,
    ...inserts.map((insert) => pendingInsertToRow(columns, insert)),
  ];
}

export function mergePendingUpdate(
  updates: PendingUpdate[],
  rowid: number,
  values: JsonObject,
  before: JsonObject,
): PendingUpdate[] {
  const existing = updates.find((update) => update.rowid === rowid);
  const merged: JsonObject = {
    ...(existing?.values ?? {}),
    ...values,
  };
  const changed: JsonObject = {};
  for (const [name, candidate] of Object.entries(merged)) {
    if (!valuesEqual(candidate, before[name])) changed[name] = candidate;
  }
  if (Object.keys(changed).length === 0) {
    return updates.filter((update) => update.rowid !== rowid);
  }
  if (existing) {
    return updates.map((update) =>
      update.rowid === rowid ? { ...update, values: changed } : update,
    );
  }
  return [...updates, { rowid, values: changed, before }];
}

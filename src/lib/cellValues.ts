import type { CellValue, ColumnInfo, Row } from "../types";

function typeMatches(typeName: string, needles: string[]): boolean {
  const upper = typeName.toUpperCase();
  return needles.some((needle) => upper.includes(needle));
}

export function isIntegerType(typeName: string): boolean {
  return typeMatches(typeName, ["INT"]);
}

export function isRealType(typeName: string): boolean {
  return typeMatches(typeName, ["REAL", "NUM", "FLOAT", "DOUBLE", "DEC"]);
}

export function isBooleanType(typeName: string): boolean {
  return typeMatches(typeName, ["BOOL"]);
}

export function isBlobType(typeName: string): boolean {
  return typeMatches(typeName, ["BLOB"]);
}

export function cellText(cell: CellValue | undefined): string {
  if (!cell) return "";
  switch (cell.kind) {
    case "null":
      return "";
    case "integer":
    case "real":
      return String(cell.value);
    case "text":
      return cell.value;
    case "blob":
      return "";
  }
}

export function cellToValue(cell: CellValue | undefined): unknown {
  if (!cell) return null;
  switch (cell.kind) {
    case "null":
      return null;
    case "integer":
    case "real":
    case "text":
      return cell.value;
    case "blob":
      return null;
  }
}

export function valueToCell(value: unknown): CellValue {
  if (value === null || value === undefined) return { kind: "null" };
  if (typeof value === "number") {
    return Number.isInteger(value)
      ? { kind: "integer", value }
      : { kind: "real", value };
  }
  if (typeof value === "boolean") {
    return { kind: "integer", value: value ? 1 : 0 };
  }
  return { kind: "text", value: String(value) };
}

export function parseCellInput(column: ColumnInfo, raw: string): unknown {
  if (raw.trim().toLowerCase() === "<null>") {
    if (column.not_null) {
      throw new Error(`${column.name} is NOT NULL.`);
    }
    return null;
  }
  if (isBlobType(column.type_name)) {
    throw new Error(`${column.name} is a BLOB column and is not editable here.`);
  }
  const trimmed = raw.trim();
  if (isBooleanType(column.type_name)) {
    if (trimmed === "1" || trimmed.toLowerCase() === "true") return 1;
    if (trimmed === "0" || trimmed.toLowerCase() === "false") return 0;
    throw new Error(`${column.name} expects 0 or 1.`);
  }
  if (isIntegerType(column.type_name)) {
    if (!/^[+-]?\d+$/.test(trimmed)) {
      throw new Error(`${column.name} expects an integer.`);
    }
    const value = Number(trimmed);
    if (!Number.isSafeInteger(value)) {
      throw new Error(`${column.name} is outside the safe integer range.`);
    }
    return value;
  }
  if (isRealType(column.type_name)) {
    if (trimmed === "" || Number.isNaN(Number(trimmed))) {
      throw new Error(`${column.name} expects a number.`);
    }
    return Number(trimmed);
  }
  return raw;
}

export function clipboardCellText(cell: CellValue): string {
  switch (cell.kind) {
    case "null":
      return "";
    case "integer":
    case "real":
      return String(cell.value);
    case "text":
      return cell.value;
    case "blob":
      return cell.value;
  }
}

export function parseClipboardRows(text: string): string[][] {
  const normalized = text.replace(/\r\n?/g, "\n");
  if (normalized === "") return [];
  const lines = normalized.split("\n");
  while (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }
  return lines.map((line) => line.split("\t"));
}

export function rowSelectionKey(row: Row, index: number): string {
  if (row.pendingKey) return `insert:${row.pendingKey}`;
  if (row.rowid !== null) return `rowid:${row.rowid}`;
  return `index:${index}`;
}

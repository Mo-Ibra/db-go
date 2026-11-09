import type { TableDesign } from "../types";

export const COLUMN_TYPES = [
  "TEXT",
  "INTEGER",
  "REAL",
  "BLOB",
  "NUMERIC",
  "BOOLEAN",
  "DATE",
  "TIMESTAMP",
  "JSON",
] as const;

export function quoteIdent(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

export function renderDefault(value: string | null): string | null {
  if (value === null) return null;
  const trimmed = value.trim();
  if (!trimmed) return "''";
  const upper = trimmed.toUpperCase();

  if (upper === "NULL") return "NULL";
  if (["CURRENT_TIMESTAMP", "CURRENT_DATE", "CURRENT_TIME"].includes(upper)) return upper;
  if (upper === "TRUE") return "1";
  if (upper === "FALSE") return "0";

  if (/^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$/.test(trimmed)) {
    return trimmed;
  }

  return `'${trimmed.replace(/'/g, "''")}'`;
}

export function buildCreateTableSql(design: TableDesign): string {
  const definitions = design.columns.map((column) => {
    const parts = [quoteIdent(column.name), column.type_name.toUpperCase()];

    if (column.primary_key) parts.push("PRIMARY KEY");
    if (column.not_null || column.primary_key) parts.push("NOT NULL");
    if (column.unique && !column.primary_key) parts.push("UNIQUE");

    const defaultValue = renderDefault(column.default_value);
    if (defaultValue !== null) parts.push(`DEFAULT ${defaultValue}`);

    if (column.references) {
      let reference = `REFERENCES ${quoteIdent(column.references.table)}(${quoteIdent(
        column.references.column,
      )})`;
      const onDelete = (column.references.on_delete ?? "NO ACTION").toUpperCase();
      if (onDelete !== "NO ACTION") reference += ` ON DELETE ${onDelete}`;
      parts.push(reference);
    }

    return `  ${parts.join(" ")}`;
  });

  const statements = [
    `CREATE TABLE ${quoteIdent(design.name.trim())} (\n${definitions.join(",\n")}\n)`,
  ];

  if (design.index_foreign_keys) {
    for (const column of design.columns) {
      if (column.references) {
        statements.push(
          `CREATE INDEX IF NOT EXISTS ${quoteIdent(
            `idx_${design.name.trim()}_${column.name}`,
          )} ON ${quoteIdent(design.name.trim())} (${quoteIdent(column.name)})`,
        );
      }
    }
  }

  return `${statements.join(";\n")};`;
}

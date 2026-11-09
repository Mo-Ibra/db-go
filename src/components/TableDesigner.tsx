import { useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import { buildCreateTableSql, COLUMN_TYPES } from "../lib/tableDdl";
import { copyText } from "../lib/clipboard";
import { cn } from "../lib/cn";
import type { SchemaMap, TableDesign, TableInfo } from "../types";

type TableDesignerProps = {
  open: boolean;
  tables: TableInfo[];
  schema: SchemaMap;
  onClose: () => void;
  onCreated: (table: string) => void;
};

type ColumnDraft = {
  key: string;
  name: string;
  typeName: string;
  primaryKey: boolean;
  notNull: boolean;
  unique: boolean;
  defaultValue: string;
  referencesTable: string;
  referencesColumn: string;
  onDelete: string;
};

let columnSequence = 0;

function nextKey(): string {
  columnSequence += 1;
  return `column-${Date.now()}-${columnSequence}`;
}

function emptyColumn(overrides: Partial<ColumnDraft> = {}): ColumnDraft {
  return {
    key: nextKey(),
    name: "",
    typeName: "TEXT",
    primaryKey: false,
    notNull: false,
    unique: false,
    defaultValue: "",
    referencesTable: "",
    referencesColumn: "",
    onDelete: "NO ACTION",
    ...overrides,
  };
}

function toDesign(
  tableName: string,
  columns: ColumnDraft[],
  indexForeignKeys: boolean,
): TableDesign {
  return {
    name: tableName.trim(),
    columns: columns.map((column) => ({
      name: column.name.trim(),
      type_name: column.typeName,
      primary_key: column.primaryKey,
      not_null: column.notNull,
      unique: column.unique,
      default_value: column.defaultValue.trim() === "" ? null : column.defaultValue,
      references: column.referencesTable
        ? {
            table: column.referencesTable,
            column: column.referencesColumn,
            on_delete: column.onDelete,
          }
        : null,
    })),
    index_foreign_keys: indexForeignKeys,
  };
}

function validateDesign(tableName: string, columns: ColumnDraft[]): string | null {
  const name = tableName.trim();
  if (!name) return "Table name is required.";
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
    return "Table names may contain only letters, numbers, and underscores, and must start with a letter or underscore.";
  }
  if (columns.length === 0) return "Add at least one column.";

  const seen = new Set<string>();
  for (const column of columns) {
    const columnName = column.name.trim();
    if (!columnName) return "Every column needs a name.";
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(columnName)) {
      return `Invalid column name: ${columnName}`;
    }
    const lower = columnName.toLowerCase();
    if (seen.has(lower)) return `Duplicate column name: ${columnName}`;
    seen.add(lower);
  }

  if (columns.filter((column) => column.primaryKey).length > 1) {
    return "Only one primary key column is supported.";
  }

  for (const column of columns) {
    if (column.referencesTable && !column.referencesColumn) {
      return `Choose a referenced column for ${column.name}.`;
    }
    if (column.referencesTable && column.onDelete === "SET NULL" && column.notNull) {
      return `${column.name} cannot be NOT NULL with ON DELETE SET NULL.`;
    }
  }

  return null;
}

const INPUT =
  "h-8 rounded-md border border-line bg-ink px-2 text-xs text-fg outline-none placeholder:text-muted-2 focus:border-accent";
const SELECT =
  "h-8 rounded-md border border-line bg-panel-2 px-1.5 text-xs text-fg outline-none focus:border-accent disabled:opacity-40";
const LABEL = "flex items-center gap-1 text-[10.5px] text-muted";
const CHECKBOX = "h-3.5 w-3.5 accent-accent";
const ICON_BUTTON =
  "inline-flex h-6 w-6 items-center justify-center rounded-md border border-transparent text-muted hover:border-line-strong hover:bg-panel-3 hover:text-fg disabled:cursor-default disabled:opacity-30";
const SMALL_GHOST =
  "inline-flex h-7 items-center justify-center rounded-md border border-transparent px-2 text-[11.5px] font-medium text-muted transition-colors hover:border-line-strong hover:bg-panel-2 hover:text-fg disabled:cursor-default disabled:opacity-40";
const FOOTER_BUTTON =
  "inline-flex h-8 items-center justify-center rounded-md border px-3 text-[12px] font-medium transition-colors disabled:cursor-default disabled:opacity-40";
const FOOTER_NEUTRAL = `${FOOTER_BUTTON} border-line-strong bg-panel-2 text-fg hover:border-[#3a4a5a] hover:bg-panel-3`;
const FOOTER_PRIMARY = `${FOOTER_BUTTON} border-[#14b8a6] bg-linear-to-b from-accent to-[#14b8a6] font-bold text-[#03110f] hover:brightness-110`;

export function TableDesigner({
  open,
  tables,
  schema,
  onClose,
  onCreated,
}: TableDesignerProps) {
  const [tableName, setTableName] = useState("");
  const [columns, setColumns] = useState<ColumnDraft[]>([]);
  const [indexForeignKeys, setIndexForeignKeys] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setTableName("");
    setColumns([
      emptyColumn({
        name: "id",
        typeName: "INTEGER",
        primaryKey: true,
        notNull: true,
      }),
    ]);
    setIndexForeignKeys(true);
    setSubmitting(false);
    setServerError(null);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !submitting) onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, submitting, onClose]);

  const validationError = useMemo(
    () => validateDesign(tableName, columns),
    [tableName, columns],
  );
  const design = useMemo(
    () => toDesign(tableName, columns, indexForeignKeys),
    [tableName, columns, indexForeignKeys],
  );
  const preview = useMemo(() => buildCreateTableSql(design), [design]);

  if (!open) return null;

  const tableOptions = tables.map((table) => table.name);
  if (tableName.trim() && !tableOptions.includes(tableName.trim())) {
    tableOptions.push(tableName.trim());
  }

  const columnsForTable = (table: string): string[] => {
    if (table === tableName.trim()) {
      return columns.map((column) => column.name.trim()).filter(Boolean);
    }
    return schema[table] ?? [];
  };

  const updateColumn = (key: string, patch: Partial<ColumnDraft>) => {
    setColumns((current) =>
      current.map((column) => (column.key === key ? { ...column, ...patch } : column)),
    );
  };

  const setPrimaryKey = (key: string, checked: boolean) => {
    setColumns((current) =>
      current.map((column) =>
        column.key === key
          ? { ...column, primaryKey: checked, notNull: checked ? true : column.notNull }
          : { ...column, primaryKey: false },
      ),
    );
  };

  const moveColumn = (index: number, direction: -1 | 1) => {
    setColumns((current) => {
      const target = index + direction;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  };

  const addTimestamps = () => {
    setColumns((current) => {
      const existing = new Set(
        current.map((column) => column.name.trim().toLowerCase()),
      );
      const additions: ColumnDraft[] = [];
      if (!existing.has("created_at")) {
        additions.push(
          emptyColumn({
            name: "created_at",
            typeName: "TEXT",
            notNull: true,
            defaultValue: "CURRENT_TIMESTAMP",
          }),
        );
      }
      if (!existing.has("updated_at")) {
        additions.push(
          emptyColumn({
            name: "updated_at",
            typeName: "TEXT",
            notNull: true,
            defaultValue: "CURRENT_TIMESTAMP",
          }),
        );
      }
      return [...current, ...additions];
    });
  };

  const handleCreate = async () => {
    if (validationError) return;
    setSubmitting(true);
    setServerError(null);
    try {
      await api.createTable(design);
      onCreated(design.name);
    } catch (cause) {
      setServerError(String(cause));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-auto bg-black/60 p-6 backdrop-blur-sm"
      onClick={(event) => {
        if (event.target === event.currentTarget && !submitting) onClose();
      }}
    >
      <div
        className="w-full max-w-[980px] rounded-xl border border-line bg-panel shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="New table"
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight">New table</h2>
            <p className="mt-0.5 text-[11px] text-muted">
              Design the schema. Every change is recorded for D1 export.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={submitting}
            aria-label="Close table designer"
            className={cn(ICON_BUTTON, "text-lg")}
          >
            ×
          </button>
        </div>

        <div className="max-h-[70vh] overflow-auto p-5">
          <div className="flex flex-wrap items-end gap-4">
            <label className="flex flex-col gap-1 text-[10.5px] font-semibold uppercase tracking-[0.08em] text-muted">
              Table name
              <input
                className={cn(INPUT, "w-[280px]")}
                value={tableName}
                onChange={(event) => setTableName(event.target.value)}
                placeholder="companies"
                aria-label="Table name"
                autoFocus
              />
            </label>
            <label className={LABEL}>
              <input
                type="checkbox"
                className={CHECKBOX}
                checked={indexForeignKeys}
                onChange={(event) => setIndexForeignKeys(event.target.checked)}
              />
              Index foreign keys
            </label>
            <button type="button" className={SMALL_GHOST} onClick={addTimestamps}>
              Add created_at / updated_at
            </button>
          </div>

          <div className="mt-4 space-y-2">
            {columns.map((column, index) => {
              const fkColumnOptions = column.referencesTable
                ? columnsForTable(column.referencesTable)
                : [];

              return (
                <div
                  key={column.key}
                  className="rounded-lg border border-line bg-panel-2/60 p-3"
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="w-4 text-center font-mono text-[10px] text-muted-2">
                      {index + 1}
                    </span>
                    <input
                      className={cn(INPUT, "w-[180px]")}
                      value={column.name}
                      onChange={(event) =>
                        updateColumn(column.key, { name: event.target.value })
                      }
                      placeholder="column_name"
                      aria-label={`Column ${index + 1} name`}
                    />
                    <select
                      className={cn(SELECT, "w-[120px]")}
                      value={column.typeName}
                      onChange={(event) =>
                        updateColumn(column.key, { typeName: event.target.value })
                      }
                      aria-label={`Column ${index + 1} type`}
                    >
                      {COLUMN_TYPES.map((type) => (
                        <option key={type} value={type}>
                          {type}
                        </option>
                      ))}
                    </select>
                    <label className={LABEL}>
                      <input
                        type="checkbox"
                        className={CHECKBOX}
                        checked={column.primaryKey}
                        onChange={(event) =>
                          setPrimaryKey(column.key, event.target.checked)
                        }
                        aria-label={`Column ${index + 1} primary key`}
                      />
                      PK
                    </label>
                    <label className={LABEL}>
                      <input
                        type="checkbox"
                        className={CHECKBOX}
                        checked={column.notNull}
                        onChange={(event) =>
                          updateColumn(column.key, { notNull: event.target.checked })
                        }
                        aria-label={`Column ${index + 1} not null`}
                      />
                      NN
                    </label>
                    <label className={LABEL}>
                      <input
                        type="checkbox"
                        className={CHECKBOX}
                        checked={column.unique}
                        onChange={(event) =>
                          updateColumn(column.key, { unique: event.target.checked })
                        }
                        aria-label={`Column ${index + 1} unique`}
                      />
                      Unique
                    </label>
                    <div className="ml-auto flex items-center gap-1">
                      <button
                        type="button"
                        className={ICON_BUTTON}
                        onClick={() => moveColumn(index, -1)}
                        disabled={index === 0}
                        aria-label={`Move column ${column.name || index + 1} up`}
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        className={ICON_BUTTON}
                        onClick={() => moveColumn(index, 1)}
                        disabled={index === columns.length - 1}
                        aria-label={`Move column ${column.name || index + 1} down`}
                      >
                        ↓
                      </button>
                      <button
                        type="button"
                        className={ICON_BUTTON}
                        onClick={() =>
                          setColumns((current) =>
                            current.filter((item) => item.key !== column.key),
                          )
                        }
                        disabled={columns.length === 1}
                        aria-label={`Remove column ${column.name || index + 1}`}
                      >
                        ×
                      </button>
                    </div>
                  </div>

                  <div className="mt-2 flex flex-wrap items-center gap-2 pl-6">
                    <input
                      className={cn(INPUT, "w-[220px]")}
                      value={column.defaultValue}
                      onChange={(event) =>
                        updateColumn(column.key, { defaultValue: event.target.value })
                      }
                      placeholder="default: NULL, 0, text"
                      aria-label={`Column ${index + 1} default`}
                    />
                    <select
                      className={cn(SELECT, "w-[170px]")}
                      value={column.referencesTable}
                      onChange={(event) => {
                        const table = event.target.value;
                        const nextColumns = table ? columnsForTable(table) : [];
                        updateColumn(column.key, {
                          referencesTable: table,
                          referencesColumn: nextColumns[0] ?? "",
                        });
                      }}
                      aria-label={`Column ${index + 1} foreign key table`}
                    >
                      <option value="">No foreign key</option>
                      {tableOptions.map((table) => (
                        <option key={table} value={table}>
                          {table}
                        </option>
                      ))}
                    </select>
                    <select
                      className={cn(SELECT, "w-[150px]")}
                      value={column.referencesColumn}
                      onChange={(event) =>
                        updateColumn(column.key, {
                          referencesColumn: event.target.value,
                        })
                      }
                      disabled={!column.referencesTable}
                      aria-label={`Column ${index + 1} foreign key column`}
                    >
                      {column.referencesTable ? (
                        fkColumnOptions.length > 0 ? (
                          fkColumnOptions.map((name) => (
                            <option key={name} value={name}>
                              {name}
                            </option>
                          ))
                        ) : (
                          <option value="">(no columns)</option>
                        )
                      ) : (
                        <option value="">column</option>
                      )}
                    </select>
                    <select
                      className={cn(SELECT, "w-[130px]")}
                      value={column.onDelete}
                      onChange={(event) =>
                        updateColumn(column.key, { onDelete: event.target.value })
                      }
                      disabled={!column.referencesTable}
                      aria-label={`Column ${index + 1} on delete`}
                    >
                      {["NO ACTION", "CASCADE", "SET NULL", "RESTRICT"].map((action) => (
                        <option key={action} value={action}>
                          {action}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
              );
            })}
          </div>

          <button
            type="button"
            className="mt-2 flex h-8 w-full items-center justify-center rounded-md border border-dashed border-line-strong bg-transparent text-[11.5px] font-medium text-muted hover:border-accent/50 hover:text-fg"
            onClick={() => setColumns((current) => [...current, emptyColumn()])}
          >
            + Add column
          </button>

          <div className="mt-4">
            <div className="mb-1 flex items-center justify-between">
              <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted">
                SQL preview
              </span>
              <button
                type="button"
                className="inline-flex h-6 items-center rounded-md border border-line bg-panel-2 px-2 text-[10.5px] text-muted hover:bg-panel-3 hover:text-fg"
                onClick={() => void copyText(preview)}
              >
                Copy
              </button>
            </div>
            <pre className="max-h-[180px] overflow-auto whitespace-pre rounded-md border border-line bg-ink p-3 font-mono text-[11px] leading-relaxed text-muted">
              {preview}
            </pre>
          </div>
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-line px-5 py-3">
          <div className="min-w-0 flex-1 text-[11px]">
            {serverError ? (
              <span className="text-danger">{serverError}</span>
            ) : validationError ? (
              <span className="text-muted-2">{validationError}</span>
            ) : (
              <span className="text-muted-2">Ready to create.</span>
            )}
          </div>
          <div className="flex flex-none items-center gap-2">
            <button
              type="button"
              className={FOOTER_NEUTRAL}
              onClick={onClose}
              disabled={submitting}
            >
              Cancel
            </button>
            <button
              type="button"
              className={FOOTER_PRIMARY}
              onClick={() => void handleCreate()}
              disabled={Boolean(validationError) || submitting}
            >
              {submitting ? "Creating..." : "Create table"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

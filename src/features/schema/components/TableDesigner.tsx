import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api";
import { buildCreateTableSql, COLUMN_TYPES } from "@/lib/tableDdl";
import { copyText } from "@/lib/clipboard";
import { ArrowDown, ArrowUp, CalendarPlus, Trash2, X } from "lucide-react";
import { cn } from "@/lib/cn";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import type { SchemaMap, TableDesign, TableInfo } from "@/types";

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
      className="fixed inset-0 z-50 flex items-center justify-center overflow-auto bg-black/65 p-4 backdrop-blur-md"
      onClick={(event) => {
        if (event.target === event.currentTarget && !submitting) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        className="modal-panel w-full max-w-[980px] rounded-xl border border-line bg-panel shadow-2xl"
        aria-label="New table"
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight">New table</h2>
            <p className="mt-0.5 text-[11px] text-muted">
              Design the schema. Every change is recorded for D1 export.
            </p>
          </div>
          <IconButton
            icon={X}
            label="Close table designer"
            onClick={onClose}
            disabled={submitting}
          />
        </div>

        <div className="max-h-[70vh] overflow-auto p-5">
          <div className="flex flex-wrap items-end gap-4">
            <label className="flex flex-col gap-1 text-[10.5px] font-semibold uppercase tracking-[0.08em] text-muted">
              Table name
              <input
                className={cn("field-input", "w-[280px]")}
                value={tableName}
                onChange={(event) => setTableName(event.target.value)}
                placeholder="companies"
                aria-label="Table name"
                autoFocus
              />
            </label>
            <label className="field-label">
              <input
                type="checkbox"
                className="h-3.5 w-3.5 accent-accent"
                checked={indexForeignKeys}
                onChange={(event) => setIndexForeignKeys(event.target.checked)}
              />
              Index foreign keys
            </label>
            <Button
              variant="ghost"
              size="sm"
              icon={CalendarPlus}
              onClick={addTimestamps}
            >
              Add created_at / updated_at
            </Button>
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
                      className={cn("field-input", "w-[180px]")}
                      value={column.name}
                      onChange={(event) =>
                        updateColumn(column.key, { name: event.target.value })
                      }
                      placeholder="column_name"
                      aria-label={`Column ${index + 1} name`}
                    />
                    <select
                      className={cn("field-select", "w-[120px]")}
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
                    <label className="field-label">
                      <input
                        type="checkbox"
                        className="h-3.5 w-3.5 accent-accent"
                        checked={column.primaryKey}
                        onChange={(event) =>
                          setPrimaryKey(column.key, event.target.checked)
                        }
                        aria-label={`Column ${index + 1} primary key`}
                      />
                      PK
                    </label>
                    <label className="field-label">
                      <input
                        type="checkbox"
                        className="h-3.5 w-3.5 accent-accent"
                        checked={column.notNull}
                        onChange={(event) =>
                          updateColumn(column.key, { notNull: event.target.checked })
                        }
                        aria-label={`Column ${index + 1} not null`}
                      />
                      NN
                    </label>
                    <label className="field-label">
                      <input
                        type="checkbox"
                        className="h-3.5 w-3.5 accent-accent"
                        checked={column.unique}
                        onChange={(event) =>
                          updateColumn(column.key, { unique: event.target.checked })
                        }
                        aria-label={`Column ${index + 1} unique`}
                      />
                      Unique
                    </label>
                    <div className="ml-auto flex items-center gap-1">
                      <IconButton
                        icon={ArrowUp}
                        label={`Move column ${column.name || index + 1} up`}
                        size="sm"
                        onClick={() => moveColumn(index, -1)}
                        disabled={index === 0}
                      />
                      <IconButton
                        icon={ArrowDown}
                        label={`Move column ${column.name || index + 1} down`}
                        size="sm"
                        onClick={() => moveColumn(index, 1)}
                        disabled={index === columns.length - 1}
                      />
                      <IconButton
                        icon={Trash2}
                        label={`Remove column ${column.name || index + 1}`}
                        size="sm"
                        variant="danger"
                        onClick={() =>
                          setColumns((current) =>
                            current.filter((item) => item.key !== column.key),
                          )
                        }
                        disabled={columns.length === 1}
                      />
                    </div>
                  </div>

                  <div className="mt-2 flex flex-wrap items-center gap-2 pl-6">
                    <input
                      className={cn("field-input", "w-[220px]")}
                      value={column.defaultValue}
                      onChange={(event) =>
                        updateColumn(column.key, { defaultValue: event.target.value })
                      }
                      placeholder="default: NULL, 0, text"
                      aria-label={`Column ${index + 1} default`}
                    />
                    <select
                      className={cn("field-select", "w-[170px]")}
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
                      className={cn("field-select", "w-[150px]")}
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
                      className={cn("field-select", "w-[130px]")}
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
            <Button variant="secondary" onClick={onClose} disabled={submitting}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={() => void handleCreate()}
              disabled={Boolean(validationError) || submitting}
            >
              {submitting ? "Creating..." : "Create table"}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

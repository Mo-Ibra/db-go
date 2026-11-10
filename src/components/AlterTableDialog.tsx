import { useEffect, useRef, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";
import { COLUMN_TYPES } from "../lib/tableDdl";
import { cn } from "../lib/cn";
import type {
  AlterTableOp,
  ColumnDesign,
  ColumnInfo,
  SchemaChange,
  SchemaMap,
} from "../types";

type AlterTableDialogProps = {
  open: boolean;
  table: string;
  columns: ColumnInfo[];
  schema: SchemaMap;
  onClose: () => void;
  onApplied: (change: SchemaChange) => void | Promise<void>;
};

const BTN =
  "inline-flex h-8 items-center justify-center rounded-md border px-3 text-[12px] font-medium transition-colors disabled:cursor-default disabled:opacity-40";
const BTN_NEUTRAL = `${BTN} border-line-strong bg-panel-2 text-fg hover:border-[#3a4a5a] hover:bg-panel-3`;
const BTN_PRIMARY = `${BTN} border-[#14b8a6] bg-linear-to-b from-accent to-[#14b8a6] font-bold text-[#03110f] hover:brightness-110`;
const INPUT =
  "h-8 rounded-md border border-line bg-ink px-2 text-xs text-fg outline-none placeholder:text-muted-2 focus:border-accent disabled:opacity-40";
const SELECT =
  "h-8 rounded-md border border-line bg-panel-2 px-1.5 text-xs text-fg outline-none focus:border-accent disabled:opacity-40";
const LABEL = "text-[10px] font-bold uppercase tracking-[0.12em] text-muted-2";
const CHECKBOX = "h-3.5 w-3.5 accent-accent";

function opLabel(operation: AlterTableOp): string {
  switch (operation.op) {
    case "add_column":
      return `Add column ${operation.column.name} ${operation.column.type_name}`;
    case "rename_column":
      return `Rename ${operation.from} to ${operation.to}`;
    case "drop_column":
      return `Drop column ${operation.name}`;
  }
}

export function AlterTableDialog({
  open,
  table,
  columns,
  schema,
  onClose,
  onApplied,
}: AlterTableDialogProps) {
  const [operations, setOperations] = useState<AlterTableOp[]>([]);
  const [addName, setAddName] = useState("");
  const [addType, setAddType] = useState("TEXT");
  const [addNotNull, setAddNotNull] = useState(false);
  const [addDefault, setAddDefault] = useState("");
  const [addForeignKey, setAddForeignKey] = useState(false);
  const [addReferenceTable, setAddReferenceTable] = useState("");
  const [addReferenceColumn, setAddReferenceColumn] = useState("");
  const [addOnDelete, setAddOnDelete] = useState("NO ACTION");
  const [renameFrom, setRenameFrom] = useState("");
  const [renameTo, setRenameTo] = useState("");
  const [dropName, setDropName] = useState("");
  const [preview, setPreview] = useState<string[]>([]);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const previewRequest = useRef(0);

  useEffect(() => {
    if (!open) return;
    setOperations([]);
    setAddName("");
    setAddType("TEXT");
    setAddNotNull(false);
    setAddDefault("");
    setAddForeignKey(false);
    setAddReferenceTable("");
    setAddReferenceColumn("");
    setAddOnDelete("NO ACTION");
    setRenameFrom(columns[0]?.name ?? "");
    setRenameTo("");
    setDropName("");
    setPreview([]);
    setPreviewError(null);
    setFormError(null);
    setApplying(false);
    // Reset only when the dialog opens or the table changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, table]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !applying) onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, applying, onClose]);

  useEffect(() => {
    if (!open) return;
    if (operations.length === 0) {
      setPreview([]);
      setPreviewError(null);
      return;
    }
    const id = ++previewRequest.current;
    void api
      .previewAlterTable(table, operations)
      .then((statements) => {
        if (id !== previewRequest.current) return;
        setPreview(statements);
        setPreviewError(null);
      })
      .catch((cause) => {
        if (id !== previewRequest.current) return;
        setPreview([]);
        setPreviewError(cause instanceof Error ? cause.message : String(cause));
      });
  }, [open, table, operations]);

  if (!open) return null;

  const addColumn = () => {
    const name = addName.trim();
    if (!name) {
      setFormError("Column name is required.");
      return;
    }
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
      setFormError("Column names may contain only letters, numbers, and underscores.");
      return;
    }
    if (columns.some((column) => column.name.toLowerCase() === name.toLowerCase())) {
      setFormError(`Column already exists: ${name}`);
      return;
    }
    if (addNotNull && addDefault.trim() === "") {
      setFormError("NOT NULL columns need a default value when added.");
      return;
    }
    if (addForeignKey && (!addReferenceTable || !addReferenceColumn)) {
      setFormError("Choose the referenced table and column.");
      return;
    }

    const column: ColumnDesign = {
      name,
      type_name: addType,
      primary_key: false,
      not_null: addNotNull,
      unique: false,
      default_value: addDefault.trim() === "" ? null : addDefault,
      references: addForeignKey
        ? {
            table: addReferenceTable,
            column: addReferenceColumn,
            on_delete: addOnDelete,
          }
        : null,
    };
    setOperations((current) => [...current, { op: "add_column", column }]);
    setAddName("");
    setAddNotNull(false);
    setAddDefault("");
    setAddForeignKey(false);
    setAddReferenceTable("");
    setAddReferenceColumn("");
    setAddOnDelete("NO ACTION");
    setFormError(null);
  };

  const addRename = () => {
    if (!renameFrom || !renameTo.trim()) {
      setFormError("Choose a source column and enter a new name.");
      return;
    }
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(renameTo.trim())) {
      setFormError("Column names may contain only letters, numbers, and underscores.");
      return;
    }
    if (renameFrom.toLowerCase() === renameTo.trim().toLowerCase()) {
      setFormError("The new column name is the same as the old one.");
      return;
    }
    setOperations((current) => [
      ...current,
      { op: "rename_column", from: renameFrom, to: renameTo.trim() },
    ]);
    setRenameTo("");
    setFormError(null);
  };

  const addDrop = () => {
    if (!dropName) {
      setFormError("Choose a column to drop.");
      return;
    }
    setOperations((current) => [...current, { op: "drop_column", name: dropName }]);
    setDropName("");
    setFormError(null);
  };

  const apply = async () => {
    if (operations.length === 0 || previewError) return;
    const drops = operations.filter((operation) => operation.op === "drop_column").length;
    if (drops > 0) {
      const confirmed = await confirm(
        `This will drop ${drops} column${drops === 1 ? "" : "s"} from ${table}. A timestamped backup is kept before the change.`,
        { title: "Apply destructive schema change?", kind: "warning" },
      );
      if (!confirmed) return;
    }

    setApplying(true);
    setFormError(null);
    try {
      const change = await api.alterTable(table, operations);
      await onApplied(change);
      onClose();
    } catch (cause) {
      setFormError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setApplying(false);
    }
  };

  const referenceColumns = addReferenceTable
    ? (schema[addReferenceTable] ?? [])
    : [];

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-auto bg-black/60 p-6 backdrop-blur-sm"
      onClick={(event) => {
        if (event.target === event.currentTarget && !applying) onClose();
      }}
    >
      <div
        className="w-full max-w-[860px] rounded-xl border border-line bg-panel shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="Alter table"
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight">Alter {table}</h2>
            <p className="mt-0.5 text-[11px] text-muted">
              Stage operations, review the SQL, then apply one migration with a backup.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={applying}
            aria-label="Close alter table"
            className="inline-flex h-6 w-6 items-center justify-center rounded-md border border-transparent text-lg leading-none text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg disabled:opacity-40"
          >
            ×
          </button>
        </div>

        <div className="max-h-[72vh] overflow-auto p-5">
          <div className="grid gap-3 lg:grid-cols-3">
            <section className="rounded-lg border border-line bg-ink/40 p-3">
              <h3 className={LABEL}>Add column</h3>
              <div className="mt-2 flex flex-col gap-2">
                <input
                  className={INPUT}
                  placeholder="column name"
                  value={addName}
                  onChange={(event) => setAddName(event.target.value)}
                />
                <select
                  className={SELECT}
                  value={addType}
                  onChange={(event) => setAddType(event.target.value)}
                >
                  {COLUMN_TYPES.map((type) => (
                    <option key={type} value={type}>
                      {type}
                    </option>
                  ))}
                </select>
                <input
                  className={INPUT}
                  placeholder="default value (optional)"
                  value={addDefault}
                  onChange={(event) => setAddDefault(event.target.value)}
                />
                <label className="flex items-center gap-1.5 text-[11px] text-muted">
                  <input
                    type="checkbox"
                    className={CHECKBOX}
                    checked={addNotNull}
                    onChange={(event) => setAddNotNull(event.target.checked)}
                    disabled={addForeignKey}
                  />
                  NOT NULL
                </label>
                <label className="flex items-center gap-1.5 text-[11px] text-muted">
                  <input
                    type="checkbox"
                    className={CHECKBOX}
                    checked={addForeignKey}
                    onChange={(event) => {
                      setAddForeignKey(event.target.checked);
                      if (event.target.checked) {
                        setAddNotNull(false);
                      } else {
                        setAddReferenceTable("");
                        setAddReferenceColumn("");
                      }
                    }}
                  />
                  Foreign key
                </label>
                {addForeignKey && (
                  <>
                    <select
                      className={SELECT}
                      value={addReferenceTable}
                      onChange={(event) => {
                        setAddReferenceTable(event.target.value);
                        setAddReferenceColumn("");
                      }}
                    >
                      <option value="">referenced table...</option>
                      {Object.keys(schema).map((name) => (
                        <option key={name} value={name}>
                          {name}
                        </option>
                      ))}
                    </select>
                    <select
                      className={SELECT}
                      value={addReferenceColumn}
                      onChange={(event) => setAddReferenceColumn(event.target.value)}
                      disabled={!addReferenceTable}
                    >
                      <option value="">referenced column...</option>
                      {referenceColumns.map((name) => (
                        <option key={name} value={name}>
                          {name}
                        </option>
                      ))}
                    </select>
                    <select
                      className={SELECT}
                      value={addOnDelete}
                      onChange={(event) => setAddOnDelete(event.target.value)}
                    >
                      {["NO ACTION", "CASCADE", "SET NULL", "RESTRICT"].map((action) => (
                        <option key={action} value={action}>
                          ON DELETE {action}
                        </option>
                      ))}
                    </select>
                  </>
                )}
                <button className={BTN_NEUTRAL} type="button" onClick={addColumn}>
                  Add to changes
                </button>
              </div>
            </section>

            <section className="rounded-lg border border-line bg-ink/40 p-3">
              <h3 className={LABEL}>Rename column</h3>
              <div className="mt-2 flex flex-col gap-2">
                <select
                  className={SELECT}
                  value={renameFrom}
                  onChange={(event) => setRenameFrom(event.target.value)}
                >
                  {columns.map((column) => (
                    <option key={column.name} value={column.name}>
                      {column.name}
                    </option>
                  ))}
                </select>
                <input
                  className={INPUT}
                  placeholder="new name"
                  value={renameTo}
                  onChange={(event) => setRenameTo(event.target.value)}
                />
                <button className={BTN_NEUTRAL} type="button" onClick={addRename}>
                  Add to changes
                </button>
              </div>
            </section>

            <section className="rounded-lg border border-line bg-ink/40 p-3">
              <h3 className={LABEL}>Drop column</h3>
              <div className="mt-2 flex flex-col gap-2">
                <select
                  className={SELECT}
                  value={dropName}
                  onChange={(event) => setDropName(event.target.value)}
                >
                  <option value="">column...</option>
                  {columns
                    .filter((column) => !column.primary_key)
                    .map((column) => (
                      <option key={column.name} value={column.name}>
                        {column.name}
                      </option>
                    ))}
                </select>
                <button
                  className={`${BTN} border-danger/35 bg-danger/10 text-[#fca5a5] hover:border-danger/60`}
                  type="button"
                  onClick={addDrop}
                >
                  Add to changes
                </button>
              </div>
            </section>
          </div>

          {formError && (
            <div className="mt-3 rounded-md border border-danger/25 bg-danger/10 px-3 py-2 font-mono text-[11px] text-[#fecaca]">
              {formError}
            </div>
          )}

          <section className="mt-4">
            <div className="mb-2 flex items-center justify-between">
              <h3 className={LABEL}>Pending operations</h3>
              <span className="text-[10.5px] text-muted">
                {operations.length} operation{operations.length === 1 ? "" : "s"}
              </span>
            </div>
            {operations.length === 0 ? (
              <div className="rounded-lg border border-line bg-ink/40 px-3 py-4 text-[11.5px] text-muted-2">
                Add an operation above.
              </div>
            ) : (
              <div className="overflow-hidden rounded-lg border border-line">
                {operations.map((operation, index) => (
                  <div
                    key={`${operation.op}-${index}`}
                    className="flex items-center justify-between gap-3 border-b border-line/60 px-3 py-2 last:border-b-0"
                  >
                    <span className="font-mono text-[11.5px] text-fg">
                      {opLabel(operation)}
                    </span>
                    <button
                      type="button"
                      className="text-[11px] text-muted hover:text-[#fca5a5]"
                      onClick={() =>
                        setOperations((current) => current.filter((_, i) => i !== index))
                      }
                    >
                      Remove
                    </button>
                  </div>
                ))}
              </div>
            )}
          </section>

          <section className="mt-4">
            <h3 className={cn(LABEL, "mb-2")}>SQL preview</h3>
            <pre className="max-h-[180px] overflow-auto whitespace-pre-wrap rounded-lg border border-line bg-ink p-3 font-mono text-[11px] leading-relaxed text-[#cbd5e1]">
              {previewError
                ? `-- ${previewError}`
                : preview.length > 0
                  ? preview.map((statement) => `${statement};`).join("\n")
                  : "-- add an operation to preview the SQL"}
            </pre>
          </section>
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <button className={BTN_NEUTRAL} type="button" onClick={onClose} disabled={applying}>
            Cancel
          </button>
          <button
            className={BTN_PRIMARY}
            type="button"
            onClick={() => void apply()}
            disabled={applying || operations.length === 0 || previewError !== null}
          >
            {applying ? "Applying..." : "Apply changes"}
          </button>
        </div>
      </div>
    </div>
  );
}

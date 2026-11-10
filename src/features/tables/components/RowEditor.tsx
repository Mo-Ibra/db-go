import { useEffect, useState } from "react";
import type { ColumnInfo, JsonObject, Row } from "@/types";
import { cellText, isBlobType, isIntegerType, parseCellInput } from "@/lib/cellValues";
import { cn } from "@/lib/cn";

type RowEditorProps = {
  open: boolean;
  mode: "insert" | "update";
  columns: ColumnInfo[];
  initial: Row | null;
  onClose: () => void;
  onSubmit: (values: JsonObject) => void;
};

function parseValue(
  column: ColumnInfo,
  raw: string,
  isNull: boolean,
): unknown {
  if (isNull) {
    if (column.not_null && !(column.primary_key && isIntegerType(column.type_name))) {
      throw new Error(`${column.name} is NOT NULL.`);
    }
    return null;
  }
  return parseCellInput(column, raw);
}

export function RowEditor({
  open,
  mode,
  columns,
  initial,
  onClose,
  onSubmit,
}: RowEditorProps) {
  const [values, setValues] = useState<Record<string, string>>({});
  const [nulls, setNulls] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const nextValues: Record<string, string> = {};
    const nextNulls: Record<string, boolean> = {};
    columns.forEach((column, index) => {
      const cell = initial?.cells[index];
      nextNulls[column.name] = cell?.kind === "null";
      nextValues[column.name] = cellText(cell);
      if (
        mode === "insert" &&
        !initial &&
        column.primary_key &&
        isIntegerType(column.type_name)
      ) {
        nextNulls[column.name] = true;
        nextValues[column.name] = "";
      }
    });
    setValues(nextValues);
    setNulls(nextNulls);
    setError(null);
  }, [open, mode, columns, initial]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  const handleSubmit = () => {
    const payload: JsonObject = {};
    try {
      for (const column of columns) {
        if (mode === "update" && column.primary_key) continue;
        // BLOB values are not editable in the grid; leave them unchanged.
        if (isBlobType(column.type_name)) continue;
        payload[column.name] = parseValue(
          column,
          values[column.name] ?? "",
          Boolean(nulls[column.name]),
        );
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      return;
    }
    onSubmit(payload);
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center overflow-auto bg-black/65 p-4 backdrop-blur-md"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        className="modal-panel w-full max-w-[620px] rounded-xl border border-line bg-panel shadow-2xl"
        aria-label={mode === "insert" ? "New row" : "Edit row"}
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight">
              {mode === "insert" ? "New row" : "Edit row"}
            </h2>
            <p className="mt-0.5 text-[11px] text-muted">
              Changes are staged until you click Save changes.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close row editor"
            className="inline-flex h-6 w-6 items-center justify-center rounded-md border border-transparent text-lg leading-none text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg"
          >
            ×
          </button>
        </div>

        <div className="max-h-[65vh] overflow-auto p-5">
          <div className="flex flex-col gap-3">
            {columns.map((column) => {
              const disabled =
                isBlobType(column.type_name) ||
                (mode === "update" && column.primary_key);
              const isNull = Boolean(nulls[column.name]);
              return (
                <label key={column.name} className="flex flex-col gap-1">
                  <span className="flex items-center gap-2 font-mono text-[10px] font-semibold uppercase tracking-[0.08em] text-muted-2">
                    {column.name}
                    <span className="normal-case tracking-normal text-muted-2/70">
                      {column.type_name}
                    </span>
                    {column.primary_key && <span className="text-accent">PK</span>}
                    {column.not_null && <span className="text-muted">NOT NULL</span>}
                  </span>
                  <div className="flex items-center gap-2">
                    <input
                      className={cn(
                        "h-8 flex-1 rounded-md border border-line bg-ink px-2 text-xs text-fg outline-none placeholder:text-muted-2 focus:border-accent disabled:opacity-40",
                      )}
                      value={isNull ? "" : (values[column.name] ?? "")}
                      onChange={(event) =>
                        setValues((current) => ({
                          ...current,
                          [column.name]: event.target.value,
                        }))
                      }
                      disabled={disabled || isNull}
                      aria-label={`${column.name} value`}
                    />
                    <label className="flex flex-none items-center gap-1 text-[10.5px] text-muted">
                      <input
                        type="checkbox"
                        className="h-3.5 w-3.5 accent-accent"
                        checked={isNull}
                        onChange={(event) =>
                          setNulls((current) => ({
                            ...current,
                            [column.name]: event.target.checked,
                          }))
                        }
                        disabled={
                          mode === "update" ||
                          (column.not_null &&
                            !(column.primary_key && isIntegerType(column.type_name)))
                        }
                        aria-label={`${column.name} is null`}
                      />
                      NULL
                    </label>
                  </div>
                </label>
              );
            })}
          </div>

          {error && (
            <div className="mt-3 rounded-md border border-danger/25 bg-danger/10 px-3 py-2 text-[11px] text-[#f85149]">
              {error}
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <button
            type="button"
            className="inline-flex h-8 items-center justify-center rounded-md border border-line-strong bg-panel-2 px-3 text-[12px] font-medium text-fg hover:border-[#3a4a5a] hover:bg-panel-3"
            onClick={onClose}
          >
            Cancel
          </button>
          <button
            type="button"
            data-testid="row-editor-submit"
            className="inline-flex h-8 items-center justify-center rounded-md border border-accent bg-accent px-3 text-[12px] font-bold text-[#0b0f14] hover:bg-[#56d364]"
            onClick={handleSubmit}
          >
            {mode === "insert" ? "Add to changes" : "Update changes"}
          </button>
        </div>
      </div>
    </div>
  );
}

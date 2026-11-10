import { useEffect, useRef, useState } from "react";
import { open as openFile } from "@tauri-apps/plugin-dialog";
import { api } from "@/lib/api";
import { cn } from "@/lib/cn";
import type {
  ImportFormat,
  ImportPreview,
  ImportRequest,
  ImportResult,
  JsonObject,
  SchemaMap,
  TableInfo,
} from "@/types";

type ImportDialogProps = {
  open: boolean;
  tables: TableInfo[];
  schema: SchemaMap;
  selectedTable: string | null;
  onClose: () => void;
  onImported: (result: ImportResult) => void | Promise<void>;
};


function inferFormat(path: string): ImportFormat {
  const extension = path.split(".").pop()?.toLowerCase();
  if (extension === "json") return "json";
  if (extension === "jsonl" || extension === "ndjson") return "jsonl";
  return "csv";
}

function fileStem(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? path;
  return name.replace(/\.[^.]+$/, "");
}

function previewValue(value: unknown): string {
  if (value === null || value === undefined) return "NULL";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

export function ImportDialog({
  open,
  tables,
  schema,
  selectedTable,
  onClose,
  onImported,
}: ImportDialogProps) {
  const [filePath, setFilePath] = useState<string | null>(null);
  const [format, setFormat] = useState<ImportFormat>("csv");
  const [hasHeaders, setHasHeaders] = useState(true);
  const [delimiter, setDelimiter] = useState(",");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<"existing" | "create">("create");
  const [targetTable, setTargetTable] = useState("");
  const [newTableName, setNewTableName] = useState("");
  const [mapping, setMapping] = useState<Record<string, string>>({});
  const [upsertKey, setUpsertKey] = useState("");
  const previewRequest = useRef(0);

  const targetColumns = schema[targetTable] ?? [];
  const targetSignature = targetColumns.join("\u0000");
  useEffect(() => {
    if (!open) return;
    setFilePath(null);
    setFormat("csv");
    setHasHeaders(true);
    setDelimiter(",");
    setPreview(null);
    setPreviewLoading(false);
    setImporting(false);
    setError(null);
    setMode(selectedTable ? "existing" : "create");
    setTargetTable(selectedTable ?? tables[0]?.name ?? "");
    setNewTableName("");
    setMapping({});
    setUpsertKey("");
    previewRequest.current += 1;
    // Reset only when the dialog opens; table/schema refreshes must not clear
    // the form while it is open.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (!open || !filePath) return;
    const id = ++previewRequest.current;
    setPreviewLoading(true);
    setError(null);
    void api
      .previewImport(
        filePath,
        format,
        hasHeaders,
        format === "csv" ? delimiter : null,
      )
      .then((result) => {
        if (id !== previewRequest.current) return;
        setPreview(result);
        setNewTableName((current) => current || result.suggested_table);
      })
      .catch((cause) => {
        if (id !== previewRequest.current) return;
        setPreview(null);
        setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (id === previewRequest.current) setPreviewLoading(false);
      });
    return () => {
      previewRequest.current += 1;
    };
  }, [open, filePath, format, hasHeaders, delimiter]);

  useEffect(() => {
    if (!preview) {
      setMapping({});
      return;
    }
    const next: Record<string, string> = {};
    for (const column of preview.columns) {
      const match = targetColumns.find(
        (target) =>
          target.toLowerCase() === column.source.toLowerCase() ||
          target.toLowerCase() === column.suggested_name.toLowerCase(),
      );
      next[column.source] = match ?? "";
    }
    setMapping(next);
    setUpsertKey((current) =>
      targetColumns.some((column) => column === current) ? current : "",
    );
    // targetColumns is represented by targetSignature to avoid resetting the
    // form when an unrelated schema refresh changes object identity.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preview, targetTable, targetSignature]);

  if (!open) return null;

  const chooseFile = async () => {
    const path = await openFile({
      multiple: false,
      directory: false,
      filters: [
        { name: "Data files", extensions: ["csv", "json", "jsonl", "ndjson"] },
        { name: "CSV", extensions: ["csv"] },
        { name: "JSON", extensions: ["json", "jsonl", "ndjson"] },
      ],
    });
    if (!path || Array.isArray(path)) return;
    setFormat(inferFormat(path));
    setFilePath(path);
    setNewTableName(fileStem(path));
    setError(null);
  };

  const mappedCount = Object.values(mapping).filter(Boolean).length;

  const runImport = async () => {
    if (!filePath || !preview) return;
    if (mode === "create" && !newTableName.trim()) {
      setError("A new table name is required.");
      return;
    }
    if (
      mode === "create" &&
      tables.some(
        (table) =>
          table.name.toLowerCase() === newTableName.trim().toLowerCase(),
      )
    ) {
      setError(`Table already exists: ${newTableName.trim()}`);
      return;
    }
    if (mode === "existing" && !targetTable) {
      setError("Choose a target table.");
      return;
    }
    if (mode === "existing" && mappedCount === 0) {
      setError("Map at least one source column.");
      return;
    }

    const request: ImportRequest = {
      table: mode === "create" ? newTableName.trim() : targetTable,
      create_table: mode === "create",
      format,
      has_headers: hasHeaders,
      delimiter: format === "csv" ? delimiter : null,
      mapping:
        mode === "existing"
          ? preview.columns.map((column) => ({
              source: column.source,
              target: mapping[column.source] || null,
            }))
          : [],
      upsert_key: mode === "existing" && upsertKey ? upsertKey : null,
    };

    setImporting(true);
    setError(null);
    try {
      const result = await api.importFile(filePath, request);
      await onImported(result);
      onClose();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setImporting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center overflow-auto bg-black/65 p-4 backdrop-blur-md"
      onClick={(event) => {
        if (event.target === event.currentTarget && !importing) onClose();
      }}
    >
      <div
        data-testid="import-dialog"
        role="dialog"
        aria-modal="true"
        className="modal-panel w-full max-w-[980px] rounded-xl border border-line bg-panel shadow-2xl"
        aria-label="Import data"
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight">Import data</h2>
            <p className="mt-0.5 text-[11px] text-muted">
              CSV, JSON, or JSONL. Writes directly to the database; take a backup
              first if you need one.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={importing}
            aria-label="Close import"
            className="inline-flex h-6 w-6 items-center justify-center rounded-md border border-transparent text-lg leading-none text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg disabled:opacity-40"
          >
            ×
          </button>
        </div>

        <div className="max-h-[74vh] overflow-auto p-5">
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              className="btn btn-neutral"
              onClick={() => void chooseFile()}
              data-testid="import-choose-file"
            >
              Choose file
            </button>
            <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-muted">
              {filePath ?? "No file selected"}
            </span>
            <label className="flex items-center gap-1 text-[10.5px] text-muted">
              Format
              <select
                className="field-select"
                value={format}
                onChange={(event) =>
                  setFormat(event.target.value as ImportFormat)
                }
                aria-label="Import format"
              >
                <option value="csv">CSV</option>
                <option value="json">JSON</option>
                <option value="jsonl">JSONL</option>
              </select>
            </label>
            {format === "csv" && (
              <>
                <label className="flex items-center gap-1 text-[10.5px] text-muted">
                  <input
                    type="checkbox"
                    className="h-3.5 w-3.5 accent-accent"
                    checked={hasHeaders}
                    onChange={(event) => setHasHeaders(event.target.checked)}
                  />
                  Header row
                </label>
                <label className="flex items-center gap-1 text-[10.5px] text-muted">
                  Delimiter
                  <select
                    className="field-select"
                    value={delimiter}
                    onChange={(event) => setDelimiter(event.target.value)}
                    aria-label="CSV delimiter"
                  >
                    <option value=",">Comma</option>
                    <option value=";">Semicolon</option>
                    <option value={"\t"}>Tab</option>
                    <option value="|">Pipe</option>
                  </select>
                </label>
              </>
            )}
          </div>

          {error && (
            <div className="mt-3 rounded-md border border-danger/25 bg-danger/10 px-3 py-2 font-mono text-[11px] text-[#f85149]">
              {error}
            </div>
          )}

          {previewLoading && (
            <div className="mt-3 text-[11.5px] text-muted">Reading preview...</div>
          )}

          {preview && (
            <>
              <div className="mt-4 grid gap-2 sm:grid-cols-2">
                <div className="card">
                  <span className="field-label">Target</span>
                  <div className="mt-2 flex gap-3 text-[11.5px]">
                    <label className="flex items-center gap-1.5">
                      <input
                        type="radio"
                        className="accent-accent"
                        checked={mode === "existing"}
                        onChange={() => {
                          setMode("existing");
                          if (!targetTable) setTargetTable(tables[0]?.name ?? "");
                        }}
                      />
                      Existing table
                    </label>
                    <label className="flex items-center gap-1.5">
                      <input
                        type="radio"
                        className="accent-accent"
                        checked={mode === "create"}
                        onChange={() => setMode("create")}
                      />
                      Create new table
                    </label>
                  </div>
                  {mode === "existing" ? (
                    <select
                      className={cn("field-select", "mt-2 w-full")}
                      value={targetTable}
                      onChange={(event) => setTargetTable(event.target.value)}
                      aria-label="Target table"
                    >
                      <option value="">Choose table...</option>
                      {tables.map((table) => (
                        <option key={table.name} value={table.name}>
                          {table.name}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      className={cn("field-input", "mt-2 w-full")}
                      value={newTableName}
                      onChange={(event) => setNewTableName(event.target.value)}
                      placeholder="New table name"
                      aria-label="New table name"
                    />
                  )}
                  {mode === "existing" && targetColumns.length > 0 && (
                    <label className="mt-2 flex items-center gap-1.5 text-[10.5px] text-muted">
                      Upsert on
                      <select
                        className="field-select"
                        value={upsertKey}
                        onChange={(event) => setUpsertKey(event.target.value)}
                        aria-label="Upsert key"
                      >
                        <option value="">Insert only</option>
                        {targetColumns.map((column) => (
                          <option key={column} value={column}>
                            {column}
                          </option>
                        ))}
                      </select>
                    </label>
                  )}
                </div>
                <div className="card">
                  <span className="field-label">Preview</span>
                  <div className="mt-2 text-[11.5px] text-muted">
                    <div>
                      {preview.total_rows.toLocaleString()} row
                      {preview.total_rows === 1 ? "" : "s"} ·{" "}
                      {preview.columns.length} source columns
                    </div>
                    <div className="mt-1 text-[10.5px] text-muted-2">
                      Showing the first {Math.min(preview.rows.length, 20)} rows.
                      {mode === "existing" && mappedCount > 0
                        ? ` ${mappedCount} column${mappedCount === 1 ? "" : "s"} mapped.`
                        : ""}
                    </div>
                  </div>
                </div>
              </div>

              {mode === "existing" ? (
                <section className="mt-4">
                  <h3 className={cn("field-label", "mb-2")}>Column mapping</h3>
                  <div className="overflow-hidden rounded-lg border border-line">
                    {preview.columns.map((column) => (
                      <div
                        key={column.source}
                        className="flex items-center gap-3 border-b border-line/60 px-3 py-2 last:border-b-0"
                      >
                        <div className="min-w-0 flex-1">
                          <div className="truncate font-mono text-[11.5px] text-fg">
                            {column.source}
                          </div>
                          <div className="text-[10px] text-muted-2">
                            {column.inferred_type}
                          </div>
                        </div>
                        <span className="text-muted-2">→</span>
                        <select
                          className={cn("field-select", "w-[220px]")}
                          value={mapping[column.source] ?? ""}
                          onChange={(event) =>
                            setMapping((current) => ({
                              ...current,
                              [column.source]: event.target.value,
                            }))
                          }
                          aria-label={`Map ${column.source}`}
                        >
                          <option value="">Skip</option>
                          {targetColumns.map((target) => (
                            <option key={target} value={target}>
                              {target}
                            </option>
                          ))}
                        </select>
                      </div>
                    ))}
                  </div>
                </section>
              ) : (
                <section className="mt-4">
                  <h3 className={cn("field-label", "mb-2")}>New table columns</h3>
                  <div className="overflow-hidden rounded-lg border border-line">
                    {preview.columns.map((column) => (
                      <div
                        key={column.source}
                        className="flex items-center gap-3 border-b border-line/60 px-3 py-2 last:border-b-0"
                      >
                        <div className="min-w-0 flex-1 truncate font-mono text-[11.5px] text-fg">
                          {column.source}
                        </div>
                        <span className="text-muted-2">→</span>
                        <div className="flex w-[280px] items-center justify-between gap-2 font-mono text-[11px]">
                          <span className="truncate text-fg">
                            {column.suggested_name}
                          </span>
                          <span className="flex-none rounded-full border border-line bg-panel-3 px-1.5 py-0.5 text-[9.5px] text-muted">
                            {column.inferred_type}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                </section>
              )}

              <section className="mt-4">
                <div className="mb-2 flex items-center justify-between">
                  <h3 className="field-label">Data preview</h3>
                  <span className="text-[10px] text-muted-2">
                    {preview.columns.map((column) => column.source).join(" · ")}
                  </span>
                </div>
                <div className="max-h-[230px] overflow-auto rounded-lg border border-line">
                  <table className="w-full border-collapse text-left text-[11px]">
                    <thead className="sticky top-0 bg-panel-2 text-[9.5px] uppercase tracking-[0.08em] text-muted-2">
                      <tr>
                        {preview.columns.map((column) => (
                          <th
                            key={column.source}
                            className="whitespace-nowrap px-2.5 py-2 font-semibold"
                          >
                            {column.source}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {preview.rows.map((row: JsonObject, index) => (
                        <tr key={index} className="border-t border-line/50">
                          {preview.columns.map((column) => (
                            <td
                              key={column.source}
                              className="max-w-[260px] truncate whitespace-nowrap px-2.5 py-1.5 font-mono text-[10.5px] text-fg"
                              title={previewValue(row[column.source])}
                            >
                              {previewValue(row[column.source])}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </section>
            </>
          )}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <button className="btn btn-neutral" type="button" onClick={onClose} disabled={importing}>
            Cancel
          </button>
          <button
            className="btn btn-primary"
            type="button"
            onClick={() => void runImport()}
            disabled={importing || previewLoading || !preview}
            data-testid="import-run"
          >
            {importing ? "Importing..." : "Import"}
          </button>
        </div>
      </div>
    </div>
  );
}

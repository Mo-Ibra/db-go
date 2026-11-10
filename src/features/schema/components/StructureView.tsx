import { useEffect, useState } from "react";
import type { IndexInfo, TableDetails, TableStats } from "@/types";
import { copyText } from "@/lib/clipboard";
import { cn } from "@/lib/cn";

type StructureViewProps = {
  table: string;
  details: TableDetails | null;
  stats: TableStats | null;
  loading: boolean;
  onRefresh: () => void;
  onAlter: () => void;
  onCreateIndex: (name: string, columns: string[], unique: boolean) => Promise<void>;
  onDropIndex: (name: string) => Promise<void>;
  onOpenReferencedTable?: (table: string) => void;
};


function originLabel(origin: string): string {
  switch (origin) {
    case "c":
      return "index";
    case "u":
      return "unique constraint";
    case "pk":
      return "primary key";
    default:
      return origin;
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function Metric({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="rounded-lg border border-line bg-ink/50 px-3 py-2">
      <div className="text-[9.5px] font-bold uppercase tracking-[0.12em] text-muted-2">
        {label}
      </div>
      <div className="mt-1 font-mono text-[15px] text-fg">{value}</div>
      {hint && <div className="mt-0.5 text-[10px] text-muted-2">{hint}</div>}
    </div>
  );
}

function IndexRow({
  index,
  sizeBytes,
  busy,
  onDrop,
}: {
  index: IndexInfo;
  sizeBytes?: number | null;
  busy: boolean;
  onDrop: (name: string) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-3 border-b border-line/60 px-3 py-2 last:border-b-0">
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <span className="truncate font-mono text-[12px] text-fg">{index.name}</span>
          {index.unique && (
            <span className="rounded-full border border-accent/30 bg-accent/10 px-1.5 py-0.5 text-[9.5px] font-semibold uppercase tracking-[0.08em] text-accent">
              unique
            </span>
          )}
          <span className="rounded-full border border-line bg-panel-3 px-1.5 py-0.5 text-[9.5px] text-muted">
            {originLabel(index.origin)}
          </span>
        </div>
        <div className="mt-0.5 truncate font-mono text-[10.5px] text-muted">
          {index.columns.join(", ") || "(no columns)"}
          {index.partial ? " · partial" : ""}
          {sizeBytes !== undefined && sizeBytes !== null ? ` · ${formatBytes(sizeBytes)}` : ""}
        </div>
      </div>
      {index.sql ? (
        <button
          type="button"
          className="inline-flex h-6 flex-none items-center rounded-md border border-danger/30 px-2 text-[10.5px] font-medium text-[#fca5a5] hover:border-danger/60 hover:bg-danger/10 disabled:cursor-default disabled:opacity-40"
          onClick={() => onDrop(index.name)}
          disabled={busy}
        >
          {busy ? "Dropping..." : "Drop"}
        </button>
      ) : (
        <span className="flex-none text-[10px] text-muted-2">automatic</span>
      )}
    </div>
  );
}

export function StructureView({
  table,
  details,
  stats,
  loading,
  onRefresh,
  onAlter,
  onCreateIndex,
  onDropIndex,
  onOpenReferencedTable,
}: StructureViewProps) {
  const [copied, setCopied] = useState(false);
  const [showIndexForm, setShowIndexForm] = useState(false);
  const [indexName, setIndexName] = useState("");
  const [indexColumns, setIndexColumns] = useState("");
  const [indexUnique, setIndexUnique] = useState(false);
  const [indexBusy, setIndexBusy] = useState(false);
  const [dropping, setDropping] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  useEffect(() => {
    setShowIndexForm(false);
    setIndexName("");
    setIndexColumns("");
    setIndexUnique(false);
    setActionError(null);
  }, [table]);

  const copyDdl = async () => {
    if (!details) return;
    await copyText(details.ddl);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  const submitIndex = async () => {
    const name = indexName.trim();
    const columns = indexColumns
      .split(",")
      .map((column) => column.trim())
      .filter(Boolean);
    if (!name) {
      setActionError("Index name is required.");
      return;
    }
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
      setActionError("Index names may contain only letters, numbers, and underscores.");
      return;
    }
    if (columns.length === 0) {
      setActionError("Choose at least one column.");
      return;
    }
    setIndexBusy(true);
    setActionError(null);
    try {
      await onCreateIndex(name, columns, indexUnique);
      setShowIndexForm(false);
      setIndexName("");
      setIndexColumns("");
      setIndexUnique(false);
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIndexBusy(false);
    }
  };

  const dropIndex = async (name: string) => {
    setDropping(name);
    setActionError(null);
    try {
      await onDropIndex(name);
    } catch (cause) {
      setActionError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setDropping(null);
    }
  };

  if (!details) {
    return (
      <div className="grid flex-1 place-items-center text-xs text-muted">
        {loading ? "Loading structure..." : "Select a table to inspect its structure."}
      </div>
    );
  }

  return (
    <div className="min-h-0 flex-1 overflow-auto">
      <div className="mx-auto flex w-full max-w-[980px] flex-col gap-4 p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold tracking-tight">Table structure</h2>
            <p className="mt-0.5 text-[11px] text-muted">
              {details.columns.length} columns · {details.indexes.length} indexes ·{" "}
              {details.foreign_keys.length} foreign keys
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button className="btn btn-ghost" onClick={onRefresh} disabled={loading}>
              {loading ? "Refreshing..." : "Refresh"}
            </button>
            <button className="btn btn-primary" onClick={onAlter}>
              Alter table
            </button>
          </div>
        </div>

        {actionError && (
          <div className="rounded-md border border-danger/25 bg-danger/10 px-3 py-2 font-mono text-[11px] text-[#fecaca]">
            {actionError}
          </div>
        )}

        <section className="card">
          <div className="mb-2 flex items-center justify-between gap-2">
            <h3 className="field-label">Storage</h3>
            {stats?.row_count_estimated && (
              <span className="text-[10px] text-muted-2">rows from sqlite_stat1</span>
            )}
          </div>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            <Metric
              label="Rows"
              value={
                stats
                  ? stats.row_count_capped
                    ? `${stats.row_count.toLocaleString()}+`
                    : stats.row_count_estimated
                      ? `~${stats.row_count.toLocaleString()}`
                      : stats.row_count.toLocaleString()
                  : "—"
              }
              hint={stats?.row_count_capped ? "capped" : undefined}
            />
            <Metric
              label="Table size"
              value={stats?.size_bytes != null ? formatBytes(stats.size_bytes) : "—"}
            />
            <Metric
              label="Index size"
              value={stats?.index_bytes != null ? formatBytes(stats.index_bytes) : "—"}
            />
            <Metric
              label="Indexes"
              value={details.indexes.length.toLocaleString()}
            />
          </div>
          {stats && stats.size_bytes === null && (
            <p className="mt-2 text-[10.5px] text-muted-2">
              Page sizes are unavailable because the SQLite build has no <span className="font-mono">dbstat</span> table.
            </p>
          )}
        </section>

        <section className="card">
          <div className="mb-2 flex items-center justify-between gap-2">
            <h3 className="field-label">DDL</h3>
            <button className="btn btn-ghost" onClick={() => void copyDdl()} type="button">
              {copied ? "Copied" : "Copy DDL"}
            </button>
          </div>
          <pre className="max-h-[220px] overflow-auto whitespace-pre-wrap rounded-lg border border-line bg-ink p-3 font-mono text-[11.5px] leading-relaxed text-[#cbd5e1]">
            {details.ddl || "-- no DDL"}
          </pre>
        </section>

        <section className="card">
          <h3 className={cn("field-label", "mb-2")}>Columns</h3>
          <div className="overflow-hidden rounded-lg border border-line">
            <table className="w-full border-collapse text-left text-[11.5px]">
              <thead className="bg-panel-2 text-[10px] uppercase tracking-[0.08em] text-muted-2">
                <tr>
                  <th className="px-3 py-2 font-semibold">Name</th>
                  <th className="px-3 py-2 font-semibold">Type</th>
                  <th className="px-3 py-2 font-semibold">Constraints</th>
                </tr>
              </thead>
              <tbody>
                {details.columns.map((column) => (
                  <tr key={column.name} className="border-t border-line/60">
                    <td className="px-3 py-2 font-mono text-fg">{column.name}</td>
                    <td className="px-3 py-2 font-mono text-muted">
                      {column.type_name || "ANY"}
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex gap-1.5">
                        {column.primary_key && (
                          <span className="rounded-full border border-accent/30 bg-accent/10 px-1.5 py-0.5 text-[9.5px] font-semibold text-accent">
                            PK
                          </span>
                        )}
                        {column.not_null && (
                          <span className="rounded-full border border-line bg-panel-3 px-1.5 py-0.5 text-[9.5px] text-muted">
                            NOT NULL
                          </span>
                        )}
                        {details.foreign_keys
                          .filter((foreignKey) => foreignKey.column === column.name)
                          .map((foreignKey) => (
                            <span
                              key={`${column.name}-${foreignKey.referenced_table}`}
                              className="rounded-full border border-[#60a5fa]/30 bg-[#60a5fa]/10 px-1.5 py-0.5 text-[9.5px] text-[#93c5fd]"
                              title={`${foreignKey.referenced_table}.${foreignKey.referenced_column}`}
                            >
                              FK
                            </span>
                          ))}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section className="card">
          <div className="mb-2 flex items-center justify-between gap-2">
            <h3 className="field-label">Indexes</h3>
            <button
              className="btn btn-neutral"
              type="button"
              onClick={() => {
                setShowIndexForm((current) => !current);
                setActionError(null);
                if (!showIndexForm) {
                  setIndexName(`idx_${table}`);
                }
              }}
            >
              {showIndexForm ? "Cancel" : "New index"}
            </button>
          </div>

          {showIndexForm && (
            <div className="mb-3 rounded-lg border border-accent/25 bg-accent/[0.04] p-3">
              <div className="flex flex-wrap items-end gap-2">
                <label className="flex flex-col gap-1">
                  <span className="field-label">Name</span>
                  <input
                    className={cn("field-input", "w-[220px]")}
                    value={indexName}
                    onChange={(event) => setIndexName(event.target.value)}
                    placeholder={`idx_${table}_column`}
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className="field-label">Columns</span>
                  <input
                    className={cn("field-input", "w-[250px]")}
                    value={indexColumns}
                    onChange={(event) => setIndexColumns(event.target.value)}
                    placeholder="column_one, column_two"
                  />
                </label>
                <label className="flex h-8 items-center gap-1.5 text-[11px] text-muted">
                  <input
                    type="checkbox"
                    className="h-3.5 w-3.5 accent-accent"
                    checked={indexUnique}
                    onChange={(event) => setIndexUnique(event.target.checked)}
                  />
                  UNIQUE
                </label>
                <button
                  className="btn btn-primary"
                  type="button"
                  onClick={() => void submitIndex()}
                  disabled={indexBusy}
                >
                  {indexBusy ? "Creating..." : "Create index"}
                </button>
              </div>
              <p className="mt-2 text-[10.5px] text-muted-2">
                A backup and a migration file are created before the index is applied.
              </p>
            </div>
          )}

          <div className="overflow-hidden rounded-lg border border-line">
            {details.indexes.length === 0 ? (
              <div className="px-3 py-4 text-[11.5px] text-muted-2">No indexes.</div>
            ) : (
              details.indexes.map((index) => (
                <IndexRow
                  key={index.name}
                  index={index}
                  sizeBytes={
                    stats?.indexes.find((candidate) => candidate.name === index.name)
                      ?.size_bytes
                  }
                  busy={dropping === index.name}
                  onDrop={(name) => void dropIndex(name)}
                />
              ))
            )}
          </div>
        </section>

        <section className={cn("card", "mb-4")}>
          <h3 className={cn("field-label", "mb-2")}>Foreign keys</h3>
          {details.foreign_keys.length === 0 ? (
            <p className="text-[11.5px] text-muted-2">No foreign keys.</p>
          ) : (
            <div className="flex flex-col gap-2">
              {details.foreign_keys.map((foreignKey) => (
                <div
                  key={`${foreignKey.column}-${foreignKey.referenced_table}-${foreignKey.referenced_column}`}
                  className="flex items-center justify-between gap-3 rounded-lg border border-line bg-ink/60 px-3 py-2 font-mono text-[11.5px] text-fg"
                >
                  <div className="min-w-0">
                    <span>
                      {foreignKey.column} → {foreignKey.referenced_table}.
                      {foreignKey.referenced_column}
                    </span>
                    <span className="ml-2 text-muted-2">
                      ON DELETE {foreignKey.on_delete} · ON UPDATE {foreignKey.on_update}
                    </span>
                  </div>
                  {onOpenReferencedTable && (
                    <button
                      type="button"
                      className="flex-none rounded-md border border-line px-2 py-1 text-[10.5px] text-muted hover:border-accent/40 hover:text-accent"
                      onClick={() => onOpenReferencedTable(foreignKey.referenced_table)}
                      title={`Open ${foreignKey.referenced_table}`}
                    >
                      Open
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

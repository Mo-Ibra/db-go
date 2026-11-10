import { useEffect, useState } from "react";
import type { IndexInfo, TableDetails } from "../types";
import { copyText } from "../lib/clipboard";
import { cn } from "../lib/cn";

type StructureViewProps = {
  table: string;
  details: TableDetails | null;
  loading: boolean;
  onRefresh: () => void;
  onAlter: () => void;
  onCreateIndex: (name: string, columns: string[], unique: boolean) => Promise<void>;
  onDropIndex: (name: string) => Promise<void>;
};

const BTN =
  "inline-flex h-[30px] items-center justify-center rounded-[7px] border px-3 text-[12.5px] font-medium transition-colors disabled:cursor-default disabled:opacity-40";
const BTN_GHOST = `${BTN} border-transparent bg-transparent text-fg hover:border-line-strong hover:bg-panel-2`;
const BTN_NEUTRAL = `${BTN} border-line-strong bg-panel-2 text-fg hover:border-[#3a4a5a] hover:bg-panel-3`;
const BTN_PRIMARY = `${BTN} border-[#14b8a6] bg-linear-to-b from-accent to-[#14b8a6] font-bold text-[#03110f] hover:brightness-110`;
const INPUT =
  "h-8 rounded-md border border-line bg-ink px-2 text-xs text-fg outline-none placeholder:text-muted-2 focus:border-accent";
const LABEL = "text-[10px] font-bold uppercase tracking-[0.12em] text-muted-2";
const CARD = "rounded-xl border border-line bg-panel/60 p-4";

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

function IndexRow({
  index,
  busy,
  onDrop,
}: {
  index: IndexInfo;
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
  loading,
  onRefresh,
  onAlter,
  onCreateIndex,
  onDropIndex,
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
            <button className={BTN_GHOST} onClick={onRefresh} disabled={loading}>
              {loading ? "Refreshing..." : "Refresh"}
            </button>
            <button className={BTN_PRIMARY} onClick={onAlter}>
              Alter table
            </button>
          </div>
        </div>

        {actionError && (
          <div className="rounded-md border border-danger/25 bg-danger/10 px-3 py-2 font-mono text-[11px] text-[#fecaca]">
            {actionError}
          </div>
        )}

        <section className={CARD}>
          <div className="mb-2 flex items-center justify-between gap-2">
            <h3 className={LABEL}>DDL</h3>
            <button className={BTN_GHOST} onClick={() => void copyDdl()} type="button">
              {copied ? "Copied" : "Copy DDL"}
            </button>
          </div>
          <pre className="max-h-[220px] overflow-auto whitespace-pre-wrap rounded-lg border border-line bg-ink p-3 font-mono text-[11.5px] leading-relaxed text-[#cbd5e1]">
            {details.ddl || "-- no DDL"}
          </pre>
        </section>

        <section className={CARD}>
          <h3 className={cn(LABEL, "mb-2")}>Columns</h3>
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

        <section className={CARD}>
          <div className="mb-2 flex items-center justify-between gap-2">
            <h3 className={LABEL}>Indexes</h3>
            <button
              className={BTN_NEUTRAL}
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
                  <span className={LABEL}>Name</span>
                  <input
                    className={cn(INPUT, "w-[220px]")}
                    value={indexName}
                    onChange={(event) => setIndexName(event.target.value)}
                    placeholder={`idx_${table}_column`}
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className={LABEL}>Columns</span>
                  <input
                    className={cn(INPUT, "w-[250px]")}
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
                  className={BTN_PRIMARY}
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
                  busy={dropping === index.name}
                  onDrop={(name) => void dropIndex(name)}
                />
              ))
            )}
          </div>
        </section>

        <section className={cn(CARD, "mb-4")}>
          <h3 className={cn(LABEL, "mb-2")}>Foreign keys</h3>
          {details.foreign_keys.length === 0 ? (
            <p className="text-[11.5px] text-muted-2">No foreign keys.</p>
          ) : (
            <div className="flex flex-col gap-2">
              {details.foreign_keys.map((foreignKey) => (
                <div
                  key={`${foreignKey.column}-${foreignKey.referenced_table}-${foreignKey.referenced_column}`}
                  className="rounded-lg border border-line bg-ink/60 px-3 py-2 font-mono text-[11.5px] text-fg"
                >
                  {foreignKey.column} → {foreignKey.referenced_table}.
                  {foreignKey.referenced_column}
                  <span className="ml-2 text-muted-2">
                    ON DELETE {foreignKey.on_delete} · ON UPDATE {foreignKey.on_update}
                  </span>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

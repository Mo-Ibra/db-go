import { useState } from "react";
import type { CellValue, ColumnInfo, Row } from "../types";
import { cn } from "../lib/cn";
import { copyText } from "../lib/clipboard";

type RowDetailProps = {
  columns: ColumnInfo[];
  row: Row;
  index: number;
  onClose: () => void;
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function renderValue(cell: CellValue) {
  switch (cell.kind) {
    case "null":
      return <span className="italic text-muted-2">NULL</span>;
    case "integer":
      return <span className="tabular-nums text-int">{cell.value.toLocaleString()}</span>;
    case "real":
      return <span className="tabular-nums text-int">{cell.value}</span>;
    case "text":
      return (
        <span className="whitespace-pre-wrap break-words text-fg">{cell.value || "\u00a0"}</span>
      );
    case "blob":
      return <span className="text-blob">BLOB · {formatBytes(cell.size)}</span>;
  }
}

function plain(cell: CellValue): string | number | null {
  switch (cell.kind) {
    case "null":
      return null;
    case "integer":
    case "real":
      return cell.value;
    case "text":
      return cell.value;
    case "blob":
      return `<blob ${cell.size} bytes>`;
  }
}

function csvValue(cell: CellValue): string {
  const value = plain(cell);
  if (value === null) return "";
  const text = String(value);
  if (/[",\n\r]/.test(text)) {
    return `"${text.replace(/"/g, '""')}"`;
  }
  return text;
}

export function RowDetail({ columns, row, index, onClose }: RowDetailProps) {
  const [copied, setCopied] = useState<"json" | "csv" | null>(null);

  const copy = async (kind: "json" | "csv") => {
    const text =
      kind === "json"
        ? JSON.stringify(
            Object.fromEntries(columns.map((column, i) => [column.name, plain(row.cells[i])])),
            null,
            2,
          )
        : `${columns.map((column) => column.name).join(",")}\n${row.cells
            .map(csvValue)
            .join(",")}`;

    await copyText(text);
    setCopied(kind);
    window.setTimeout(() => setCopied(null), 1200);
  };

  const buttonClass =
    "inline-flex h-7 flex-1 items-center justify-center rounded-md border border-line-strong bg-panel-2 px-2 text-[11px] font-medium text-fg transition-colors hover:border-[#3a4a5a] hover:bg-panel-3";

  return (
    <aside className="flex w-[320px] flex-none flex-col border-l border-line bg-panel">
      <div className="flex h-12 flex-none items-center justify-between border-b border-line px-3">
        <div className="flex items-center gap-2">
          <span className="text-[10.5px] font-bold uppercase tracking-[0.14em] text-muted">
            Row
          </span>
          <span className="font-mono text-xs text-fg">{row.rowid ?? index + 1}</span>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close row detail"
          className="grid h-6 w-6 place-items-center rounded-md text-lg leading-none text-muted hover:bg-panel-2 hover:text-fg"
        >
          ×
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto p-3">
        <div className="flex flex-col gap-2.5">
          {columns.map((column, i) => (
            <div key={`${column.name}-${i}`}>
              <div className="flex items-baseline gap-1.5 font-mono text-[9.5px] font-semibold uppercase tracking-[0.08em] text-muted-2">
                <span>{column.name}</span>
                <span className="normal-case tracking-normal text-muted-2/70">
                  {column.type_name}
                </span>
              </div>
              <div className="mt-0.5 break-words font-mono text-[11.5px]">
                {renderValue(row.cells[i])}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="flex flex-none gap-2 border-t border-line p-3">
        <button
          type="button"
          className={cn(
            buttonClass,
            copied === "json" && "border-accent/50 text-accent",
          )}
          onClick={() => void copy("json")}
        >
          {copied === "json" ? "Copied" : "Copy JSON"}
        </button>
        <button
          type="button"
          className={cn(buttonClass, copied === "csv" && "border-accent/50 text-accent")}
          onClick={() => void copy("csv")}
        >
          {copied === "csv" ? "Copied" : "Copy CSV"}
        </button>
      </div>
    </aside>
  );
}

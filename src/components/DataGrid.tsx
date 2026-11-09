import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { CellValue, ColumnInfo, Row, SortSpec } from "../types";
import { cn } from "../lib/cn";

const COLUMN_WIDTH = 180;
const ROW_HEIGHT = 30;

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function cellTitle(cell: CellValue): string {
  switch (cell.kind) {
    case "null":
      return "NULL";
    case "integer":
    case "real":
      return String(cell.value);
    case "text":
      return cell.value;
    case "blob":
      return `BLOB (${formatBytes(cell.size)})`;
  }
}

function renderCell(cell: CellValue) {
  switch (cell.kind) {
    case "null":
      return <span className="italic text-muted-2">NULL</span>;
    case "integer":
      return <span className="tabular-nums text-int">{cell.value.toLocaleString()}</span>;
    case "real":
      return <span className="tabular-nums text-int">{cell.value}</span>;
    case "text":
      return <span className="text-fg">{cell.value || "\u00a0"}</span>;
    case "blob":
      return <span className="text-blob">BLOB · {formatBytes(cell.size)}</span>;
  }
}

type DataGridProps = {
  columns: ColumnInfo[];
  rows: Row[];
  sort?: SortSpec | null;
  onSort?: (column: string) => void;
  onRowClick?: (row: Row, index: number) => void;
  selectedIndex?: number | null;
  dirtyRowIds?: ReadonlySet<number>;
};

export function DataGrid({
  columns,
  rows,
  sort,
  onSort,
  onRowClick,
  selectedIndex,
  dirtyRowIds,
}: DataGridProps) {
  const headerRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => bodyRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  const width = Math.max(columns.length * COLUMN_WIDTH, 1);

  const syncScroll = () => {
    if (headerRef.current && bodyRef.current) {
      headerRef.current.scrollLeft = bodyRef.current.scrollLeft;
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div
        className="flex-none overflow-hidden border-b border-line-strong bg-panel"
        ref={headerRef}
      >
        <div className="flex" style={{ width }}>
          {columns.map((column, index) => {
            const active = sort?.column === column.name;
            const content = (
              <>
                <span className="flex max-w-full items-center gap-1 truncate font-mono text-[11.5px] font-semibold text-fg">
                  {column.name}
                  {active && (
                    <span className="text-accent">
                      {sort?.direction === "asc" ? "↑" : "↓"}
                    </span>
                  )}
                </span>
                <span className="text-[8.5px] font-semibold uppercase tracking-[0.07em] text-muted-2">
                  {column.type_name || "ANY"}
                  {column.primary_key ? " · PK" : ""}
                  {column.not_null ? " · NN" : ""}
                </span>
              </>
            );

            return onSort ? (
              <button
                type="button"
                key={`${column.name}-${index}`}
                onClick={() => onSort(column.name)}
                className="flex h-10 flex-none flex-col items-start justify-center gap-px overflow-hidden bg-transparent px-2.5 text-left hover:bg-panel-2"
                style={{ width: COLUMN_WIDTH }}
                title={`Sort by ${column.name}`}
                aria-sort={
                  active ? (sort?.direction === "asc" ? "ascending" : "descending") : "none"
                }
              >
                {content}
              </button>
            ) : (
              <div
                key={`${column.name}-${index}`}
                className="flex h-10 flex-none flex-col items-start justify-center gap-px overflow-hidden px-2.5"
                style={{ width: COLUMN_WIDTH }}
              >
                {content}
              </div>
            );
          })}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto" ref={bodyRef} onScroll={syncScroll}>
        {rows.length === 0 ? (
          <div className="grid h-[120px] place-items-center text-xs text-muted-2">No rows</div>
        ) : (
          <div
            style={{
              height: virtualizer.getTotalSize(),
              width,
              position: "relative",
            }}
          >
            {virtualizer.getVirtualItems().map((item) => {
              const row = rows[item.index];
              const selected = selectedIndex === item.index;
              const dirty =
                row.pendingKey !== undefined ||
                (row.rowid !== null && dirtyRowIds?.has(row.rowid));
              return (
                <div
                  className={cn(
                    "flex border-b border-line/60",
                    dirty
                      ? "bg-accent/[0.07] shadow-[inset_2px_0_0_#2dd4bf]"
                      : selected
                        ? "bg-accent/10"
                        : "hover:bg-accent/5",
                    onRowClick && "cursor-pointer",
                  )}
                  data-row={item.index}
                  key={item.key}
                  onClick={() => onRowClick?.(row, item.index)}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    height: item.size,
                    width,
                    transform: `translateY(${item.start}px)`,
                  }}
                >
                  {row.cells.map((cell, index) => (
                    <div
                      className="flex flex-none items-center overflow-hidden text-ellipsis whitespace-nowrap px-2.5 font-mono text-[11.5px]"
                      key={index}
                      style={{ width: COLUMN_WIDTH }}
                      title={cellTitle(cell)}
                    >
                      {renderCell(cell)}
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

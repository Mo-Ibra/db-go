import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { CellValue, ColumnInfo, Row } from "../types";

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
      return <span className="cell-null">NULL</span>;
    case "integer":
      return <span className="cell-number">{cell.value.toLocaleString()}</span>;
    case "real":
      return <span className="cell-number">{cell.value}</span>;
    case "text":
      return <span className="cell-text">{cell.value || "\u00a0"}</span>;
    case "blob":
      return <span className="cell-blob">BLOB · {formatBytes(cell.size)}</span>;
  }
}

export function DataGrid({ columns, rows }: { columns: ColumnInfo[]; rows: Row[] }) {
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
    <div className="grid">
      <div className="grid-header-viewport" ref={headerRef}>
        <div className="grid-header" style={{ width }}>
          {columns.map((column, index) => (
            <div
              className="grid-cell header-cell"
              key={`${column.name}-${index}`}
              style={{ width: COLUMN_WIDTH }}
            >
              <span className="col-name">{column.name}</span>
              <span className="col-type">
                {column.type_name || "ANY"}
                {column.primary_key ? " · PK" : ""}
                {column.not_null ? " · NN" : ""}
              </span>
            </div>
          ))}
        </div>
      </div>

      <div className="grid-body" ref={bodyRef} onScroll={syncScroll}>
        {rows.length === 0 ? (
          <div className="grid-empty">No rows</div>
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
              return (
                <div
                  className="grid-row"
                  key={item.key}
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
                      className="grid-cell"
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

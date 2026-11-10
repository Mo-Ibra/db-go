import { useEffect, useRef, useState } from "react";
import type {
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { CellValue, ColumnInfo, Row, SortSpec } from "../types";
import { cn } from "../lib/cn";
import { cellText, rowSelectionKey } from "../lib/cellValues";
import { readClipboardText } from "../lib/clipboard";

const COLUMN_WIDTH = 180;
const ROW_HEIGHT = 30;
const SELECTION_WIDTH = 34;

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

export type CellPosition = {
  rowIndex: number;
  columnIndex: number;
};

type DataGridProps = {
  columns: ColumnInfo[];
  rows: Row[];
  sort?: SortSpec | null;
  onSort?: (column: string) => void;
  onRowClick?: (row: Row, index: number) => void;
  selectedIndex?: number | null;
  dirtyRowIds?: ReadonlySet<number>;
  selectionKeys?: ReadonlySet<string>;
  allSelected?: boolean;
  someSelected?: boolean;
  onToggleRow?: (row: Row, index: number, event: ReactMouseEvent) => void;
  onToggleAll?: () => void;
  onCopy?: () => void;
  onPaste?: (text: string, cell: CellPosition) => void;
  onClipboardError?: (message: string) => void;
  onEditError?: (message: string) => void;
  canEditCell?: (row: Row, column: ColumnInfo) => boolean;
  onCommitCell?: (
    row: Row,
    rowIndex: number,
    column: ColumnInfo,
    columnIndex: number,
    text: string,
  ) => string | null;
};

export function DataGrid({
  columns,
  rows,
  sort,
  onSort,
  onRowClick,
  selectedIndex,
  dirtyRowIds,
  selectionKeys,
  allSelected = false,
  someSelected = false,
  onToggleRow,
  onToggleAll,
  onCopy,
  onPaste,
  onClipboardError,
  onEditError,
  canEditCell,
  onCommitCell,
}: DataGridProps) {
  const headerRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const closingEditor = useRef(false);
  const [activeCell, setActiveCell] = useState<CellPosition | null>(null);
  const [editing, setEditing] = useState<CellPosition | null>(null);
  const [editValue, setEditValue] = useState("");
  const [editError, setEditError] = useState<string | null>(null);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => bodyRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  const hasSelectionColumn = Boolean(onToggleRow);
  const width = Math.max(
    columns.length * COLUMN_WIDTH + (hasSelectionColumn ? SELECTION_WIDTH : 0),
    1,
  );

  useEffect(() => {
    setEditing(null);
    setEditError(null);
    closingEditor.current = false;
  }, [rows]);

  useEffect(() => {
    setActiveCell(null);
    setEditing(null);
    setEditError(null);
  }, [columns]);

  const syncScroll = () => {
    if (headerRef.current && bodyRef.current) {
      headerRef.current.scrollLeft = bodyRef.current.scrollLeft;
    }
  };

  const startEdit = (rowIndex: number, columnIndex: number) => {
    if (!onCommitCell) return;
    const row = rows[rowIndex];
    const column = columns[columnIndex];
    if (!row || !column) return;
    if (canEditCell && !canEditCell(row, column)) return;
    setActiveCell({ rowIndex, columnIndex });
    setEditing({ rowIndex, columnIndex });
    setEditValue(cellText(row.cells[columnIndex]));
    setEditError(null);
    closingEditor.current = false;
    bodyRef.current?.focus();
  };

  const tryCommit = (): string | null => {
    if (!editing) return null;
    const row = rows[editing.rowIndex];
    const column = columns[editing.columnIndex];
    if (!row || !column) return "Cell is no longer available.";
    const original = row.cells[editing.columnIndex];
    if (original && cellText(original) === editValue) {
      return null;
    }
    return (
      onCommitCell?.(row, editing.rowIndex, column, editing.columnIndex, editValue) ??
      null
    );
  };

  const handleEditorKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      closingEditor.current = true;
      setEditing(null);
      setEditError(null);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const error = tryCommit();
      if (error) {
        setEditError(error);
        onEditError?.(error);
        return;
      }
      closingEditor.current = true;
      setEditing(null);
      setEditError(null);
      if (editing) setActiveCell({ ...editing });
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      const error = tryCommit();
      if (error) {
        setEditError(error);
        onEditError?.(error);
        return;
      }
      if (!editing) return;
      const nextColumn = editing.columnIndex + (event.shiftKey ? -1 : 1);
      closingEditor.current = true;
      setEditing(null);
      if (nextColumn >= 0 && nextColumn < columns.length) {
        startEdit(editing.rowIndex, nextColumn);
      } else {
        startEdit(editing.rowIndex + (event.shiftKey ? -1 : 1), 0);
      }
    }
  };

  const handleEditorBlur = () => {
    if (closingEditor.current) {
      closingEditor.current = false;
      return;
    }
    const error = tryCommit();
    setEditing(null);
    setEditError(null);
    if (error) onEditError?.(error);
  };

  const moveActive = (rowDelta: number, columnDelta: number) => {
    if (rows.length === 0 || columns.length === 0) return;
    const current = activeCell ?? { rowIndex: 0, columnIndex: 0 };
    setActiveCell({
      rowIndex: Math.min(Math.max(current.rowIndex + rowDelta, 0), rows.length - 1),
      columnIndex: Math.min(
        Math.max(current.columnIndex + columnDelta, 0),
        columns.length - 1,
      ),
    });
  };

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (editing) return;
    const mod = event.metaKey || event.ctrlKey;
    if (mod && event.key.toLowerCase() === "c") {
      event.preventDefault();
      onCopy?.();
      return;
    }
    if (mod && event.key.toLowerCase() === "v") {
      if (!onPaste) return;
      if (!activeCell) {
        onClipboardError?.("Select a cell before pasting.");
        return;
      }
      event.preventDefault();
      const target = { ...activeCell };
      void readClipboardText().then((text) => {
        if (text) onPaste(text, target);
        else onClipboardError?.("Clipboard access is unavailable in this webview.");
      });
      return;
    }

    if (!activeCell) {
      if (
        (event.key === "Enter" || event.key === "F2") &&
        rows.length > 0 &&
        columns.length > 0
      ) {
        event.preventDefault();
        startEdit(0, 0);
      }
      return;
    }

    switch (event.key) {
      case "Enter":
      case "F2":
        event.preventDefault();
        startEdit(activeCell.rowIndex, activeCell.columnIndex);
        break;
      case "ArrowDown":
        event.preventDefault();
        moveActive(1, 0);
        break;
      case "ArrowUp":
        event.preventDefault();
        moveActive(-1, 0);
        break;
      case "ArrowRight":
        event.preventDefault();
        moveActive(0, 1);
        break;
      case "ArrowLeft":
        event.preventDefault();
        moveActive(0, -1);
        break;
      case "Escape":
        setActiveCell(null);
        break;
      default:
        break;
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div
        className="flex-none overflow-hidden border-b border-line-strong bg-panel"
        ref={headerRef}
      >
        <div className="flex" style={{ width }}>
          {hasSelectionColumn && (
            <div
              className="flex h-10 flex-none items-center justify-center border-r border-line/60 bg-panel"
              style={{ width: SELECTION_WIDTH }}
            >
              <input
                type="checkbox"
                className="h-3.5 w-3.5 accent-accent"
                checked={allSelected}
                ref={(element) => {
                  if (element) element.indeterminate = someSelected && !allSelected;
                }}
                onChange={onToggleAll}
                aria-label="Select all visible rows"
              />
            </div>
          )}
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

      <div
        className="min-h-0 flex-1 overflow-auto outline-none"
        ref={bodyRef}
        onScroll={syncScroll}
        onKeyDown={handleKeyDown}
        tabIndex={0}
      >
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
              const checked =
                selectionKeys?.has(rowSelectionKey(row, item.index)) ?? false;
              return (
                <div
                  className={cn(
                    "flex border-b border-line/60",
                    dirty
                      ? "bg-accent/[0.07] shadow-[inset_2px_0_0_#2dd4bf]"
                      : selected
                        ? "bg-accent/10"
                        : checked
                          ? "bg-accent/[0.05]"
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
                  {hasSelectionColumn && (
                    <div
                      className="flex flex-none items-center justify-center border-r border-line/60"
                      style={{ width: SELECTION_WIDTH }}
                      onClick={(event) => event.stopPropagation()}
                    >
                      <input
                        type="checkbox"
                        className="h-3.5 w-3.5 accent-accent"
                        checked={checked}
                        onClick={(event) => {
                          event.stopPropagation();
                          onToggleRow?.(row, item.index, event);
                        }}
                        onChange={() => {}}
                        aria-label={`Select row ${item.index + 1}`}
                      />
                    </div>
                  )}
                  {row.cells.map((cell, index) => {
                    const active =
                      activeCell?.rowIndex === item.index &&
                      activeCell.columnIndex === index;
                    const isEditing =
                      editing?.rowIndex === item.index &&
                      editing.columnIndex === index;
                    return (
                      <div
                        className={cn(
                          "relative flex flex-none items-center overflow-hidden text-ellipsis whitespace-nowrap px-2.5 font-mono text-[11.5px]",
                          active &&
                            !isEditing &&
                            "outline outline-1 -outline-offset-1 outline-accent/50",
                        )}
                        key={index}
                        data-cell={`${item.index}-${index}`}
                        style={{ width: COLUMN_WIDTH }}
                        title={isEditing ? undefined : cellTitle(cell)}
                        onClick={() => {
                          if (!onCommitCell && !onPaste) return;
                          bodyRef.current?.focus();
                          setActiveCell({ rowIndex: item.index, columnIndex: index });
                        }}
                        onDoubleClick={(event) => {
                          event.stopPropagation();
                          startEdit(item.index, index);
                        }}
                      >
                        {isEditing ? (
                          <input
                            autoFocus
                            value={editValue}
                            onChange={(event) => setEditValue(event.target.value)}
                            onKeyDown={handleEditorKeyDown}
                            onBlur={handleEditorBlur}
                            onClick={(event) => event.stopPropagation()}
                            onDoubleClick={(event) => event.stopPropagation()}
                            onFocus={(event) => event.currentTarget.select()}
                            spellCheck={false}
                            autoComplete="off"
                            className={cn(
                              "h-full w-full min-w-0 rounded-sm border bg-ink px-1.5 font-mono text-[11.5px] text-fg outline-none",
                              editError ? "border-danger" : "border-accent",
                            )}
                            title="Enter to save · Esc to cancel · type <NULL> for NULL"
                            aria-label={`Edit ${columns[index]?.name ?? "cell"}`}
                          />
                        ) : (
                          renderCell(cell)
                        )}
                      </div>
                    );
                  })}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { api } from "./lib/api";
import type {
  BackupInfo,
  CellValue,
  ColumnInfo,
  CountResult,
  D1ExportResult,
  DbInfo,
  ExportFormat,
  FilterOp,
  FilterSpec,
  ForeignKeyInfo,
  ImportResult,
  JsonObject,
  MigrationInfo,
  Page,
  QueryPlan,
  QueryResult,
  Row,
  RowUpdate,
  SchemaChange,
  SchemaGraph,
  SchemaMap,
  SortSpec,
  TableDetails,
  TableInfo,
  TableStats,
} from "./types";
import { DataGrid } from "./components/DataGrid";
import { RowDetail } from "./components/RowDetail";
import { RowEditor } from "./components/RowEditor";
import { StructureView } from "./components/StructureView";
import { AlterTableDialog } from "./components/AlterTableDialog";
import { MigrationsDialog } from "./components/MigrationsDialog";
import { BackupsDialog } from "./components/BackupsDialog";
import { ImportDialog } from "./components/ImportDialog";
import { D1ExportDialog } from "./components/D1ExportDialog";
import { QueryPlanPanel } from "./components/QueryPlanPanel";
import { TableDesigner } from "./components/TableDesigner";
import { cn } from "./lib/cn";
import {
  cellToValue,
  clipboardCellText,
  parseCellInput,
  parseClipboardRows,
  rowSelectionKey,
  valueToCell,
} from "./lib/cellValues";
import { copyText } from "./lib/clipboard";

const SqlEditor = lazy(() =>
  import("./components/SqlEditor").then((module) => ({ default: module.SqlEditor })),
);

const DiagramView = lazy(() =>
  import("./components/DiagramView").then((module) => ({ default: module.DiagramView })),
);

const PAGE_SIZES = [100, 200, 500, 1000];
const QUERY_LIMITS = [100, 500, 1000, 5000];
const QUERY_HISTORY_KEY = "dbgo:query-history";
const RECENT_DATABASES_KEY = "dbgo:recent-databases";
const RECENT_DATABASE_LIMIT = 10;

type RecentDatabase = {
  path: string;
  name: string;
  openedAt: number;
};

function basename(path: string): string {
  return (
    path
      .split(/[\\/]/)
      .filter((part) => part && part !== ".")
      .pop() ?? path
  );
}

function loadRecentDatabases(): RecentDatabase[] {
  try {
    const raw = localStorage.getItem(RECENT_DATABASES_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (item): item is RecentDatabase =>
          item !== null &&
          typeof item === "object" &&
          typeof (item as RecentDatabase).path === "string" &&
          typeof (item as RecentDatabase).name === "string" &&
          typeof (item as RecentDatabase).openedAt === "number",
      )
      .slice(0, RECENT_DATABASE_LIMIT);
  } catch {
    return [];
  }
}

function persistRecentDatabases(entries: RecentDatabase[]) {
  try {
    localStorage.setItem(RECENT_DATABASES_KEY, JSON.stringify(entries));
  } catch {
    // Recent databases are a convenience; ignore storage failures.
  }
}

function formatRecentOpened(ms: number): string {
  const date = new Date(ms);
  const now = new Date();
  if (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  ) {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
}

const FILTER_OPS: { value: FilterOp; label: string }[] = [
  { value: "contains", label: "contains" },
  { value: "eq", label: "=" },
  { value: "neq", label: "≠" },
  { value: "gt", label: ">" },
  { value: "gte", label: "≥" },
  { value: "lt", label: "<" },
  { value: "lte", label: "≤" },
  { value: "is_null", label: "is null" },
  { value: "not_null", label: "is not null" },
];

const BTN_BASE =
  "inline-flex items-center justify-center rounded-[7px] border text-[12.5px] font-medium transition-colors disabled:cursor-default disabled:opacity-40";
const BTN = `${BTN_BASE} h-[30px] px-3`;
const BTN_LARGE = `${BTN_BASE} h-[38px] px-5 text-[13px]`;
const BTN_SM = `${BTN_BASE} h-7 px-2.5 text-[11.5px]`;
const BTN_PRIMARY_COLOR =
  "border-[#14b8a6] bg-linear-to-b from-accent to-[#14b8a6] font-bold text-[#03110f] hover:brightness-110";
const BTN_PRIMARY = `${BTN} ${BTN_PRIMARY_COLOR}`;
const BTN_SM_PRIMARY = `${BTN_SM} ${BTN_PRIMARY_COLOR}`;
const BTN_NEUTRAL = `${BTN} border-line-strong bg-panel-2 text-fg hover:border-[#3a4a5a] hover:bg-panel-3`;
const BTN_LARGE_PRIMARY = `${BTN_LARGE} ${BTN_PRIMARY_COLOR}`;
const BTN_LARGE_NEUTRAL = `${BTN_LARGE} border-line-strong bg-panel-2 text-fg hover:border-[#3a4a5a] hover:bg-panel-3`;
const BTN_GHOST = `${BTN} border-transparent bg-transparent text-fg hover:border-line-strong hover:bg-panel-2`;
const BTN_SM_GHOST = `${BTN_SM} border-transparent bg-transparent text-fg hover:border-line-strong hover:bg-panel-2`;
const BADGE =
  "rounded-full border border-line bg-panel-3 px-1.5 py-0.5 text-[10.5px] text-muted tabular-nums";
const BADGE_SUBTLE =
  "rounded-full border border-line/70 bg-transparent px-1.5 py-0.5 text-[10.5px] text-muted tabular-nums";
const SELECT =
  "h-7 rounded-md border border-line bg-panel-2 px-1.5 text-xs text-fg outline-none focus:border-accent";

type PendingInsert = {
  key: string;
  values: JsonObject;
};

type PendingUpdate = {
  rowid: number;
  values: JsonObject;
  before: JsonObject;
};

type RowEditorState =
  | { mode: "insert"; row: Row | null }
  | { mode: "update"; row: Row };

function makePendingKey(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function isBlobColumn(column: ColumnInfo): boolean {
  return column.type_name.toUpperCase().includes("BLOB");
}

function rowToValues(columns: ColumnInfo[], row: Row): JsonObject {
  const values: JsonObject = {};
  columns.forEach((column, index) => {
    if (isBlobColumn(column)) return;
    values[column.name] = cellToValue(row.cells[index]);
  });
  return values;
}

function valuesEqual(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function applyValuesToRow(columns: ColumnInfo[], row: Row, values: JsonObject): Row {
  return {
    ...row,
    cells: columns.map((column, index) => {
      if (column.primary_key || isBlobColumn(column) || !(column.name in values)) {
        return row.cells[index];
      }
      return valueToCell(values[column.name]);
    }),
  };
}

function pendingInsertToRow(columns: ColumnInfo[], insert: PendingInsert): Row {
  return {
    rowid: null,
    pendingKey: insert.key,
    cells: columns.map((column) => valueToCell(insert.values[column.name])),
  };
}

function quoteIdent(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

function isNullOp(op: FilterOp): boolean {
  return op === "is_null" || op === "not_null";
}

function syncFilterColumn(
  current: { column: string; op: FilterOp; value: string },
  columns: ColumnInfo[],
) {
  if (columns.some((column) => column.name === current.column)) return current;
  return { ...current, column: columns[0]?.name ?? "" };
}

export default function App() {
  const [db, setDb] = useState<DbInfo | null>(null);
  const [recentDatabases, setRecentDatabases] = useState<RecentDatabase[]>(
    loadRecentDatabases,
  );
  const [missingRecents, setMissingRecents] = useState<Set<string>>(new Set());
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [filterText, setFilterText] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [columns, setColumns] = useState<ColumnInfo[]>([]);
  const [foreignKeys, setForeignKeys] = useState<ForeignKeyInfo[]>([]);
  const [page, setPage] = useState<Page | null>(null);
  const [pageSize, setPageSize] = useState(200);
  const [history, setHistory] = useState<(number | null)[]>([null]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [pageIndex, setPageIndex] = useState(0);
  const [count, setCount] = useState<CountResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [showDesigner, setShowDesigner] = useState(false);
  const [view, setView] = useState<"data" | "structure" | "sql" | "diagram">("data");
  const [sqlText, setSqlText] = useState("");
  const [schema, setSchema] = useState<SchemaMap>({});
  const [schemaGraph, setSchemaGraph] = useState<SchemaGraph | null>(null);
  const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
  const [queryRunning, setQueryRunning] = useState(false);
  const [queryPlan, setQueryPlan] = useState<QueryPlan | null>(null);
  const [explainRunning, setExplainRunning] = useState(false);
  const [queryLimit, setQueryLimit] = useState(1000);
  const [queryHistory, setQueryHistory] = useState<string[]>(() => {
    try {
      const raw = localStorage.getItem(QUERY_HISTORY_KEY);
      if (!raw) return [];
      const parsed: unknown = JSON.parse(raw);
      return Array.isArray(parsed)
        ? parsed.filter((item): item is string => typeof item === "string")
        : [];
    } catch {
      return [];
    }
  });
  const [sort, setSort] = useState<SortSpec | null>(null);
  const [dataFilter, setDataFilter] = useState<FilterSpec | null>(null);
  const [filterDraft, setFilterDraft] = useState<{
    column: string;
    op: FilterOp;
    value: string;
  }>({ column: "", op: "contains", value: "" });
  const [selectedRow, setSelectedRow] = useState<{ row: Row; index: number } | null>(null);
  const [selectionKeys, setSelectionKeys] = useState<Set<string>>(new Set());
  const [lastSelectedIndex, setLastSelectedIndex] = useState<number | null>(null);
  const [pendingInserts, setPendingInserts] = useState<PendingInsert[]>([]);
  const [pendingUpdates, setPendingUpdates] = useState<PendingUpdate[]>([]);
  const [pendingDeletes, setPendingDeletes] = useState<number[]>([]);
  const [rowEditor, setRowEditor] = useState<RowEditorState | null>(null);
  const [saving, setSaving] = useState(false);
  const [tableDetails, setTableDetails] = useState<TableDetails | null>(null);
  const [tableStats, setTableStats] = useState<TableStats | null>(null);
  const [detailsLoading, setDetailsLoading] = useState(false);
  const [showAlterTable, setShowAlterTable] = useState(false);
  const [showMigrations, setShowMigrations] = useState(false);
  const [migrations, setMigrations] = useState<MigrationInfo[]>([]);
  const [migrationsLoading, setMigrationsLoading] = useState(false);
  const [migrationsApplying, setMigrationsApplying] = useState(false);
  const [migrationsError, setMigrationsError] = useState<string | null>(null);
  const [showBackups, setShowBackups] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [showD1Export, setShowD1Export] = useState(false);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backupsLoading, setBackupsLoading] = useState(false);
  const [backupsError, setBackupsError] = useState<string | null>(null);
  const [creatingBackup, setCreatingBackup] = useState(false);
  const [restoringBackup, setRestoringBackup] = useState<string | null>(null);
  const [exportFormat, setExportFormat] = useState<ExportFormat>("csv");
  const [exporting, setExporting] = useState(false);
  const requestId = useRef(0);
  const detailsRequestId = useRef(0);
  const bootstrapped = useRef(false);
  const lastDataVersion = useRef<number | null>(null);
  const externalChangePending = useRef(false);

  const hasPending =
    pendingInserts.length + pendingUpdates.length + pendingDeletes.length > 0;

  const pendingRowIds = useMemo(
    () => new Set(pendingUpdates.map((update) => update.rowid)),
    [pendingUpdates],
  );

  const displayRows = useMemo(() => {
    if (!page) return [];
    const updates = new Map(pendingUpdates.map((update) => [update.rowid, update.values]));
    const deleted = new Set(pendingDeletes);
    const rows = page.rows
      .filter((row) => row.rowid === null || !deleted.has(row.rowid))
      .map((row) => {
        if (row.rowid === null) return row;
        const values = updates.get(row.rowid);
        return values ? applyValuesToRow(columns, row, values) : row;
      });
    return [...rows, ...pendingInserts.map((insert) => pendingInsertToRow(columns, insert))];
  }, [page, columns, pendingUpdates, pendingDeletes, pendingInserts]);

  const visibleSelectionKeys = useMemo(
    () => displayRows.map((row, index) => rowSelectionKey(row, index)),
    [displayRows],
  );
  const allSelected =
    visibleSelectionKeys.length > 0 &&
    visibleSelectionKeys.every((key) => selectionKeys.has(key));
  const someSelected = visibleSelectionKeys.some((key) => selectionKeys.has(key));

  const guardPending = useCallback((): boolean => {
    if (saving || restoringBackup !== null) {
      setNotice(
        restoringBackup !== null
          ? "A restore is in progress. Wait for it to finish."
          : "A save is in progress. Wait for it to finish.",
      );
      return true;
    }
    if (!hasPending) return false;
    setNotice("Save or discard your pending changes first.");
    return true;
  }, [hasPending, saving, restoringBackup]);

  const discardChanges = useCallback(() => {
    setPendingInserts([]);
    setPendingUpdates([]);
    setPendingDeletes([]);
    setRowEditor(null);
    setSelectedRow(null);
    setSelectionKeys(new Set());
    setLastSelectedIndex(null);
    setNotice("Discarded unsaved changes.");
  }, []);

  const clearSelection = useCallback(() => {
    setSelectionKeys(new Set());
    setLastSelectedIndex(null);
  }, []);

  const toggleRowSelection = useCallback(
    (row: Row, index: number, event: ReactMouseEvent) => {
      const key = rowSelectionKey(row, index);

      if (event.shiftKey && lastSelectedIndex !== null) {
        const start = Math.min(lastSelectedIndex, index);
        const end = Math.max(lastSelectedIndex, index);
        const range = displayRows
          .slice(start, end + 1)
          .map((candidate, offset) => rowSelectionKey(candidate, start + offset));
        setSelectionKeys((current) => {
          const next = new Set(current);
          range.forEach((rangeKey) => next.add(rangeKey));
          return next;
        });
        setLastSelectedIndex(index);
        return;
      }

      setSelectionKeys((current) => {
        const next = new Set(current);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        return next;
      });
      setLastSelectedIndex(index);
    },
    [displayRows, lastSelectedIndex],
  );

  const toggleAllSelection = useCallback(() => {
    setSelectionKeys((current) => {
      const next = new Set(current);
      if (allSelected) {
        visibleSelectionKeys.forEach((key) => next.delete(key));
      } else {
        visibleSelectionKeys.forEach((key) => next.add(key));
      }
      return next;
    });
    setLastSelectedIndex(null);
  }, [allSelected, visibleSelectionKeys]);

  const copySelectedRows = useCallback(async () => {
    const rows = displayRows.filter((row, index) =>
      selectionKeys.has(rowSelectionKey(row, index)),
    );
    if (rows.length === 0) {
      setNotice("Select rows to copy first.");
      return;
    }
    const sanitize = (value: string) =>
      value.replace(/\t/g, " ").replace(/\r?\n/g, " ");
    const header = columns.map((column) => sanitize(column.name)).join("\t");
    const lines = rows.map((row) =>
      row.cells.map((cell) => sanitize(clipboardCellText(cell))).join("\t"),
    );
    await copyText([header, ...lines].join("\n"));
    setNotice(`Copied ${rows.length} row(s)`);
  }, [displayRows, selectionKeys, columns]);

  const deleteSelectedRows = useCallback(() => {
    const rows = displayRows.filter((row, index) =>
      selectionKeys.has(rowSelectionKey(row, index)),
    );
    if (rows.length === 0) return;

    const insertKeys = new Set<string>();
    const rowids = new Set<number>();
    for (const row of rows) {
      if (row.pendingKey) insertKeys.add(row.pendingKey);
      else if (row.rowid !== null) rowids.add(row.rowid);
    }

    if (insertKeys.size > 0) {
      setPendingInserts((current) =>
        current.filter((insert) => !insertKeys.has(insert.key)),
      );
    }
    if (rowids.size > 0) {
      setPendingUpdates((current) =>
        current.filter((update) => !rowids.has(update.rowid)),
      );
      setPendingDeletes((current) => {
        const next = new Set(current);
        rowids.forEach((rowid) => next.add(rowid));
        return [...next];
      });
    }

    const parts = [];
    if (rowids.size > 0) parts.push(`staged ${rowids.size} row(s) for deletion`);
    if (insertKeys.size > 0) parts.push(`removed ${insertKeys.size} pending insert(s)`);
    setNotice(parts.join(" · "));
    clearSelection();
  }, [displayRows, selectionKeys, clearSelection]);

  const handleCellCommit = useCallback(
    (
      row: Row,
      _rowIndex: number,
      column: ColumnInfo,
      _columnIndex: number,
      text: string,
    ): string | null => {      let value: unknown;
      try {
        value = parseCellInput(column, text);
      } catch (cause) {
        return cause instanceof Error ? cause.message : String(cause);
      }

      if (row.pendingKey) {
        setPendingInserts((current) =>
          current.map((insert) =>
            insert.key === row.pendingKey
              ? { ...insert, values: { ...insert.values, [column.name]: value } }
              : insert,
          ),
        );
        return null;
      }
      if (row.rowid === null) return "This row cannot be edited.";

      const rowid = row.rowid;
      setPendingUpdates((current) => {
        const existing = current.find((update) => update.rowid === rowid);
        const before = existing?.before ?? rowToValues(columns, row);
        const merged: JsonObject = {
          ...(existing?.values ?? {}),
          [column.name]: value,
        };
        const changed: JsonObject = {};
        for (const [name, candidate] of Object.entries(merged)) {
          if (!valuesEqual(candidate, before[name])) changed[name] = candidate;
        }
        if (Object.keys(changed).length === 0) {
          return current.filter((update) => update.rowid !== rowid);
        }
        if (existing) {
          return current.map((update) =>
            update.rowid === rowid ? { ...update, values: changed } : update,
          );
        }
        return [...current, { rowid, values: changed, before }];
      });
      return null;
    },
    [columns],
  );

  const canEditCell = useCallback(
    (row: Row, column: ColumnInfo) => {
      if (view !== "data") return false;
      if (column.primary_key || isBlobColumn(column)) return false;
      return row.pendingKey !== undefined || row.rowid !== null;
    },
    [view],
  );

  const pasteIntoCell = useCallback(
    (text: string, cell: { rowIndex: number; columnIndex: number }) => {
      const matrix = parseClipboardRows(text);
      if (matrix.length === 0) return;

      const targetColumns = columns.slice(cell.columnIndex);
      if (targetColumns.length === 0) return;

      const firstRow = matrix[0];
      const headerMatches =
        matrix.length > 1 &&
        firstRow.length > 0 &&
        targetColumns
          .slice(0, firstRow.length)
          .every(
            (column, index) =>
              firstRow[index]?.trim().toLowerCase() === column.name.toLowerCase(),
          );
      const dataRows = headerMatches ? matrix.slice(1) : matrix;
      if (dataRows.length === 0) return;

      const targetRows = displayRows.slice(
        cell.rowIndex,
        cell.rowIndex + dataRows.length,
      );
      if (targetRows.length === 0) return;

      const changes: {
        row: Row;
        rowid: number | null;
        pendingKey?: string;
        values: JsonObject;
      }[] = [];

      for (let rowIndex = 0; rowIndex < targetRows.length; rowIndex += 1) {
        const row = targetRows[rowIndex];
        const pastedRow = dataRows[rowIndex];
        if (!pastedRow) break;
        const values: JsonObject = {};
        for (
          let columnIndex = 0;
          columnIndex < Math.min(pastedRow.length, targetColumns.length);
          columnIndex += 1
        ) {
          const column = targetColumns[columnIndex];
          if (column.primary_key || isBlobColumn(column)) {
            setNotice(`Cannot paste into ${column.name}.`);
            return;
          }
          try {
            values[column.name] = parseCellInput(column, pastedRow[columnIndex]);
          } catch (cause) {
            setError(cause instanceof Error ? cause.message : String(cause));
            return;
          }
        }
        if (Object.keys(values).length > 0) {
          changes.push({
            row,
            rowid: row.rowid,
            pendingKey: row.pendingKey,
            values,
          });
        }
      }

      if (changes.length === 0) return;

      const insertChanges = new Map<string, JsonObject>();
      for (const change of changes) {
        if (!change.pendingKey) continue;
        insertChanges.set(change.pendingKey, {
          ...(insertChanges.get(change.pendingKey) ?? {}),
          ...change.values,
        });
      }
      if (insertChanges.size > 0) {
        setPendingInserts((current) =>
          current.map((insert) => {
            const values = insertChanges.get(insert.key);
            return values
              ? { ...insert, values: { ...insert.values, ...values } }
              : insert;
          }),
        );
      }

      const existingChanges = changes.filter(
        (change) => !change.pendingKey && change.rowid !== null,
      );
      if (existingChanges.length > 0) {
        setPendingUpdates((current) => {
          let next = current;
          for (const change of existingChanges) {
            const rowid = change.rowid;
            if (rowid === null) continue;
            const currentUpdate = next.find((update) => update.rowid === rowid);
            const before =
              currentUpdate?.before ?? rowToValues(columns, change.row);
            const merged: JsonObject = {
              ...(currentUpdate?.values ?? {}),
              ...change.values,
            };
            const changed: JsonObject = {};
            for (const [name, candidate] of Object.entries(merged)) {
              if (!valuesEqual(candidate, before[name])) changed[name] = candidate;
            }
            if (Object.keys(changed).length === 0) {
              next = next.filter((update) => update.rowid !== rowid);
            } else if (currentUpdate) {
              next = next.map((update) =>
                update.rowid === rowid ? { ...update, values: changed } : update,
              );
            } else {
              next = [...next, { rowid, values: changed, before }];
            }
          }
          return next;
        });
      }

      setNotice(`Pasted ${changes.length} row(s) into pending changes`);
    },
    [columns, displayRows],
  );

  const pagedMode = sort !== null || dataFilter !== null;

  const loadTable = useCallback(
    async (table: string, cursor: number | null, size = pageSize): Promise<Page | null> => {
      const id = ++requestId.current;
      setLoading(true);
      setError(null);
      try {
        const [nextColumns, nextForeignKeys, result] = await Promise.all([
          api.describeTable(table),
          api.foreignKeys(table),
          api.loadPage(table, cursor, "next", size),
        ]);
        if (id !== requestId.current) return null;

        setColumns(nextColumns);
        setForeignKeys(nextForeignKeys);
        setPage(result);
        setSelected(table);
        setPageIndex(0);
        setSelectedRow(null);
        setSelectionKeys(new Set());
        setLastSelectedIndex(null);
        setFilterDraft((current) => syncFilterColumn(current, nextColumns));

        void api
          .countRows(table)
          .then((nextCount) => {
            if (id === requestId.current) setCount(nextCount);
          })
          .catch(() => {
            if (id === requestId.current) setCount(null);
          });

        return result;
      } catch (cause) {
        if (id === requestId.current) {
          setError(String(cause));
          setPage(null);
          setForeignKeys([]);
        }
        return null;
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    },
    [pageSize],
  );

  const loadOffsetPage = useCallback(
    async (
      table: string,
      index: number,
      activeSort: SortSpec | null,
      activeFilter: FilterSpec | null,
      size = pageSize,
    ): Promise<Page | null> => {
      const id = ++requestId.current;
      setLoading(true);
      setError(null);
      try {
        const [nextColumns, nextForeignKeys, result] = await Promise.all([
          api.describeTable(table),
          api.foreignKeys(table),
          api.loadTablePage(table, activeFilter, activeSort, index * size, size),
        ]);
        if (id !== requestId.current) return null;

        setColumns(nextColumns);
        setForeignKeys(nextForeignKeys);
        setPage(result);
        setSelected(table);
        setPageIndex(index);
        setSelectedRow(null);
        setSelectionKeys(new Set());
        setLastSelectedIndex(null);
        setFilterDraft((current) => syncFilterColumn(current, nextColumns));

        if (activeFilter) {
          setCount(null);
        } else {
          void api
            .countRows(table)
            .then((nextCount) => {
              if (id === requestId.current) setCount(nextCount);
            })
            .catch(() => {
              if (id === requestId.current) setCount(null);
            });
        }

        return result;
      } catch (cause) {
        if (id === requestId.current) {
          setError(String(cause));
          setPage(null);
          setForeignKeys([]);
        }
        return null;
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    },
    [pageSize],
  );

  const loadDetails = useCallback(async (table: string) => {
    const id = ++detailsRequestId.current;
    setDetailsLoading(true);
    setError(null);
    try {
      const [details, stats] = await Promise.all([
        api.tableDetails(table),
        api.tableStats(table),
      ]);
      if (id !== detailsRequestId.current) return;
      setTableDetails(details);
      setTableStats(stats);
    } catch (cause) {
      if (id === detailsRequestId.current) {
        setError(String(cause));
        setTableDetails(null);
        setTableStats(null);
      }
    } finally {
      if (id === detailsRequestId.current) setDetailsLoading(false);
    }
  }, []);

  const loadMigrations = useCallback(async () => {
    setMigrationsLoading(true);
    setMigrationsError(null);
    try {
      setMigrations(await api.listMigrations());
    } catch (cause) {
      setMigrationsError(String(cause));
    } finally {
      setMigrationsLoading(false);
    }
  }, []);

  const loadBackups = useCallback(async () => {
    setBackupsLoading(true);
    setBackupsError(null);
    try {
      setBackups(await api.listBackups());
    } catch (cause) {
      setBackupsError(String(cause));
    } finally {
      setBackupsLoading(false);
    }
  }, []);

  const rememberDatabase = useCallback((info: DbInfo) => {
    const fileName = basename(info.path);
    const name = info.project_path ? basename(info.project_path) : fileName;
    const entry: RecentDatabase = {
      path: info.path,
      name: name || fileName,
      openedAt: Date.now(),
    };
    setRecentDatabases((current) => {
      const next = [
        entry,
        ...current.filter((item) => item.path !== entry.path),
      ].slice(0, RECENT_DATABASE_LIMIT);
      persistRecentDatabases(next);
      return next;
    });
  }, []);

  useEffect(() => {
    if (recentDatabases.length === 0) {
      setMissingRecents(new Set());
      return;
    }
    let cancelled = false;
    void Promise.all(
      recentDatabases.map(async (entry) => ({
        path: entry.path,
        exists: await api.pathExists(entry.path).catch(() => true),
      })),
    ).then((results) => {
      if (cancelled) return;
      setMissingRecents(
        new Set(
          results
            .filter((item) => !item.exists)
            .map((item) => item.path),
        ),
      );
    });
    return () => {
      cancelled = true;
    };
  }, [recentDatabases]);

  const applyDbInfo = useCallback((info: DbInfo) => {
    setDb(info);
    setTables(info.tables);
    setSelected(null);
    setColumns([]);
    setForeignKeys([]);
    setPage(null);
    setCount(null);
    setFilterText("");
    setHistory([null]);
    setHistoryIndex(0);
    setPageIndex(0);
    setView("data");
    setSqlText("");
    setSchema({});
    setSchemaGraph(null);
    setQueryResult(null);
    setQueryRunning(false);
    setQueryPlan(null);
    setExplainRunning(false);
    setSort(null);
    setDataFilter(null);
    setFilterDraft({ column: "", op: "contains", value: "" });
    setSelectedRow(null);
    setSelectionKeys(new Set());
    setLastSelectedIndex(null);
    setPendingInserts([]);
    setPendingUpdates([]);
    setPendingDeletes([]);
    setRowEditor(null);
    setSaving(false);
    setExporting(false);
    externalChangePending.current = false;
    detailsRequestId.current += 1;
    setTableDetails(null);
    setTableStats(null);
    setDetailsLoading(false);
    setShowAlterTable(false);
    setShowMigrations(false);
    setMigrations([]);
    setMigrationsError(null);
    setMigrationsApplying(false);
    setMigrationsLoading(false);
    setShowBackups(false);
    setShowImport(false);
    setShowD1Export(false);
    setBackups([]);
    setBackupsError(null);
    setBackupsLoading(false);
    setCreatingBackup(false);
    setRestoringBackup(null);
    setNotice(null);
    setShowDesigner(false);
  }, []);

  const openPath = useCallback(
    async (path: string): Promise<boolean> => {
      setLoading(true);
      setError(null);
      try {
        const info = await api.openDatabase(path);
        applyDbInfo(info);
        rememberDatabase(info);
        if (info.agent_files_error) {
          setNotice(`Agent files: ${info.agent_files_error}`);
        } else if (info.agent_files?.created) {
          setNotice(`Agent files created in ${info.agent_files.directory}`);
        }
        void Promise.all([api.schemaMap(), api.schemaGraph()])
          .then(([nextSchema, nextGraph]) => {
            setSchema(nextSchema);
            setSchemaGraph(nextGraph);
          })
          .catch(() => {
            setSchema({});
            setSchemaGraph(null);
          });

        const firstTable = info.tables[0]?.name;
        if (firstTable) void loadTable(firstTable, null);
        return true;
      } catch (cause) {
        setError(String(cause));
        return false;
      } finally {
        setLoading(false);
      }
    },
    [applyDbInfo, loadTable, rememberDatabase],
  );

  const openRecentDatabase = useCallback(
    async (entry: RecentDatabase) => {
      if (missingRecents.has(entry.path)) return;
      const opened = await openPath(entry.path);
      if (!opened) {
        setMissingRecents((current) => {
          const next = new Set(current);
          next.add(entry.path);
          return next;
        });
      }
    },
    [missingRecents, openPath],
  );

  const removeRecentDatabase = useCallback((path: string) => {
    setRecentDatabases((current) => {
      const next = current.filter((item) => item.path !== path);
      persistRecentDatabases(next);
      return next;
    });
    setMissingRecents((current) => {
      const next = new Set(current);
      next.delete(path);
      return next;
    });
  }, []);

  const clearRecentDatabases = useCallback(() => {
    setRecentDatabases([]);
    setMissingRecents(new Set());
    persistRecentDatabases([]);
  }, []);

  const createNewDatabase = useCallback(async () => {
    if (guardPending()) return;
    try {
      const path = await open({
        directory: true,
        multiple: false,
        title: "Choose a folder for the database project",
      });
      if (!path || Array.isArray(path)) return;

      setLoading(true);
      setError(null);
      try {
        let info: DbInfo;
        try {
          info = await api.createProject(path, false);
        } catch (cause) {
          const message = String(cause);
          if (!message.toLowerCase().includes("already exists")) throw cause;
          const replace = await confirm(
            "A database project already exists in that folder. Replace it? A timestamped backup will be kept.",
            { title: "Replace project?", kind: "warning" },
          );
          if (!replace) return;
          info = await api.createProject(path, true);
        }

        applyDbInfo(info);
        rememberDatabase(info);
        void api.schemaGraph().then(setSchemaGraph).catch(() => setSchemaGraph(null));
        if (info.backup_path) {
          setNotice(`Existing database backed up to ${info.backup_path}`);
        }
        setShowDesigner(true);
      } finally {
        setLoading(false);
      }
    } catch (cause) {
      setError(String(cause));
    }
  }, [applyDbInfo, guardPending]);

  const handleTableCreated = useCallback(
    async (table: string) => {
      setShowDesigner(false);
      try {
        const [nextTables, nextSchema, nextGraph] = await Promise.all([
          api.listTables(),
          api.schemaMap(),
          api.schemaGraph(),
        ]);
        setTables(nextTables);
        setSchema(nextSchema);
        setSchemaGraph(nextGraph);
        setView("data");
        setSort(null);
        setDataFilter(null);
        setPageIndex(0);
        setSelectedRow(null);
        setHistory([null]);
        setHistoryIndex(0);
        void loadTable(table, null);
      } catch (cause) {
        setError(String(cause));
      }
    },
    [loadTable],
  );

  const openDb = useCallback(async () => {
    if (guardPending()) return;
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: "SQLite database", extensions: ["db", "sqlite", "sqlite3", "d1"] },
        ],
      });
      if (!path || Array.isArray(path)) return;
      await openPath(path);
    } catch (cause) {
      setError(String(cause));
    }
  }, [openPath, guardPending]);

  const selectTable = useCallback(
    (table: string) => {
      if (table === selected) return;
      if (guardPending()) return;
      setView("data");
      setSort(null);
      setDataFilter(null);
      setPageIndex(0);
      setSelectedRow(null);
      setFilterDraft((current) => ({ ...current, value: "" }));
      setPage(null);
      setColumns([]);
      setForeignKeys([]);
      setTableStats(null);
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);
      void loadTable(table, null);
    },
    [loadTable, selected, guardPending],
  );

  const toggleSort = useCallback(
    (column: string) => {
      if (!selected) return;
      if (guardPending()) return;
      const nextSort: SortSpec | null =
        sort?.column === column
          ? sort.direction === "asc"
            ? { column, direction: "desc" }
            : null
          : { column, direction: "asc" };

      setSort(nextSort);
      setPageIndex(0);
      setSelectedRow(null);
      setPage(null);

      if (nextSort || dataFilter) {
        void loadOffsetPage(selected, 0, nextSort, dataFilter);
      } else {
        setHistory([null]);
        setHistoryIndex(0);
        void loadTable(selected, null);
      }
    },
    [selected, sort, dataFilter, loadOffsetPage, loadTable, guardPending],
  );

  const applyFilter = useCallback(() => {
    if (!selected || !filterDraft.column) return;
    if (guardPending()) return;
    const nextFilter: FilterSpec = {
      column: filterDraft.column,
      op: filterDraft.op,
      value: isNullOp(filterDraft.op) ? null : filterDraft.value,
    };
    setDataFilter(nextFilter);
    setPageIndex(0);
    setSelectedRow(null);
    setPage(null);
    void loadOffsetPage(selected, 0, sort, nextFilter);
  }, [selected, filterDraft, sort, loadOffsetPage, guardPending]);

  const clearFilter = useCallback(() => {
    if (!selected) return;
    if (guardPending()) return;
    setDataFilter(null);
    setPageIndex(0);
    setSelectedRow(null);
    setPage(null);
    if (sort) {
      void loadOffsetPage(selected, 0, sort, null);
    } else {
      setHistory([null]);
      setHistoryIndex(0);
      void loadTable(selected, null);
    }
  }, [selected, sort, loadOffsetPage, loadTable, guardPending]);

  const runQuery = useCallback(
    async (text: string) => {
      const statement = text.trim();
      if (!statement) return;

      setSqlText(statement);
      setView("sql");
      setSelectedRow(null);
      setQueryPlan(null);
      setQueryRunning(true);
      setError(null);

      try {
        const result = await api.runQuery(statement, queryLimit);
        setQueryResult(result);
        setQueryHistory((current) => {
          const next = [statement, ...current.filter((item) => item !== statement)].slice(0, 20);
          try {
            localStorage.setItem(QUERY_HISTORY_KEY, JSON.stringify(next));
          } catch {
            // Ignore storage failures; history is a convenience.
          }
          return next;
        });
      } catch (cause) {
        setQueryResult(null);
        setError(String(cause));
      } finally {
        setQueryRunning(false);
      }
    },
    [queryLimit],
  );

  const explainQuery = useCallback(async (text: string) => {
    const statement = text.trim();
    if (!statement) return;

    setSqlText(statement);
    setView("sql");
    setSelectedRow(null);
    setExplainRunning(true);
    setError(null);
    try {
      setQueryPlan(await api.explainQuery(statement));
    } catch (cause) {
      setQueryPlan(null);
      setError(String(cause));
    } finally {
      setExplainRunning(false);
    }
  }, []);

  const changeSqlText = useCallback((value: string) => {
    setSqlText(value);
    setQueryPlan(null);
  }, []);

  const exportCurrent = useCallback(async () => {
    if (exporting) return;
    if (guardPending()) return;

    const isData = view === "data";
    const isSql = view === "sql";
    if (isData && !selected) return;
    if (isSql && !sqlText.trim()) return;
    if (!isData && !isSql) return;

    const base =
      ((isData ? selected : "query") ?? "export")
        .replace(/[^A-Za-z0-9_-]+/g, "_")
        .slice(0, 40) || "export";
    try {
      const path = await save({
        defaultPath: `${base}-${new Date().toISOString().slice(0, 10)}.${exportFormat}`,
        filters: [{ name: exportFormat.toUpperCase(), extensions: [exportFormat] }],
      });
      if (!path || Array.isArray(path)) return;

      setExporting(true);
      setError(null);
      const result = await api.exportData({
        table: isData ? selected : null,
        query: isSql ? sqlText : null,
        filter: isData ? dataFilter : null,
        sort: isData ? sort : null,
        format: exportFormat,
        path,
      });
      setNotice(
        `Exported ${result.rows.toLocaleString()} row(s) to ${result.path}`,
      );
    } catch (cause) {
      setError(String(cause));
    } finally {
      setExporting(false);
    }
  }, [
    exporting,
    guardPending,
    view,
    selected,
    sqlText,
    exportFormat,
    dataFilter,
    sort,
  ]);

  const nextPage = useCallback(async () => {
    if (!selected || !page) return;
    if (guardPending()) return;
    if (pagedMode) {
      if (!page.has_more) return;
      await loadOffsetPage(selected, pageIndex + 1, sort, dataFilter);
      return;
    }
    if (!page.has_more || page.next_cursor === null) return;
    const cursor = page.next_cursor;
    const result = await loadTable(selected, cursor);
    if (result) {
      const nextHistory = [...history.slice(0, historyIndex + 1), cursor];
      setHistory(nextHistory);
      setHistoryIndex(nextHistory.length - 1);
    }
  }, [
    selected,
    page,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    loadOffsetPage,
    loadTable,
    history,
    historyIndex,
    guardPending,
  ]);

  const prevPage = useCallback(async () => {
    if (!selected) return;
    if (guardPending()) return;
    if (pagedMode) {
      if (pageIndex <= 0) return;
      await loadOffsetPage(selected, pageIndex - 1, sort, dataFilter);
      return;
    }
    if (historyIndex <= 0) return;
    const nextIndex = historyIndex - 1;
    const result = await loadTable(selected, history[nextIndex]);
    if (result) setHistoryIndex(nextIndex);
  }, [
    selected,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    loadOffsetPage,
    history,
    historyIndex,
    loadTable,
    guardPending,
  ]);

  const refresh = useCallback(() => {
    if (guardPending()) return;
    if (view === "diagram") {
      void api
        .schemaGraph()
        .then(setSchemaGraph)
        .catch((cause) => setError(String(cause)));
      return;
    }
    if (view === "structure") {
      if (selected) void loadDetails(selected);
      return;
    }
    if (view === "sql") {
      if (sqlText.trim()) void runQuery(sqlText);
      return;
    }
    if (!selected) return;
    if (pagedMode) {
      void loadOffsetPage(selected, pageIndex, sort, dataFilter);
    } else {
      void loadTable(selected, history[historyIndex]);
    }
  }, [
    view,
    sqlText,
    runQuery,
    selected,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    loadOffsetPage,
    loadTable,
    history,
    historyIndex,
    guardPending,
    loadDetails,
  ]);

  const switchView = useCallback(
    (next: "data" | "structure" | "sql" | "diagram") => {
      if (next === "sql" && !sqlText.trim() && selected) {
        setSqlText(`SELECT * FROM ${quoteIdent(selected)} LIMIT 200;`);
      }
      if (next === "diagram" && !schemaGraph) {
        void api
          .schemaGraph()
          .then(setSchemaGraph)
          .catch((cause) => setError(String(cause)));
      }
      if (next === "structure" && selected) {
        void loadDetails(selected);
      }
      setSelectedRow(null);
      setView(next);
    },
    [sqlText, selected, schemaGraph, loadDetails],
  );

  const changePageSize = useCallback(
    (size: number) => {
      if (guardPending()) return;
      setPageSize(size);
      setPage(null);
      setCount(null);
      setPageIndex(0);
      setSelectedRow(null);
      if (!selected) return;
      if (pagedMode) {
        void loadOffsetPage(selected, 0, sort, dataFilter, size);
      } else {
        setHistory([null]);
        setHistoryIndex(0);
        void loadTable(selected, null, size);
      }
    },
    [selected, pagedMode, sort, dataFilter, loadOffsetPage, loadTable, guardPending],
  );

  const requestDesigner = useCallback(() => {
    if (guardPending()) return;
    setShowDesigner(true);
  }, [guardPending]);

  const refreshAfterSchemaChange = useCallback(
    async (table: string | null, message: string) => {
      try {
        const [nextTables, nextSchema, nextGraph] = await Promise.all([
          api.listTables(),
          api.schemaMap(),
          api.schemaGraph(),
        ]);
        setTables(nextTables);
        setSchema(nextSchema);
        setSchemaGraph(nextGraph);
        try {
          lastDataVersion.current = await api.dataVersion();
        } catch {
          // Reloads below still refresh the visible state.
        }
        externalChangePending.current = false;
        setNotice(message);

        if (!table) return;
        if (!nextTables.some((item) => item.name === table)) {
          setSelected(null);
          setPage(null);
          setColumns([]);
          setCount(null);
          setTableDetails(null);
          setTableStats(null);
          setView("data");
          return;
        }
        if (view === "structure") {
          await loadDetails(table);
        } else if (view === "data" && selected === table) {
          if (pagedMode) {
            await loadOffsetPage(table, pageIndex, sort, dataFilter);
          } else {
            await loadTable(table, history[historyIndex]);
          }
        }
      } catch (cause) {
        setError(String(cause));
      }
    },
    [
      view,
      selected,
      pagedMode,
      pageIndex,
      sort,
      dataFilter,
      history,
      historyIndex,
      loadTable,
      loadOffsetPage,
      loadDetails,
    ],
  );

  const requestAlterTable = useCallback(() => {
    if (!selected) return;
    if (guardPending()) return;
    setShowAlterTable(true);
  }, [selected, guardPending]);

  const handleAlterApplied = useCallback(
    async (change: SchemaChange) => {
      const backup = change.backup_path ? ` · backup ${change.backup_path}` : "";
      await refreshAfterSchemaChange(
        selected,
        `Applied schema change · migration ${change.migration_name}${backup}`,
      );
    },
    [selected, refreshAfterSchemaChange],
  );

  const handleCreateIndex = useCallback(
    async (name: string, columns: string[], unique: boolean) => {
      if (!selected) throw new Error("No table selected.");
      if (guardPending()) throw new Error("Save or discard pending changes first.");
      const change = await api.createIndex(selected, name, columns, unique);
      await refreshAfterSchemaChange(
        selected,
        `Created index ${name} · migration ${change.migration_name}`,
      );
    },
    [selected, guardPending, refreshAfterSchemaChange],
  );

  const handleDropIndex = useCallback(
    async (name: string) => {
      if (!selected) throw new Error("No table selected.");
      if (guardPending()) throw new Error("Save or discard pending changes first.");
      const confirmed = await confirm(
        `Drop index ${name}? A timestamped backup is kept before the change.`,
        { title: "Drop index?", kind: "warning" },
      );
      if (!confirmed) return;
      const change = await api.dropIndex(name);
      await refreshAfterSchemaChange(
        selected,
        `Dropped index ${name} · migration ${change.migration_name}`,
      );
    },
    [selected, guardPending, refreshAfterSchemaChange],
  );

  const openForeignKey = useCallback(
    async (foreignKey: ForeignKeyInfo, cell: CellValue) => {
      if (guardPending()) return;
      if (!foreignKey.referenced_column) return;
      if (cell.kind === "null" || cell.kind === "blob") return;

      const value = String(cell.value);
      const filter: FilterSpec = {
        column: foreignKey.referenced_column,
        op: "eq",
        value,
      };
      setView("data");
      setSort(null);
      setDataFilter(filter);
      setFilterDraft({ column: filter.column, op: filter.op, value });
      setPageIndex(0);
      setSelectedRow(null);
      setPage(null);
      setColumns([]);
      setForeignKeys([]);
      setTableStats(null);
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);
      await loadOffsetPage(foreignKey.referenced_table, 0, null, filter);
    },
    [guardPending, loadOffsetPage],
  );

  const openReferencedTable = useCallback(
    (table: string) => {
      if (guardPending()) return;
      setView("data");
      setSort(null);
      setDataFilter(null);
      setFilterDraft({ column: "", op: "contains", value: "" });
      setPageIndex(0);
      setSelectedRow(null);
      setPage(null);
      setColumns([]);
      setForeignKeys([]);
      setTableStats(null);
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);
      void loadTable(table, null);
    },
    [guardPending, loadTable],
  );

  const openMigrations = useCallback(() => {
    if (guardPending()) return;
    setShowMigrations(true);
    void loadMigrations();
  }, [guardPending, loadMigrations]);

  const handleApplyMigrations = useCallback(async () => {
    if (guardPending()) return;
    setMigrationsApplying(true);
    setMigrationsError(null);
    try {
      const result = await api.applyMigrations();
      await loadMigrations();
      if (result.applied.length === 0) {
        setNotice("No pending migrations.");
        return;
      }
      const names = result.applied.map((migration) => migration.name).join(", ");
      await refreshAfterSchemaChange(
        selected,
        `Applied ${result.applied.length} migration(s): ${names}`,
      );
    } catch (cause) {
      setMigrationsError(String(cause));
      try {
        setMigrations(await api.listMigrations());
      } catch {
        // Keep the original error visible.
      }
    } finally {
      setMigrationsApplying(false);
    }
  }, [guardPending, loadMigrations, refreshAfterSchemaChange, selected]);

  const openBackups = useCallback(() => {
    if (guardPending()) return;
    setShowBackups(true);
    void loadBackups();
  }, [guardPending, loadBackups]);

  const handleCreateBackup = useCallback(async () => {
    if (creatingBackup || restoringBackup !== null) return;
    setCreatingBackup(true);
    setBackupsError(null);
    try {
      const path = await api.createBackup();
      await loadBackups();
      setNotice(`Backup created at ${path}`);
    } catch (cause) {
      setBackupsError(String(cause));
    } finally {
      setCreatingBackup(false);
    }
  }, [creatingBackup, restoringBackup, loadBackups]);

  const requestImport = useCallback(() => {
    if (guardPending()) return;
    setShowImport(true);
  }, [guardPending]);

  const handleImported = useCallback(
    async (result: ImportResult) => {
      setShowImport(false);
      const ignored =
        result.ignored_columns.length > 0
          ? ` · ignored ${result.ignored_columns.join(", ")}`
          : "";
      const created = result.created_table ? " · created table" : "";
      await refreshAfterSchemaChange(
        result.table,
        `Imported ${result.rows.toLocaleString()} row(s) into ${result.table} · session ${result.session_id}${created}${ignored}`,
      );
      openReferencedTable(result.table);
    },
    [refreshAfterSchemaChange, openReferencedTable],
  );

  const requestD1Export = useCallback(() => {
    if (guardPending()) return;
    setShowD1Export(true);
  }, [guardPending]);

  const handleD1Exported = useCallback((result: D1ExportResult) => {
    setShowD1Export(false);
    const warnings =
      result.warnings.length > 0
        ? ` · ${result.warnings.length} warning${result.warnings.length === 1 ? "" : "s"}`
        : "";
    setNotice(
      `D1 export written to ${result.output_dir} · ${result.rows.toLocaleString()} row(s) · ${result.data_files.length} data file${result.data_files.length === 1 ? "" : "s"}${warnings}`,
    );
  }, []);

  const handleRestoreBackup = useCallback(
    async (backup: BackupInfo) => {
      if (guardPending()) return;
      const confirmed = await confirm(
        `Restore ${backup.name}? This replaces the current database and cannot be undone unless you take a backup first. Changes made after this snapshot will be lost.`,
        { title: "Restore backup?", kind: "warning" },
      );
      if (!confirmed) return;

      setRestoringBackup(backup.path);
      setBackupsError(null);
      try {
        const result = await api.restoreBackup(backup.path);
        await loadBackups();
        await refreshAfterSchemaChange(
          selected,
          `Restored ${backup.name} · ${result.tables} table(s)`,
        );
      } catch (cause) {
        setBackupsError(String(cause));
        try {
          setBackups(await api.listBackups());
        } catch {
          // Keep the original error visible.
        }
      } finally {
        setRestoringBackup(null);
      }
    },
    [guardPending, loadBackups, refreshAfterSchemaChange, selected],
  );

  const saveChanges = useCallback(async () => {
    if (!selected || !hasPending || saving) return;
    setSaving(true);
    setError(null);
    try {
      const updates: RowUpdate[] = pendingUpdates.map((update) => ({
        rowid: update.rowid,
        values: update.values,
      }));
      const inserts = pendingInserts.map((insert) => insert.values);
      const result = await api.saveChanges(selected, updates, inserts, pendingDeletes);

      setPendingInserts([]);
      setPendingUpdates([]);
      setPendingDeletes([]);
      setRowEditor(null);
      setSelectedRow(null);
      setSelectionKeys(new Set());
      setLastSelectedIndex(null);

      const parts = [
        `Saved ${result.affected} change${result.affected === 1 ? "" : "s"}`,
        `session ${result.session_id}`,
      ];
      if (result.backup_path) parts.push(`backup ${result.backup_path}`);
      setNotice(parts.join(" · "));

      // The save changed data_version; record it so the poll doesn't reload again.
      try {
        lastDataVersion.current = await api.dataVersion();
      } catch {
        // The explicit reload below still refreshes the view.
      }
      externalChangePending.current = false;

      if (pagedMode) {
        await loadOffsetPage(selected, pageIndex, sort, dataFilter);
      } else {
        await loadTable(selected, history[historyIndex]);
      }
      void api.countRows(selected).then(setCount).catch(() => {});
    } catch (cause) {
      setError(String(cause));
    } finally {
      setSaving(false);
    }
  }, [
    selected,
    hasPending,
    saving,
    pendingUpdates,
    pendingInserts,
    pendingDeletes,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    history,
    historyIndex,
    loadOffsetPage,
    loadTable,
  ]);

  const submitRowEditor = useCallback(
    (values: JsonObject) => {
      const editor = rowEditor;
      if (!editor) return;

      if (editor.mode === "insert") {
        const pendingKey = editor.row?.pendingKey;
        if (pendingKey) {
          setPendingInserts((current) =>
            current.map((insert) =>
              insert.key === pendingKey ? { ...insert, values } : insert,
            ),
          );
        } else {
          setPendingInserts((current) => [
            ...current,
            { key: makePendingKey(), values },
          ]);
        }
        setRowEditor(null);
        return;
      }

      const row = editor.row;
      if (row.pendingKey) {
        setPendingInserts((current) =>
          current.map((insert) =>
            insert.key === row.pendingKey ? { ...insert, values } : insert,
          ),
        );
      } else if (row.rowid !== null) {
        const rowid = row.rowid;
        setPendingUpdates((current) => {
          const existing = current.find((update) => update.rowid === rowid);
          const before = existing?.before ?? rowToValues(columns, row);
          // Keep only columns the user actually changed, so a later save
          // cannot overwrite unrelated edits made by another process.
          const changed: JsonObject = {};
          for (const [name, value] of Object.entries(values)) {
            if (!valuesEqual(value, before[name])) changed[name] = value;
          }
          if (Object.keys(changed).length === 0) {
            return current.filter((update) => update.rowid !== rowid);
          }
          if (existing) {
            return current.map((update) =>
              update.rowid === rowid ? { ...update, values: changed } : update,
            );
          }
          return [...current, { rowid, values: changed, before }];
        });
      }
      setRowEditor(null);
    },
    [rowEditor, columns],
  );

  const editSelectedRow = useCallback(() => {
    const row = selectedRow?.row;
    if (!row) return;
    setRowEditor({ mode: row.pendingKey ? "insert" : "update", row });
  }, [selectedRow]);

  const deleteSelectedRow = useCallback(() => {
    const row = selectedRow?.row;
    if (!row) return;
    const selectionKey = selectedRow
      ? rowSelectionKey(selectedRow.row, selectedRow.index)
      : null;
    if (row.pendingKey) {
      setPendingInserts((current) =>
        current.filter((insert) => insert.key !== row.pendingKey),
      );
    } else if (row.rowid !== null) {
      const rowid = row.rowid;
      setPendingUpdates((current) =>
        current.filter((update) => update.rowid !== rowid),
      );
      setPendingDeletes((current) =>
        current.includes(rowid) ? current : [...current, rowid],
      );
    }
    if (selectionKey) {
      setSelectionKeys((current) => {
        const next = new Set(current);
        next.delete(selectionKey);
        return next;
      });
    }
    setSelectedRow(null);
  }, [selectedRow]);

  useEffect(() => {
    setSelectedRow((current) => {
      if (!current || view !== "data") return current;
      const next = displayRows[current.index];
      if (!next) return null;
      return next === current.row ? current : { row: next, index: current.index };
    });
  }, [displayRows, view]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void openDb();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [openDb]);

  useEffect(() => {
    if (bootstrapped.current) return;
    bootstrapped.current = true;
    void api
      .initialDatabase()
      .then((path) => {
        if (path) void openPath(path);
      })
      .catch(() => {});
  }, [openPath]);

  useEffect(() => {
    lastDataVersion.current = null;
  }, [db?.path]);

  useEffect(() => {
    if (!db) return;
    let cancelled = false;
    const timer = window.setInterval(() => {
      void (async () => {
        if (cancelled || loading || queryRunning || saving || restoringBackup !== null) return;
        try {
          const version = await api.dataVersion();
          if (cancelled) return;
          if (lastDataVersion.current === null) {
            lastDataVersion.current = version;
            return;
          }
          if (version === lastDataVersion.current) return;

          if (hasPending) {
            // Keep staged edits and let the user decide; the next poll after
            // saving or discarding will pick up the external change.
            if (!externalChangePending.current) {
              externalChangePending.current = true;
              setNotice(
                "Another process changed the database. Your unsaved changes are kept; save or discard them to reload.",
              );
            }
            return;
          }

          externalChangePending.current = false;
          lastDataVersion.current = version;

          const [nextTables, nextSchema, nextGraph] = await Promise.all([
            api.listTables(),
            api.schemaMap(),
            api.schemaGraph(),
          ]);
          if (cancelled) return;
          setTables(nextTables);
          setSchema(nextSchema);
          setSchemaGraph(nextGraph);

          if (view === "diagram") return;
          if (view === "structure") {
            if (!selected) return;
            if (!nextTables.some((table) => table.name === selected)) {
              setSelected(null);
              setPage(null);
              setColumns([]);
              setCount(null);
              setTableDetails(null);
              setTableStats(null);
              return;
            }
            await loadDetails(selected);
            return;
          }
          if (view === "sql") {
            setNotice(
              "Database changed by another process. Rerun the query to see fresh data.",
            );
            return;
          }
          if (!selected) return;
          if (!nextTables.some((table) => table.name === selected)) {
            setSelected(null);
            setPage(null);
            setColumns([]);
            setCount(null);
            return;
          }

          if (pagedMode) {
            await loadOffsetPage(selected, pageIndex, sort, dataFilter);
          } else {
            await loadTable(selected, history[historyIndex]);
          }
        } catch {
          // Ignore transient polling errors.
        }
      })();
    }, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [
    db,
    view,
    selected,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    history,
    historyIndex,
    loading,
    queryRunning,
    saving,
    restoringBackup,
    hasPending,
    loadTable,
    loadOffsetPage,
    loadDetails,
  ]);

  const visibleTables = tables.filter((table) =>
    table.name.toLowerCase().includes(filterText.trim().toLowerCase()),
  );
  const currentPage = pagedMode ? pageIndex : historyIndex;
  const rowStart = page && page.rows.length > 0 ? currentPage * pageSize + 1 : 0;
  const rowEnd = page ? currentPage * pageSize + page.rows.length : 0;
  const detailColumns = view === "data" ? columns : (queryResult?.columns ?? []);
  const nextDisabled = loading || !page?.has_more;
  const prevDisabled = loading || (pagedMode ? pageIndex <= 0 : historyIndex <= 0);

  return (
    <div className="flex h-screen flex-col">
      <header className="flex h-[52px] flex-none items-center justify-between border-b border-line bg-ink/80 px-4 backdrop-blur-xl">
        <div className="flex items-center gap-2.5">
          <div className="grid h-7 w-7 place-items-center rounded-lg bg-linear-to-br from-accent to-accent-2 text-[11px] font-extrabold tracking-tight text-[#04110f] shadow-[inset_0_0_0_1px_rgba(255,255,255,0.12),0_6px_18px_rgba(45,212,191,0.25)]">
            DB
          </div>
          <div className="font-bold tracking-tight">DBGo</div>
          <span className="ml-0.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-2">
            local SQL workbench
          </span>
        </div>
        <div className="flex items-center gap-2">
          {db && (
            <button
              className={BTN_GHOST}
              onClick={refresh}
              disabled={!selected || loading || queryRunning}
              title={
                view === "sql"
                  ? "Run the current query"
                  : view === "diagram"
                    ? "Refresh the schema graph"
                    : view === "structure"
                      ? "Refresh the table structure"
                      : "Refresh the current page"
              }
            >
              {view === "sql" ? "Run query" : "Refresh"}
            </button>
          )}
          {db && (
            <button
              className={BTN_GHOST}
              onClick={openMigrations}
              disabled={loading || queryRunning || saving}
              title="List and apply migrations"
            >
              Migrations
            </button>
          )}
          {db && (
            <button
              className={BTN_GHOST}
              onClick={openBackups}
              disabled={loading || queryRunning || saving || restoringBackup !== null}
              title="Browse and restore backups"
            >
              Backups
            </button>
          )}
          {db && (
            <button
              className={BTN_GHOST}
              onClick={requestImport}
              disabled={loading || restoringBackup !== null || creatingBackup}
              title="Import CSV, JSON, or JSONL"
            >
              Import
            </button>
          )}
          {db && (
            <button
              className={BTN_GHOST}
              onClick={requestD1Export}
              disabled={loading || restoringBackup !== null || creatingBackup}
              title="Export a Cloudflare D1 bundle"
            >
              D1 Export
            </button>
          )}
          <button className={BTN_GHOST} onClick={() => void createNewDatabase()}>
            New database
          </button>
          <button className={BTN_PRIMARY} onClick={() => void openDb()}>
            Open database
          </button>
        </div>
      </header>

      <div className="flex min-h-0 flex-1">
        <aside className="flex w-[268px] min-w-[220px] max-w-[360px] flex-col border-r border-line bg-panel/60">
          <div className="flex h-11 flex-none items-center justify-between px-3.5">
            <span className="text-[10.5px] font-bold uppercase tracking-[0.14em] text-muted">
              Tables
            </span>
            <div className="flex items-center gap-2">
              <span className={BADGE}>{tables.length}</span>
              <button
                type="button"
                className="inline-flex h-6 w-6 items-center justify-center rounded-md border border-transparent text-base leading-none text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg disabled:cursor-default disabled:opacity-40"
                onClick={requestDesigner}
                disabled={!db || hasPending}
                title={hasPending ? "Save or discard pending changes first" : "New table"}
                aria-label="New table"
              >
                +
              </button>
            </div>
          </div>
          <input
            className="mx-3 mb-2.5 h-[30px] flex-none rounded-[7px] border border-line bg-ink px-2.5 text-[12.5px] text-fg outline-none placeholder:text-muted-2 focus:border-accent focus:ring-2 focus:ring-accent/20 disabled:opacity-45"
            placeholder="Filter tables..."
            value={filterText}
            onChange={(event) => setFilterText(event.target.value)}
            disabled={!db}
            aria-label="Filter tables"
          />
          <div className="min-h-0 flex-1 overflow-auto px-2 pb-2">
            {!db && <div className="px-2.5 py-4 text-xs text-muted-2">No database open</div>}
            {db && visibleTables.length === 0 && (
              <div className="px-2.5 py-4 text-xs text-muted-2">No tables</div>
            )}
            {visibleTables.map((table) => (
              <button
                key={table.name}
                className={cn(
                  "group flex w-full items-center gap-2 rounded-md border border-transparent px-2 py-1.5 text-left text-[12.5px] text-fg hover:bg-panel-2",
                  selected === table.name && "border-accent/25 bg-accent/10 text-[#b8fff4]",
                )}
                onClick={() => selectTable(table.name)}
                title={table.name}
              >
                <span
                  className={cn(
                    "text-[13px] leading-none",
                    selected === table.name ? "text-accent" : "text-muted-2 group-hover:text-muted",
                  )}
                >
                  ▦
                </span>
                <span className="truncate font-mono text-xs">{table.name}</span>
              </button>
            ))}
          </div>
          <div className="flex-none border-t border-line px-3 py-2 text-[10.5px] text-muted-2">
            {db ? (
              <span className="flex items-center gap-1.5 truncate font-mono" title={db.path}>
                <span className="h-1.5 w-1.5 flex-none rounded-full bg-accent shadow-[0_0_8px_rgba(45,212,191,0.6)]" />
                {db.path.split("/").pop()}
              </span>
            ) : (
              <span>Open a SQLite file to begin</span>
            )}
          </div>
        </aside>

        <main className="flex min-h-0 min-w-0 flex-1 flex-col">
          {error && (
            <div className="flex flex-none items-center justify-between gap-3 border-b border-danger/25 bg-danger/10 px-3.5 py-2 font-mono text-[11.5px] text-[#fecaca]">
              <span>{error}</span>
              <button
                className="text-lg leading-none"
                onClick={() => setError(null)}
                aria-label="Dismiss"
              >
                ×
              </button>
            </div>
          )}

          {notice && (
            <div className="flex flex-none items-center justify-between gap-3 border-b border-accent/25 bg-accent/10 px-3.5 py-2 font-mono text-[11.5px] text-[#b8fff4]">
              <span>{notice}</span>
              <button
                className="text-lg leading-none"
                onClick={() => setNotice(null)}
                aria-label="Dismiss notice"
              >
                ×
              </button>
            </div>
          )}

          {db && hasPending && (
            <div
              data-testid="pending-bar"
              className="flex flex-none items-center justify-between gap-3 border-b border-accent/30 bg-accent/[0.08] px-3.5 py-2"
            >
              <div className="flex min-w-0 items-center gap-2.5">
                <span className="h-1.5 w-1.5 flex-none rounded-full bg-accent shadow-[0_0_8px_rgba(45,212,191,0.7)]" />
                <span className="whitespace-nowrap font-mono text-[11.5px] font-semibold text-[#b8fff4]">
                  {pendingInserts.length + pendingUpdates.length + pendingDeletes.length} unsaved
                  change
                  {pendingInserts.length + pendingUpdates.length + pendingDeletes.length === 1
                    ? ""
                    : "s"}
                  {selected ? ` on ${selected}` : ""}
                </span>
                <span className="hidden whitespace-nowrap text-[10.5px] text-muted sm:inline">
                  {pendingInserts.length} new · {pendingUpdates.length} edited ·{" "}
                  {pendingDeletes.length} deleted
                </span>
              </div>
              <div className="flex flex-none items-center gap-2">
                <button
                  className={BTN_SM_GHOST}
                  onClick={discardChanges}
                  disabled={saving}
                  data-testid="discard-changes"
                >
                  Discard
                </button>
                <button
                  className={BTN_SM_PRIMARY}
                  onClick={() => void saveChanges()}
                  disabled={saving}
                  data-testid="save-changes"
                >
                  {saving ? "Saving..." : "Save changes"}
                </button>
              </div>
            </div>
          )}

          {!db ? (
            <div className="flex flex-1 flex-col items-center overflow-auto p-10 text-center">
              <div className="my-auto flex w-full max-w-[560px] flex-col items-center gap-2.5">
                <div className="grid h-[54px] w-[54px] place-items-center rounded-[14px] bg-linear-to-br from-accent to-accent-2 text-base font-extrabold tracking-tight text-[#04110f] shadow-[0_14px_40px_rgba(45,212,191,0.25)]">
                  DB
                </div>
                <h1 className="mt-1 text-[22px] tracking-tight">Create or open a database</h1>
                <p className="max-w-[420px] text-[13px] leading-relaxed text-muted">
                  DBGo is a local-first SQL workbench. Your data never leaves this machine.
                </p>
                <div className="flex items-center gap-2">
                  <button
                    className={BTN_LARGE_PRIMARY}
                    onClick={() => void createNewDatabase()}
                  >
                    Create database
                  </button>
                  <button className={BTN_LARGE_NEUTRAL} onClick={() => void openDb()}>
                    Open database
                  </button>
                </div>
                <div className="mt-1.5 font-mono text-[11px] text-muted-2">Ctrl / Cmd + O</div>

                {recentDatabases.length > 0 && (
                  <div className="mt-6 w-full text-left">
                    <div className="mb-2 flex items-center justify-between">
                      <span className="text-[10.5px] font-bold uppercase tracking-[0.14em] text-muted">
                        Recent databases
                      </span>
                      <button
                        type="button"
                        className="text-[10.5px] text-muted-2 hover:text-fg"
                        onClick={clearRecentDatabases}
                      >
                        Clear
                      </button>
                    </div>
                    <div className="overflow-hidden rounded-xl border border-line bg-panel/60">
                      {recentDatabases.map((entry) => {
                        const missing = missingRecents.has(entry.path);
                        return (
                          <div
                            key={entry.path}
                            className="flex items-center gap-2 border-b border-line/60 px-3 py-2 last:border-b-0"
                          >
                            <button
                              type="button"
                              data-recent-path={entry.path}
                              onClick={() => void openRecentDatabase(entry)}
                              disabled={missing}
                              className="flex min-w-0 flex-1 flex-col items-start text-left disabled:cursor-default disabled:opacity-45"
                              title={entry.path}
                            >
                              <span className="flex min-w-0 items-center gap-2">
                                <span className="truncate font-mono text-[12px] text-fg">
                                  {entry.name}
                                </span>
                                {missing && (
                                  <span className="rounded-full border border-danger/30 bg-danger/10 px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-[0.08em] text-[#fca5a5]">
                                    missing
                                  </span>
                                )}
                              </span>
                              <span className="mt-0.5 w-full truncate font-mono text-[10px] text-muted-2">
                                {entry.path}
                              </span>
                            </button>
                            <span className="flex-none text-[10px] text-muted-2">
                              {formatRecentOpened(entry.openedAt)}
                            </span>
                            <button
                              type="button"
                              aria-label={`Remove ${entry.name} from recent databases`}
                              onClick={() => removeRecentDatabase(entry.path)}
                              className="inline-flex h-6 w-6 flex-none items-center justify-center rounded-md border border-transparent text-base leading-none text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg"
                              title="Remove from recent databases"
                            >
                              ×
                            </button>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            </div>
          ) : !selected ? (
            tables.length === 0 ? (
              <div className="flex flex-1 flex-col items-center justify-center gap-2.5 p-10 text-center">
                <div className="grid h-[42px] w-[42px] place-items-center rounded-[11px] border border-line bg-panel-3 text-lg text-muted">
                  ▦
                </div>
                <h2 className="mt-1 text-base tracking-tight">No tables yet</h2>
                <p className="max-w-[420px] text-[13px] leading-relaxed text-muted">
                  Create your first table to start shaping the database.
                </p>
                <button
                  className={BTN_LARGE_PRIMARY}
                  onClick={requestDesigner}
                >
                  Create your first table
                </button>
              </div>
            ) : (
              <div className="flex flex-1 flex-col items-center justify-center gap-2.5 p-10 text-center">
                <div className="grid h-[42px] w-[42px] place-items-center rounded-[11px] border border-line bg-panel-3 text-lg text-muted">
                  ▦
                </div>
                <h2 className="mt-1 text-base tracking-tight">Select a table</h2>
                <p className="max-w-[420px] text-[13px] leading-relaxed text-muted">
                  Pick a table from the sidebar to browse its rows.
                </p>
              </div>
            )
          ) : (
            <>
              <div className="flex h-12 flex-none items-center justify-between gap-3 border-b border-line px-3.5">
                <div className="flex min-w-0 items-center gap-2">
                  <div
                    className="flex gap-0.5 rounded-[7px] border border-line bg-ink p-0.5"
                    role="tablist"
                    aria-label="View"
                  >
                    <button
                      role="tab"
                      aria-selected={view === "data"}
                      className={cn(
                        "h-6 rounded-[5px] px-2.5 text-[11.5px] font-semibold",
                        view === "data" ? "bg-panel-3 text-fg" : "text-muted hover:text-fg",
                      )}
                      onClick={() => switchView("data")}
                    >
                      Data
                    </button>
                    <button
                      role="tab"
                      aria-selected={view === "structure"}
                      className={cn(
                        "h-6 rounded-[5px] px-2.5 text-[11.5px] font-semibold",
                        view === "structure" ? "bg-panel-3 text-fg" : "text-muted hover:text-fg",
                      )}
                      onClick={() => switchView("structure")}
                    >
                      Structure
                    </button>
                    <button
                      role="tab"
                      aria-selected={view === "sql"}
                      className={cn(
                        "h-6 rounded-[5px] px-2.5 text-[11.5px] font-semibold",
                        view === "sql" ? "bg-panel-3 text-fg" : "text-muted hover:text-fg",
                      )}
                      onClick={() => switchView("sql")}
                    >
                      SQL
                    </button>
                    <button
                      role="tab"
                      aria-selected={view === "diagram"}
                      className={cn(
                        "h-6 rounded-[5px] px-2.5 text-[11.5px] font-semibold",
                        view === "diagram" ? "bg-panel-3 text-fg" : "text-muted hover:text-fg",
                      )}
                      onClick={() => switchView("diagram")}
                    >
                      Diagram
                    </button>
                  </div>
                  {view === "diagram" ? (
                    <>
                      <span className="truncate font-mono text-[13px] font-semibold">Schema</span>
                      <span className={BADGE_SUBTLE}>
                        {schemaGraph?.tables.length ?? 0} tables
                      </span>
                      <span className={BADGE_SUBTLE}>
                        {schemaGraph?.relationships.length ?? 0} relations
                      </span>
                    </>
                  ) : (
                    <>
                      <span
                        data-testid="table-title"
                        className="truncate font-mono text-[13px] font-semibold"
                      >
                        {selected}
                      </span>
                      <span className={BADGE_SUBTLE}>
                        {view === "structure" && tableDetails?.name === selected
                          ? tableDetails.columns.length
                          : columns.length}{" "}
                        columns
                      </span>
                      {view === "structure" && tableDetails?.name === selected && (
                        <span className={BADGE_SUBTLE}>
                          {tableDetails.indexes.length} indexes
                        </span>
                      )}
                      {count && !dataFilter && view === "data" && (
                        <span className={BADGE_SUBTLE}>
                          {count.capped
                            ? `${count.count.toLocaleString()}+`
                            : count.count.toLocaleString()}{" "}
                          rows
                        </span>
                      )}
                    </>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  {view === "data" ? (
                    <>
                      <button
                        className={BTN_SM_PRIMARY}
                        onClick={() => setRowEditor({ mode: "insert", row: null })}
                        disabled={columns.length === 0}
                        data-testid="add-row"
                        title="Add a row as a pending change"
                      >
                        + Row
                      </button>
                      <label className="flex items-center gap-1.5 text-[10.5px] font-semibold uppercase tracking-[0.08em] text-muted">
                        Rows
                        <select
                          className={SELECT}
                          value={pageSize}
                          onChange={(event) => changePageSize(Number(event.target.value))}
                          aria-label="Page size"
                        >
                          {PAGE_SIZES.map((size) => (
                            <option key={size} value={size}>
                              {size}
                            </option>
                          ))}
                        </select>
                      </label>
                      <div className="flex items-center gap-1.5">
                        <button
                          className={BTN_SM_GHOST}
                          onClick={() => void prevPage()}
                          disabled={prevDisabled}
                        >
                          Previous
                        </button>
                        <span className="min-w-[58px] text-center text-[11px] tabular-nums text-muted">
                          Page {currentPage + 1}
                        </span>
                        <button
                          className={BTN_SM_GHOST}
                          onClick={() => void nextPage()}
                          disabled={nextDisabled}
                        >
                          Next
                        </button>
                      </div>
                      <span className="mx-1 h-4 w-px bg-line" />
                      <select
                        className={SELECT}
                        value={exportFormat}
                        onChange={(event) =>
                          setExportFormat(event.target.value as ExportFormat)
                        }
                        aria-label="Export format"
                      >
                        <option value="csv">CSV</option>
                        <option value="json">JSON</option>
                      </select>
                      <button
                        className={BTN_SM_GHOST}
                        onClick={() => void exportCurrent()}
                        disabled={exporting || !selected}
                        data-testid="export-data"
                        title="Export the current table view"
                      >
                        {exporting ? "Exporting..." : "Export"}
                      </button>
                    </>
                  ) : view === "structure" ? (
                    <span className={BADGE_SUBTLE}>structure</span>
                  ) : view === "sql" ? (
                    <span className={BADGE_SUBTLE}>read-only</span>
                  ) : (
                    <span className={BADGE_SUBTLE}>schema</span>
                  )}
                </div>
              </div>

              {view === "data" && (
                <div className="flex h-10 flex-none items-center gap-2 border-b border-line bg-panel/40 px-3">
                  <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted">
                    Filter
                  </span>
                  <select
                    className={SELECT}
                    value={filterDraft.column}
                    onChange={(event) =>
                      setFilterDraft((current) => ({ ...current, column: event.target.value }))
                    }
                    disabled={columns.length === 0}
                    aria-label="Filter column"
                  >
                    {columns.map((column) => (
                      <option key={column.name} value={column.name}>
                        {column.name}
                      </option>
                    ))}
                  </select>
                  <select
                    className={SELECT}
                    value={filterDraft.op}
                    onChange={(event) =>
                      setFilterDraft((current) => ({
                        ...current,
                        op: event.target.value as FilterOp,
                      }))
                    }
                    aria-label="Filter operator"
                  >
                    {FILTER_OPS.map((op) => (
                      <option key={op.value} value={op.value}>
                        {op.label}
                      </option>
                    ))}
                  </select>
                  <input
                    className="h-7 w-[200px] rounded-md border border-line bg-ink px-2 text-xs text-fg outline-none placeholder:text-muted-2 focus:border-accent disabled:opacity-40"
                    placeholder="value"
                    value={filterDraft.value}
                    onChange={(event) =>
                      setFilterDraft((current) => ({ ...current, value: event.target.value }))
                    }
                    onKeyDown={(event) => {
                      if (event.key === "Enter") applyFilter();
                    }}
                    disabled={isNullOp(filterDraft.op)}
                    aria-label="Filter value"
                  />
                  <button
                    className={BTN_PRIMARY}
                    onClick={applyFilter}
                    disabled={!filterDraft.column}
                  >
                    Apply
                  </button>
                  {dataFilter && (
                    <button className={BTN_GHOST} onClick={clearFilter}>
                      Clear
                    </button>
                  )}
                  {dataFilter && <span className={BADGE_SUBTLE}>filtered</span>}
                  {sort && (
                    <span className={BADGE_SUBTLE}>
                      sorted by {sort.column} {sort.direction}
                    </span>
                  )}
                </div>
              )}

              {view === "sql" && (
                <div className="flex-none border-b border-line bg-panel">
                  <div className="flex h-[42px] items-center gap-2 px-3">
                    <button
                      className={BTN_PRIMARY}
                      onClick={() => void runQuery(sqlText)}
                      disabled={queryRunning || explainRunning || !sqlText.trim()}
                    >
                      {queryRunning ? "Running..." : "Run"}
                    </button>
                    <button
                      className={BTN_NEUTRAL}
                      onClick={() => void explainQuery(sqlText)}
                      disabled={queryRunning || explainRunning || !sqlText.trim()}
                      title="Show SQLite's query plan without running the statement"
                    >
                      {explainRunning ? "Explaining..." : "Explain"}
                    </button>
                    <span className="whitespace-nowrap font-mono text-[10px] text-muted-2">
                      Ctrl / Cmd + Enter
                    </span>
                    <label className="flex items-center gap-1.5 text-[10.5px] font-semibold uppercase tracking-[0.08em] text-muted">
                      Max rows
                      <select
                        className={SELECT}
                        value={queryLimit}
                        onChange={(event) => setQueryLimit(Number(event.target.value))}
                        aria-label="Max rows"
                      >
                        {QUERY_LIMITS.map((limit) => (
                          <option key={limit} value={limit}>
                            {limit}
                          </option>
                        ))}
                      </select>
                    </label>
                    <select
                      className="h-[26px] max-w-[260px] rounded-md border border-line bg-panel-2 px-1.5 text-[11px] text-fg outline-none disabled:opacity-45"
                      value=""
                      onChange={(event) => {
                        const value = event.target.value;
                        if (value) changeSqlText(value);
                      }}
                      disabled={queryHistory.length === 0}
                      title="Query history"
                    >
                      <option value="">History...</option>
                      {queryHistory.map((item, index) => (
                        <option key={`${index}-${item.slice(0, 24)}`} value={item}>
                          {item.replace(/\s+/g, " ").slice(0, 90)}
                        </option>
                      ))}
                    </select>
                    <span className="mx-1 h-4 w-px bg-line" />
                    <select
                      className={SELECT}
                      value={exportFormat}
                      onChange={(event) =>
                        setExportFormat(event.target.value as ExportFormat)
                      }
                      aria-label="Export format"
                    >
                      <option value="csv">CSV</option>
                      <option value="json">JSON</option>
                    </select>
                    <button
                      className={BTN_SM_GHOST}
                      onClick={() => void exportCurrent()}
                      disabled={exporting || !sqlText.trim()}
                      data-testid="export-query"
                      title="Export this query result"
                    >
                      {exporting ? "Exporting..." : "Export"}
                    </button>
                    {queryResult && (
                      <span className="ml-auto whitespace-nowrap font-mono text-[11px] text-muted">
                        {queryResult.rows.length.toLocaleString()} rows
                        {queryResult.truncated ? " · truncated" : ""}
                      </span>
                    )}
                  </div>
                  <div className="h-[170px] overflow-hidden border-t border-line">
                    <Suspense
                      fallback={
                        <div className="grid h-full place-items-center text-xs text-muted">
                          Loading editor...
                        </div>
                      }
                    >
                      <SqlEditor
                        value={sqlText}
                        schema={schema}
                        onChange={changeSqlText}
                        onRun={(value) => void runQuery(value)}
                      />
                    </Suspense>
                  </div>
                </div>
              )}

              {view === "sql" && queryPlan && (
                <QueryPlanPanel
                  plan={queryPlan}
                  onClose={() => setQueryPlan(null)}
                />
              )}

              {view === "diagram" ? (
                <div className="flex min-h-0 flex-1">
                  {!schemaGraph ? (
                    <div className="grid flex-1 place-items-center text-xs text-muted">
                      Loading diagram...
                    </div>
                  ) : (
                    <Suspense
                      fallback={
                        <div className="grid flex-1 place-items-center text-xs text-muted">
                          Loading diagram...
                        </div>
                      }
                    >
                      <DiagramView
                        graph={schemaGraph}
                        dbPath={db?.path ?? ""}
                        onOpenTable={(table) => {
                          if (guardPending()) return;
                          setView("data");
                          void loadTable(table, null);
                        }}
                        onCreateTable={requestDesigner}
                      />
                    </Suspense>
                  )}
                </div>
              ) : view === "structure" ? (
                selected ? (
                  <StructureView
                    table={selected}
                    details={tableDetails?.name === selected ? tableDetails : null}
                    stats={tableStats?.name === selected ? tableStats : null}
                    loading={detailsLoading}
                    onRefresh={() => void loadDetails(selected)}
                    onAlter={requestAlterTable}
                    onCreateIndex={handleCreateIndex}
                    onDropIndex={handleDropIndex}
                    onOpenReferencedTable={openReferencedTable}
                  />
                ) : (
                  <div className="grid flex-1 place-items-center text-xs text-muted">
                    Select a table to inspect its structure.
                  </div>
                )
              ) : (
                <div className="flex min-h-0 flex-1">
                  <div className="flex min-w-0 flex-1 flex-col">
                    {view === "data" ? (
                      loading && !page ? (
                        <div className="grid flex-1 place-items-center text-xs text-muted">
                          Loading rows...
                        </div>
                      ) : (
                        <>
                          {selectionKeys.size > 0 && (
                            <div
                              data-testid="selection-bar"
                              className="flex h-9 flex-none items-center justify-between border-b border-line bg-accent/[0.04] px-3"
                            >
                              <span className="text-[11px] font-semibold text-[#b8fff4]">
                                {selectionKeys.size} selected
                              </span>
                              <div className="flex items-center gap-1.5">
                                <button
                                  className={BTN_SM_GHOST}
                                  onClick={() => void copySelectedRows()}
                                  data-testid="copy-selection"
                                >
                                  Copy TSV
                                </button>
                                <button
                                  className="inline-flex h-7 items-center rounded-md border border-danger/35 bg-danger/10 px-2.5 text-[11.5px] font-medium text-[#fca5a5] hover:border-danger/60"
                                  onClick={deleteSelectedRows}
                                  data-testid="delete-selection"
                                >
                                  Delete
                                </button>
                                <button
                                  className={BTN_SM_GHOST}
                                  onClick={clearSelection}
                                  data-testid="clear-selection"
                                >
                                  Clear
                                </button>
                              </div>
                            </div>
                          )}
                          <div className="flex min-h-0 flex-1 flex-col">
                            <DataGrid
                              columns={columns}
                              rows={displayRows}
                              sort={sort}
                              onSort={toggleSort}
                              onRowClick={(_, index) =>
                                setSelectedRow({ row: displayRows[index], index })
                              }
                              selectedIndex={selectedRow?.index ?? null}
                              dirtyRowIds={pendingRowIds}
                              selectionKeys={selectionKeys}
                              allSelected={allSelected}
                              someSelected={someSelected}
                              onToggleRow={toggleRowSelection}
                              onToggleAll={toggleAllSelection}
                              onCopy={() => void copySelectedRows()}
                              onPaste={pasteIntoCell}
                              onClipboardError={(message) => setError(message)}
                              onEditError={(message) => setError(message)}
                              canEditCell={canEditCell}
                              onCommitCell={handleCellCommit}
                            />
                          </div>
                        </>
                      )
                    ) : queryRunning ? (
                      <div className="grid flex-1 place-items-center text-xs text-muted">
                        Running query...
                      </div>
                    ) : queryResult ? (
                      <DataGrid
                        columns={queryResult.columns}
                        rows={queryResult.rows}
                        onRowClick={(row, index) => setSelectedRow({ row, index })}
                        selectedIndex={selectedRow?.index ?? null}
                      />
                    ) : (
                      <div className="grid flex-1 place-items-center text-xs text-muted">
                        Run a query to see results
                      </div>
                    )}
                  </div>
                  {selectedRow && detailColumns.length > 0 && (
                    <RowDetail
                      columns={detailColumns}
                      row={selectedRow.row}
                      index={selectedRow.index}
                      onClose={() => setSelectedRow(null)}
                      pending={
                        view === "data" &&
                        (selectedRow.row.pendingKey !== undefined ||
                          (selectedRow.row.rowid !== null &&
                            pendingRowIds.has(selectedRow.row.rowid)))
                      }
                      onEdit={
                        view === "data" &&
                        (selectedRow.row.pendingKey !== undefined ||
                          selectedRow.row.rowid !== null)
                          ? editSelectedRow
                          : undefined
                      }
                      onDelete={
                        view === "data" &&
                        (selectedRow.row.pendingKey !== undefined ||
                          selectedRow.row.rowid !== null)
                          ? deleteSelectedRow
                          : undefined
                      }
                      foreignKeys={view === "data" ? foreignKeys : undefined}
                      onNavigateForeignKey={
                        view === "data"
                          ? (foreignKey, cell) => void openForeignKey(foreignKey, cell)
                          : undefined
                      }
                    />
                  )}
                </div>
              )}
            </>
          )}

          <footer className="flex h-[26px] flex-none items-center justify-between gap-4 border-t border-line bg-panel px-3 font-mono text-[10.5px] text-muted-2">
            <span className="max-w-[60%] truncate" title={db?.path ?? undefined}>
              {db ? db.path : "No database open"}
            </span>
            <span className="whitespace-nowrap">
              {view === "diagram"
                ? `${schemaGraph?.tables.length ?? 0} tables · ${
                    schemaGraph?.relationships.length ?? 0
                  } relationships`
                : view === "structure"
                  ? tableDetails?.name === selected
                    ? `${tableDetails.columns.length} columns · ${tableDetails.indexes.length} indexes · ${tableDetails.foreign_keys.length} foreign keys`
                    : "structure"
                  : view === "sql"
                    ? queryResult
                      ? `query · ${queryResult.rows.length.toLocaleString()} rows${
                          queryResult.truncated ? " · truncated" : ""
                        }`
                      : "read-only SQL"
                    : selected && page
                      ? page.rows.length === 0
                        ? "0 rows"
                        : `rows ${rowStart.toLocaleString()}–${rowEnd.toLocaleString()} · ${
                            pagedMode ? "filtered/sorted" : "keyset pagination"
                          }`
                      : selected
                        ? pagedMode
                          ? "filtered/sorted"
                          : "keyset pagination"
                        : "SQLite"}
            </span>
          </footer>
        </main>
      </div>

      <RowEditor
        open={rowEditor !== null}
        mode={rowEditor?.mode ?? "insert"}
        columns={columns}
        initial={rowEditor?.row ?? null}
        onClose={() => setRowEditor(null)}
        onSubmit={submitRowEditor}
      />

      <AlterTableDialog
        open={showAlterTable}
        table={selected ?? ""}
        columns={tableDetails?.name === selected ? tableDetails.columns : columns}
        schema={schema}
        onClose={() => setShowAlterTable(false)}
        onApplied={handleAlterApplied}
      />

      <MigrationsDialog
        open={showMigrations}
        migrations={migrations}
        loading={migrationsLoading}
        applying={migrationsApplying}
        error={migrationsError}
        onClose={() => setShowMigrations(false)}
        onRefresh={() => void loadMigrations()}
        onApply={() => void handleApplyMigrations()}
      />

      <BackupsDialog
        open={showBackups}
        backups={backups}
        loading={backupsLoading}
        error={backupsError}
        restoring={restoringBackup}
        creating={creatingBackup}
        onClose={() => setShowBackups(false)}
        onRefresh={() => void loadBackups()}
        onCreateBackup={() => void handleCreateBackup()}
        onRestore={(backup) => void handleRestoreBackup(backup)}
      />

      <ImportDialog
        open={showImport}
        tables={tables}
        schema={schema}
        selectedTable={selected}
        onClose={() => setShowImport(false)}
        onImported={handleImported}
      />

      <D1ExportDialog
        open={showD1Export}
        defaultName={
          db
            ? db.project_path
              ? basename(db.project_path)
              : basename(db.path).replace(/\.[^.]+$/, "")
            : "my-db"
        }
        onClose={() => setShowD1Export(false)}
        onExported={handleD1Exported}
      />

      <TableDesigner
        open={showDesigner}
        tables={tables}
        schema={schema}
        onClose={() => setShowDesigner(false)}
        onCreated={(table) => void handleTableCreated(table)}
      />
    </div>
  );
}

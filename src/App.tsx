import { lazy, Suspense, useCallback, useEffect, useRef, useState } from "react";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { api } from "./lib/api";
import type {
  ColumnInfo,
  CountResult,
  DbInfo,
  FilterOp,
  FilterSpec,
  Page,
  QueryResult,
  Row,
  SchemaGraph,
  SchemaMap,
  SortSpec,
  TableInfo,
} from "./types";
import { DataGrid } from "./components/DataGrid";
import { RowDetail } from "./components/RowDetail";
import { TableDesigner } from "./components/TableDesigner";
import { cn } from "./lib/cn";

const SqlEditor = lazy(() =>
  import("./components/SqlEditor").then((module) => ({ default: module.SqlEditor })),
);

const DiagramView = lazy(() =>
  import("./components/DiagramView").then((module) => ({ default: module.DiagramView })),
);

const PAGE_SIZES = [100, 200, 500, 1000];
const QUERY_LIMITS = [100, 500, 1000, 5000];
const QUERY_HISTORY_KEY = "dbgo:query-history";

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
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [filterText, setFilterText] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [columns, setColumns] = useState<ColumnInfo[]>([]);
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
  const [view, setView] = useState<"data" | "sql" | "diagram">("data");
  const [sqlText, setSqlText] = useState("");
  const [schema, setSchema] = useState<SchemaMap>({});
  const [schemaGraph, setSchemaGraph] = useState<SchemaGraph | null>(null);
  const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
  const [queryRunning, setQueryRunning] = useState(false);
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
  const requestId = useRef(0);
  const bootstrapped = useRef(false);
  const lastDataVersion = useRef<number | null>(null);

  const pagedMode = sort !== null || dataFilter !== null;

  const loadTable = useCallback(
    async (table: string, cursor: number | null, size = pageSize): Promise<Page | null> => {
      const id = ++requestId.current;
      setLoading(true);
      setError(null);
      try {
        const nextColumns = await api.describeTable(table);
        const result = await api.loadPage(table, cursor, "next", size);
        if (id !== requestId.current) return null;

        setColumns(nextColumns);
        setPage(result);
        setSelected(table);
        setPageIndex(0);
        setSelectedRow(null);
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
        const nextColumns = await api.describeTable(table);
        const result = await api.loadTablePage(
          table,
          activeFilter,
          activeSort,
          index * size,
          size,
        );
        if (id !== requestId.current) return null;

        setColumns(nextColumns);
        setPage(result);
        setSelected(table);
        setPageIndex(index);
        setSelectedRow(null);
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
        }
        return null;
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    },
    [pageSize],
  );

  const applyDbInfo = useCallback((info: DbInfo) => {
    setDb(info);
    setTables(info.tables);
    setSelected(null);
    setColumns([]);
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
    setSort(null);
    setDataFilter(null);
    setFilterDraft({ column: "", op: "contains", value: "" });
    setSelectedRow(null);
    setNotice(null);
    setShowDesigner(false);
  }, []);

  const openPath = useCallback(
    async (path: string) => {
      setLoading(true);
      setError(null);
      try {
        const info = await api.openDatabase(path);
        applyDbInfo(info);
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
      } catch (cause) {
        setError(String(cause));
      } finally {
        setLoading(false);
      }
    },
    [applyDbInfo, loadTable],
  );

  const createNewDatabase = useCallback(async () => {
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
  }, [applyDbInfo]);

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
  }, [openPath]);

  const selectTable = useCallback(
    (table: string) => {
      if (table === selected) return;
      setView("data");
      setSort(null);
      setDataFilter(null);
      setPageIndex(0);
      setSelectedRow(null);
      setFilterDraft((current) => ({ ...current, value: "" }));
      setPage(null);
      setColumns([]);
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);
      void loadTable(table, null);
    },
    [loadTable, selected],
  );

  const toggleSort = useCallback(
    (column: string) => {
      if (!selected) return;
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
    [selected, sort, dataFilter, loadOffsetPage, loadTable],
  );

  const applyFilter = useCallback(() => {
    if (!selected || !filterDraft.column) return;
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
  }, [selected, filterDraft, sort, loadOffsetPage]);

  const clearFilter = useCallback(() => {
    if (!selected) return;
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
  }, [selected, sort, loadOffsetPage, loadTable]);

  const runQuery = useCallback(
    async (text: string) => {
      const statement = text.trim();
      if (!statement) return;

      setSqlText(statement);
      setView("sql");
      setSelectedRow(null);
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

  const nextPage = useCallback(async () => {
    if (!selected || !page) return;
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
  ]);

  const prevPage = useCallback(async () => {
    if (!selected) return;
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
  ]);

  const refresh = useCallback(() => {
    if (view === "diagram") {
      void api
        .schemaGraph()
        .then(setSchemaGraph)
        .catch((cause) => setError(String(cause)));
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
  ]);

  const switchView = useCallback(
    (next: "data" | "sql" | "diagram") => {
      if (next === "sql" && !sqlText.trim() && selected) {
        setSqlText(`SELECT * FROM ${quoteIdent(selected)} LIMIT 200;`);
      }
      if (next === "diagram" && !schemaGraph) {
        void api
          .schemaGraph()
          .then(setSchemaGraph)
          .catch((cause) => setError(String(cause)));
      }
      setSelectedRow(null);
      setView(next);
    },
    [sqlText, selected, schemaGraph],
  );

  const changePageSize = useCallback(
    (size: number) => {
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
    [selected, pagedMode, sort, dataFilter, loadOffsetPage, loadTable],
  );

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
        if (cancelled || loading || queryRunning) return;
        try {
          const version = await api.dataVersion();
          if (cancelled) return;
          if (lastDataVersion.current === null) {
            lastDataVersion.current = version;
            return;
          }
          if (version === lastDataVersion.current) return;
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
    loadTable,
    loadOffsetPage,
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
                    : "Refresh the current page"
              }
            >
              {view === "sql" ? "Run query" : "Refresh"}
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
                onClick={() => setShowDesigner(true)}
                disabled={!db}
                title="New table"
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

          {!db ? (
            <div className="flex flex-1 flex-col items-center justify-center gap-2.5 p-10 text-center">
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
                  onClick={() => setShowDesigner(true)}
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
                      <span className={BADGE_SUBTLE}>{columns.length} columns</span>
                      {count && !dataFilter && (
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
                    </>
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
                      disabled={queryRunning || !sqlText.trim()}
                    >
                      {queryRunning ? "Running..." : "Run"}
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
                        if (value) setSqlText(value);
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
                        onChange={setSqlText}
                        onRun={(value) => void runQuery(value)}
                      />
                    </Suspense>
                  </div>
                </div>
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
                          setView("data");
                          void loadTable(table, null);
                        }}
                        onCreateTable={() => setShowDesigner(true)}
                      />
                    </Suspense>
                  )}
                </div>
              ) : (
                <div className="flex min-h-0 flex-1">
                  <div className="flex min-w-0 flex-1 flex-col">
                    {view === "data" ? (
                      loading && !page ? (
                        <div className="grid flex-1 place-items-center text-xs text-muted">
                          Loading rows...
                        </div>
                      ) : (
                        <DataGrid
                          columns={columns}
                          rows={page?.rows ?? []}
                          sort={sort}
                          onSort={toggleSort}
                          onRowClick={(row, index) => setSelectedRow({ row, index })}
                          selectedIndex={selectedRow?.index ?? null}
                        />
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

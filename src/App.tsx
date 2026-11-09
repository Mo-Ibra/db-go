import { lazy, Suspense, useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./lib/api";
import type {
  ColumnInfo,
  CountResult,
  DbInfo,
  Page,
  QueryResult,
  SchemaMap,
  TableInfo,
} from "./types";
import { DataGrid } from "./components/DataGrid";

const SqlEditor = lazy(() =>
  import("./components/SqlEditor").then((module) => ({ default: module.SqlEditor })),
);

const PAGE_SIZES = [100, 200, 500, 1000];
const QUERY_LIMITS = [100, 500, 1000, 5000];
const QUERY_HISTORY_KEY = "dbgo:query-history";

function quoteIdent(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

export default function App() {
  const [db, setDb] = useState<DbInfo | null>(null);
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [columns, setColumns] = useState<ColumnInfo[]>([]);
  const [page, setPage] = useState<Page | null>(null);
  const [pageSize, setPageSize] = useState(200);
  const [history, setHistory] = useState<(number | null)[]>([null]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [count, setCount] = useState<CountResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<"data" | "sql">("data");
  const [sqlText, setSqlText] = useState("");
  const [schema, setSchema] = useState<SchemaMap>({});
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
  const requestId = useRef(0);
  const bootstrapped = useRef(false);

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

  const openPath = useCallback(
    async (path: string) => {
      setLoading(true);
      setError(null);
      try {
        const info = await api.openDatabase(path);
        setDb(info);
        setTables(info.tables);
        setSelected(null);
        setColumns([]);
        setPage(null);
        setCount(null);
        setFilter("");
        setHistory([null]);
        setHistoryIndex(0);
        setView("data");
        setSqlText("");
        setSchema({});
        setQueryResult(null);
        setQueryRunning(false);

        void api.schemaMap().then(setSchema).catch(() => setSchema({}));

        const firstTable = info.tables[0]?.name;
        if (firstTable) void loadTable(firstTable, null);
      } catch (cause) {
        setError(String(cause));
      } finally {
        setLoading(false);
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
      setPage(null);
      setColumns([]);
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);
      void loadTable(table, null);
    },
    [loadTable, selected],
  );

  const nextPage = useCallback(async () => {
    if (!selected || !page || !page.has_more || page.next_cursor === null) return;
    const cursor = page.next_cursor;
    const result = await loadTable(selected, cursor);
    if (result) {
      const nextHistory = [...history.slice(0, historyIndex + 1), cursor];
      setHistory(nextHistory);
      setHistoryIndex(nextHistory.length - 1);
    }
  }, [selected, page, history, historyIndex, loadTable]);

  const prevPage = useCallback(async () => {
    if (!selected || historyIndex <= 0) return;
    const nextIndex = historyIndex - 1;
    const result = await loadTable(selected, history[nextIndex]);
    if (result) setHistoryIndex(nextIndex);
  }, [selected, history, historyIndex, loadTable]);

  const runQuery = useCallback(
    async (text: string) => {
      const statement = text.trim();
      if (!statement) return;

      setSqlText(statement);
      setView("sql");
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

  const refresh = useCallback(() => {
    if (view === "sql") {
      if (sqlText.trim()) void runQuery(sqlText);
      return;
    }
    if (selected) void loadTable(selected, history[historyIndex]);
  }, [view, sqlText, runQuery, selected, history, historyIndex, loadTable]);

  const switchView = useCallback(
    (next: "data" | "sql") => {
      if (next === "sql" && !sqlText.trim() && selected) {
        setSqlText(`SELECT * FROM ${quoteIdent(selected)} LIMIT 200;`);
      }
      setView(next);
    },
    [sqlText, selected],
  );

  const changePageSize = useCallback(
    (size: number) => {
      setPageSize(size);
      setPage(null);
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);
      if (selected) void loadTable(selected, null, size);
    },
    [selected, loadTable],
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

  const visibleTables = tables.filter((table) =>
    table.name.toLowerCase().includes(filter.trim().toLowerCase()),
  );
  const rowStart = page ? historyIndex * pageSize + 1 : 0;
  const rowEnd = page ? historyIndex * pageSize + page.rows.length : 0;

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark">DB</div>
          <div className="brand-name">DBGo</div>
          <span className="brand-tag">local SQL workbench</span>
        </div>
        <div className="topbar-actions">
          {db && (
            <button
              className="btn ghost"
              onClick={refresh}
              disabled={!selected || loading || queryRunning}
              title={view === "sql" ? "Run the current query" : "Refresh the current page"}
            >
              {view === "sql" ? "Run query" : "Refresh"}
            </button>
          )}
          <button className="btn primary" onClick={() => void openDb()}>
            Open database
          </button>
        </div>
      </header>

      <div className="main">
        <aside className="sidebar">
          <div className="sidebar-head">
            <span className="sidebar-title">Tables</span>
            <span className="badge">{tables.length}</span>
          </div>
          <input
            className="search"
            placeholder="Filter tables..."
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            disabled={!db}
          />
          <div className="table-list">
            {!db && <div className="sidebar-empty">No database open</div>}
            {db && visibleTables.length === 0 && (
              <div className="sidebar-empty">No tables</div>
            )}
            {visibleTables.map((table) => (
              <button
                key={table.name}
                className={`table-item ${selected === table.name ? "active" : ""}`}
                onClick={() => selectTable(table.name)}
                title={table.name}
              >
                <span className="table-icon">▦</span>
                <span className="table-name">{table.name}</span>
              </button>
            ))}
          </div>
          <div className="sidebar-foot">
            {db ? (
              <span className="path" title={db.path}>
                <span className="db-dot" />
                {db.path.split("/").pop()}
              </span>
            ) : (
              <span className="muted">Open a SQLite file to begin</span>
            )}
          </div>
        </aside>

        <main className="content">
          {error && (
            <div className="error-banner">
              <span>{error}</span>
              <button className="icon-btn" onClick={() => setError(null)} aria-label="Dismiss">
                ×
              </button>
            </div>
          )}

          {!db ? (
            <div className="empty">
              <div className="empty-mark">DB</div>
              <h1>Open a SQLite database</h1>
              <p>DBGo is a local-first SQL workbench. Your data never leaves this machine.</p>
              <button className="btn primary large" onClick={() => void openDb()}>
                Open database
              </button>
              <div className="hint">Ctrl / Cmd + O</div>
            </div>
          ) : !selected ? (
            <div className="empty">
              <div className="empty-mark small">▦</div>
              <h2>Select a table</h2>
              <p>Pick a table from the sidebar to browse its rows.</p>
            </div>
          ) : (
            <>
              <div className="toolbar">
                <div className="toolbar-left">
                  <div className="tabs" role="tablist" aria-label="View">
                    <button
                      role="tab"
                      aria-selected={view === "data"}
                      className={`tab ${view === "data" ? "active" : ""}`}
                      onClick={() => switchView("data")}
                    >
                      Data
                    </button>
                    <button
                      role="tab"
                      aria-selected={view === "sql"}
                      className={`tab ${view === "sql" ? "active" : ""}`}
                      onClick={() => switchView("sql")}
                    >
                      SQL
                    </button>
                  </div>
                  <span className="table-title">{selected}</span>
                  <span className="badge subtle">{columns.length} columns</span>
                  {count && (
                    <span className="badge subtle">
                      {count.capped
                        ? `${count.count.toLocaleString()}+`
                        : count.count.toLocaleString()}{" "}
                      rows
                    </span>
                  )}
                </div>
                <div className="toolbar-right">
                  {view === "data" ? (
                    <>
                      <label className="page-size">
                        Rows
                        <select
                          value={pageSize}
                          onChange={(event) => changePageSize(Number(event.target.value))}
                        >
                          {PAGE_SIZES.map((size) => (
                            <option key={size} value={size}>
                              {size}
                            </option>
                          ))}
                        </select>
                      </label>
                      <div className="pager">
                        <button
                          className="btn ghost"
                          onClick={() => void prevPage()}
                          disabled={historyIndex <= 0 || loading}
                        >
                          Previous
                        </button>
                        <span className="page-label">Page {historyIndex + 1}</span>
                        <button
                          className="btn ghost"
                          onClick={() => void nextPage()}
                          disabled={!page?.has_more || loading}
                        >
                          Next
                        </button>
                      </div>
                    </>
                  ) : (
                    <span className="badge subtle">read-only</span>
                  )}
                </div>
              </div>

              {view === "sql" && (
                <div className="sql-panel">
                  <div className="sql-toolbar">
                    <button
                      className="btn primary"
                      onClick={() => void runQuery(sqlText)}
                      disabled={queryRunning || !sqlText.trim()}
                    >
                      {queryRunning ? "Running..." : "Run"}
                    </button>
                    <span className="sql-hint">Ctrl / Cmd + Enter</span>
                    <label className="page-size">
                      Max rows
                      <select
                        value={queryLimit}
                        onChange={(event) => setQueryLimit(Number(event.target.value))}
                      >
                        {QUERY_LIMITS.map((limit) => (
                          <option key={limit} value={limit}>
                            {limit}
                          </option>
                        ))}
                      </select>
                    </label>
                    <select
                      className="history-select"
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
                      <span className="sql-meta">
                        {queryResult.rows.length.toLocaleString()} rows
                        {queryResult.truncated ? " · truncated" : ""}
                      </span>
                    )}
                  </div>
                  <div className="sql-editor-wrap">
                    <Suspense fallback={<div className="loading">Loading editor...</div>}>
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

              <div className="grid-shell">
                {view === "data" ? (
                  loading && !page ? (
                    <div className="loading">Loading rows...</div>
                  ) : (
                    <DataGrid columns={columns} rows={page?.rows ?? []} />
                  )
                ) : queryRunning ? (
                  <div className="loading">Running query...</div>
                ) : queryResult ? (
                  <DataGrid columns={queryResult.columns} rows={queryResult.rows} />
                ) : (
                  <div className="loading">Run a query to see results</div>
                )}
              </div>
            </>
          )}

          <footer className="statusbar">
            <span className="status-left" title={db?.path ?? undefined}>
              {db ? db.path : "No database open"}
            </span>
            <span className="status-right">
              {view === "sql"
                ? queryResult
                  ? `query · ${queryResult.rows.length.toLocaleString()} rows${
                      queryResult.truncated ? " · truncated" : ""
                    }`
                  : "read-only SQL"
                : selected && page
                  ? `rows ${rowStart.toLocaleString()}–${rowEnd.toLocaleString()} · keyset pagination`
                  : selected
                    ? "keyset pagination"
                    : "SQLite"}
            </span>
          </footer>
        </main>
      </div>
    </div>
  );
}

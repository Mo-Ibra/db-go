import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./lib/api";
import type { ColumnInfo, CountResult, DbInfo, Page, TableInfo } from "./types";
import { DataGrid } from "./components/DataGrid";

const PAGE_SIZES = [100, 200, 500, 1000];

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

  const refresh = useCallback(() => {
    if (selected) void loadTable(selected, history[historyIndex]);
  }, [selected, history, historyIndex, loadTable]);

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
              disabled={!selected || loading}
              title="Refresh the current page"
            >
              Refresh
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
                </div>
              </div>

              <div className="grid-shell">
                {loading && !page ? (
                  <div className="loading">Loading rows...</div>
                ) : (
                  <DataGrid columns={columns} rows={page?.rows ?? []} />
                )}
              </div>
            </>
          )}

          <footer className="statusbar">
            <span className="status-left" title={db?.path ?? undefined}>
              {db ? db.path : "No database open"}
            </span>
            <span className="status-right">
              {selected && page
                ? `rows ${rowStart.toLocaleString()}–${rowEnd.toLocaleString()} · `
                : ""}
              {selected ? "keyset pagination" : "SQLite"}
            </span>
          </footer>
        </main>
      </div>
    </div>
  );
}

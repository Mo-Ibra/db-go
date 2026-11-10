import { useCallback } from "react";
import type { MutableRefObject } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { api } from "@/lib/api";
import type {
  CellValue,
  CountResult,
  FilterOp,
  FilterSpec,
  ForeignKeyInfo,
  Page,
  SchemaChange,
  SchemaGraph,
  SchemaMap,
  SortSpec,
  TableDetails,
  TableInfo,
  TableStats,
} from "@/types";

type View = "data" | "structure" | "sql" | "diagram";

type SchemaActionsOptions = {
  setShowDesigner: (show: boolean) => void;
  setShowAlterTable: (show: boolean) => void;
  setTables: (tables: TableInfo[]) => void;
  setSchema: (schema: SchemaMap) => void;
  setSchemaGraph: (graph: SchemaGraph | null) => void;
  setView: (view: View) => void;
  setSort: (sort: SortSpec | null) => void;
  setDataFilter: (filter: FilterSpec | null) => void;
  setFilterDraft: (draft: { column: string; op: FilterOp; value: string }) => void;
  setPageIndex: (index: number) => void;
  resetTableUi: () => void;
  setHistory: (history: (number | null)[]) => void;
  setHistoryIndex: (index: number) => void;
  setSelected: (table: string | null) => void;
  setPage: (page: Page | null) => void;
  setColumns: (columns: import("@/types").ColumnInfo[]) => void;
  setForeignKeys: (keys: ForeignKeyInfo[]) => void;
  setCount: (count: CountResult | null) => void;
  setTableDetails: (details: TableDetails | null) => void;
  setTableStats: (stats: TableStats | null) => void;
  setSelectedRow: (row: null) => void;
  setSqlText: (sql: string) => void;
  setError: (error: string | null) => void;
  setNotice: (notice: string | null) => void;
  guardPending: () => boolean;
  loadTable: (table: string, cursor: number | null) => Promise<Page | null>;
  loadOffsetPage: (
    table: string,
    index: number,
    sort: SortSpec | null,
    filter: FilterSpec | null,
  ) => Promise<Page | null>;
  loadDetails: (table: string) => Promise<void>;
  reloadCurrent: () => Promise<void>;
  runQuery: (sql: string) => Promise<void>;
  lastDataVersion: MutableRefObject<number | null>;
  externalChangePending: MutableRefObject<boolean>;
  selected: string | null;
  view: View;
  pagedMode: boolean;
  pageIndex: number;
  sort: SortSpec | null;
  dataFilter: FilterSpec | null;
  history: (number | null)[];
  historyIndex: number;
  sqlText: string;
  schemaGraph: SchemaGraph | null;
};

function quoteIdent(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

export function useSchemaActions(options: SchemaActionsOptions) {
  const {
    setShowDesigner,
    setShowAlterTable,
    setTables,
    setSchema,
    setSchemaGraph,
    setView,
    setSort,
    setDataFilter,
    setFilterDraft,
    setPageIndex,
    resetTableUi,
    setHistory,
    setHistoryIndex,
    setSelected,
    setPage,
    setColumns,
    setForeignKeys,
    setCount,
    setTableDetails,
    setTableStats,
    setSelectedRow,
    setSqlText,
    setError,
    setNotice,
    guardPending,
    loadTable,
    loadOffsetPage,
    loadDetails,
    reloadCurrent,
    runQuery,
    lastDataVersion,
    externalChangePending,
    selected,
    view,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    history,
    historyIndex,
    sqlText,
    schemaGraph,
  } = options;

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
        resetTableUi();
        setHistory([null]);
        setHistoryIndex(0);
        void loadTable(table, null);
      } catch (cause) {
        setError(String(cause));
      }
    },
    [
      loadTable,
      resetTableUi,
      setDataFilter,
      setError,
      setHistory,
      setHistoryIndex,
      setPageIndex,
      setSchema,
      setSchemaGraph,
      setShowDesigner,
      setSort,
      setTables,
      setView,
    ],
  );

  const requestDesigner = useCallback(() => {
    if (guardPending()) return;
    setShowDesigner(true);
  }, [guardPending, setShowDesigner]);

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
      lastDataVersion,
      externalChangePending,
      setColumns,
      setCount,
      setError,
      setNotice,
      setPage,
      setSchema,
      setSchemaGraph,
      setSelected,
      setTableDetails,
      setTableStats,
      setTables,
      setView,
    ],
  );

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
    void reloadCurrent();
  }, [
    view,
    sqlText,
    runQuery,
    selected,
    guardPending,
    loadDetails,
    reloadCurrent,
    setError,
    setSchemaGraph,
  ]);

  const switchView = useCallback(
    (next: View) => {
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
    [
      sqlText,
      selected,
      schemaGraph,
      loadDetails,
      setError,
      setSchemaGraph,
      setSelectedRow,
      setSqlText,
      setView,
    ],
  );

  const requestAlterTable = useCallback(() => {
    if (!selected) return;
    if (guardPending()) return;
    setShowAlterTable(true);
  }, [selected, guardPending, setShowAlterTable]);

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
      resetTableUi();
      setPage(null);
      setColumns([]);
      setForeignKeys([]);
      setTableStats(null);
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);
      await loadOffsetPage(foreignKey.referenced_table, 0, null, filter);
    },
    [
      guardPending,
      loadOffsetPage,
      resetTableUi,
      setColumns,
      setCount,
      setDataFilter,
      setFilterDraft,
      setForeignKeys,
      setHistory,
      setHistoryIndex,
      setPage,
      setPageIndex,
      setSort,
      setTableStats,
      setView,
    ],
  );

  const openReferencedTable = useCallback(
    (table: string) => {
      if (guardPending()) return;
      setView("data");
      setSort(null);
      setDataFilter(null);
      setFilterDraft({ column: "", op: "contains", value: "" });
      setPageIndex(0);
      resetTableUi();
      setPage(null);
      setColumns([]);
      setForeignKeys([]);
      setTableStats(null);
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);
      void loadTable(table, null);
    },
    [
      guardPending,
      loadTable,
      resetTableUi,
      setColumns,
      setCount,
      setDataFilter,
      setFilterDraft,
      setForeignKeys,
      setHistory,
      setHistoryIndex,
      setPage,
      setPageIndex,
      setSort,
      setTableStats,
      setView,
    ],
  );

  return {
    handleTableCreated,
    requestDesigner,
    refreshAfterSchemaChange,
    refresh,
    switchView,
    requestAlterTable,
    handleAlterApplied,
    handleCreateIndex,
    handleDropIndex,
    openForeignKey,
    openReferencedTable,
  };
}

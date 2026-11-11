import { useCallback } from "react";
import type { MutableRefObject } from "react";
import { api } from "@/lib/api";
import { getSqlDraft } from "@/features/query/sqlDraft";
import type {
  ColumnInfo,
  CountResult,
  FilterSpec,
  ForeignKeyInfo,
  Page,
  SchemaGraph,
  SchemaMap,
  SortSpec,
  TableDetails,
  TableInfo,
  TableStats,
} from "@/types";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

type SchemaRefreshOptions = {
  view: View;
  selected: string | null;
  pagedMode: boolean;
  pageIndex: number;
  sort: SortSpec | null;
  dataFilter: FilterSpec | null;
  history: (number | null)[];
  historyIndex: number;
  loadTable: (table: string, cursor: number | null) => Promise<Page | null>;
  loadOffsetPage: (
    table: string,
    index: number,
    sort: SortSpec | null,
    filter: FilterSpec | null,
  ) => Promise<Page | null>;
  loadDetails: (table: string) => Promise<void>;
  runQuery: (sql: string) => Promise<void>;
  reloadCurrent: () => Promise<void>;
  lastDataVersion: MutableRefObject<number | null>;
  externalChangePending: MutableRefObject<boolean>;
  setTables: (tables: TableInfo[]) => void;
  setSchema: (schema: SchemaMap) => void;
  setSchemaGraph: (graph: SchemaGraph | null) => void;
  setView: (view: View) => void;
  setSelected: (table: string | null) => void;
  setPage: (page: Page | null) => void;
  setColumns: (columns: ColumnInfo[]) => void;
  setForeignKeys?: (keys: ForeignKeyInfo[]) => void;
  setCount: (count: CountResult | null) => void;
  setTableDetails: (details: TableDetails | null) => void;
  setTableStats: (stats: TableStats | null) => void;
  setError: (error: string | null) => void;
  setNotice: (notice: string | null) => void;
  guardPending: () => boolean;
};

/** Refreshes schema state after a mutation or an external change. */
export function useSchemaRefresh(options: SchemaRefreshOptions) {
  const {
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
    runQuery,
    reloadCurrent,
    lastDataVersion,
    externalChangePending,
    setTables,
    setSchema,
    setSchemaGraph,
    setView,
    setSelected,
    setPage,
    setColumns,
    setCount,
    setTableDetails,
    setTableStats,
    setError,
    setNotice,
    guardPending,
  } = options;

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
      const draft = getSqlDraft();
      if (draft.trim()) void runQuery(draft);
      return;
    }
    if (!selected) return;
    void reloadCurrent();
  }, [
    view,
    runQuery,
    selected,
    guardPending,
    loadDetails,
    reloadCurrent,
    setError,
    setSchemaGraph,
  ]);

  return { refreshAfterSchemaChange, refresh };
}

import { useEffect } from "react";
import type { MutableRefObject } from "react";
import { api } from "@/lib/api";
import type {
  ColumnInfo,
  FilterSpec,
  Page,
  SortSpec,
  TableDetails,
  TableInfo,
  TableStats,
} from "@/types";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

type ExternalDataSyncOptions = {
  dbPath: string | null;
  view: View;
  selected: string | null;
  pagedMode: boolean;
  pageIndex: number;
  sort: SortSpec | null;
  dataFilter: FilterSpec | null;
  history: (number | null)[];
  historyIndex: number;
  loading: boolean;
  queryRunning: boolean;
  saving: boolean;
  restoringBackup: string | null;
  hasPending: boolean;
  lastDataVersion: MutableRefObject<number | null>;
  externalChangePending: MutableRefObject<boolean>;
  setNotice: (notice: string | null) => void;
  setTables: (tables: TableInfo[]) => void;
  setSchema: (schema: Record<string, string[]>) => void;
  setSchemaGraph: (
    graph: import("@/types").SchemaGraph | null,
  ) => void;
  setSelected: (table: string | null) => void;
  setPage: (page: Page | null) => void;
  setColumns: (columns: ColumnInfo[]) => void;
  setCount: (count: import("@/types").CountResult | null) => void;
  setTableDetails: (details: TableDetails | null) => void;
  setTableStats: (stats: TableStats | null) => void;
  loadDetails: (table: string) => Promise<void>;
  loadTable: (table: string, cursor: number | null) => Promise<Page | null>;
  loadOffsetPage: (
    table: string,
    index: number,
    sort: SortSpec | null,
    filter: FilterSpec | null,
  ) => Promise<Page | null>;
};

export function useExternalDataSync(options: ExternalDataSyncOptions) {
  const {
    dbPath,
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
    lastDataVersion,
    externalChangePending,
    setNotice,
    setTables,
    setSchema,
    setSchemaGraph,
    setSelected,
    setPage,
    setColumns,
    setCount,
    setTableDetails,
    setTableStats,
    loadDetails,
    loadTable,
    loadOffsetPage,
  } = options;

  useEffect(() => {
    lastDataVersion.current = null;
  }, [dbPath, lastDataVersion]);

  useEffect(() => {
    if (!dbPath) return;
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
          if (view === "cards") {
            setNotice("Database changed by another process. Refresh the cards to reload.");
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
    dbPath,
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
    lastDataVersion,
    externalChangePending,
    setNotice,
    setTables,
    setSchema,
    setSchemaGraph,
    setSelected,
    setPage,
    setColumns,
    setCount,
    setTableDetails,
    setTableStats,
    loadDetails,
    loadTable,
    loadOffsetPage,
  ]);
}

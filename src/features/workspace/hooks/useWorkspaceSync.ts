import { useEffect } from "react";
import type { MutableRefObject } from "react";
import { api } from "@/lib/api";
import type {
  ColumnInfo,
  CountResult,
  DbInfo,
  FilterSpec,
  Page,
  SchemaGraph,
  SchemaMap,
  SortSpec,
  TableDetails,
  TableInfo,
  TableStats,
} from "@/types";
import { useExternalDataSync } from "@/features/workspace/hooks/useExternalDataSync";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

type WorkspaceSyncOptions = {
  db: DbInfo | null;
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
  loadTable: (table: string, cursor: number | null) => Promise<Page | null>;
  loadOffsetPage: (
    table: string,
    index: number,
    sort: SortSpec | null,
    filter: FilterSpec | null,
  ) => Promise<Page | null>;
  loadDetails: (table: string) => Promise<void>;
  setCount: (count: CountResult | null) => void;
  setNotice: (notice: string | null) => void;
  setTables: (tables: TableInfo[]) => void;
  setSchema: (schema: SchemaMap) => void;
  setSchemaGraph: (graph: SchemaGraph | null) => void;
  setSelected: (table: string | null) => void;
  setPage: (page: Page | null) => void;
  setColumns: (columns: ColumnInfo[]) => void;
  setTableDetails: (details: TableDetails | null) => void;
  setTableStats: (stats: TableStats | null) => void;
  initialized: MutableRefObject<boolean>;
  lastDataVersion: MutableRefObject<number | null>;
  externalChangePending: MutableRefObject<boolean>;
  saveReloadRef: MutableRefObject<() => Promise<void>>;
};

/**
 * Effects that keep the workspace in sync with the database on disk:
 * post-save reload, first-open bootstrap, and external change polling.
 */
export function useWorkspaceSync(options: WorkspaceSyncOptions) {
  const {
    db,
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
    loadTable,
    loadOffsetPage,
    loadDetails,
    setCount,
    setNotice,
    setTables,
    setSchema,
    setSchemaGraph,
    setSelected,
    setPage,
    setColumns,
    setTableDetails,
    setTableStats,
    initialized,
    lastDataVersion,
    externalChangePending,
    saveReloadRef,
  } = options;

  useEffect(() => {
    saveReloadRef.current = async () => {
      try {
        lastDataVersion.current = await api.dataVersion();
      } catch {
        // The explicit reload below still refreshes the view.
      }
      externalChangePending.current = false;
      if (!selected) return;
      if (pagedMode) {
        await loadOffsetPage(selected, pageIndex, sort, dataFilter);
      } else {
        await loadTable(selected, history[historyIndex]);
      }
      void api.countRows(selected).then(setCount).catch(() => {});
    };
  }, [
    selected,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    history,
    historyIndex,
    loadTable,
    loadOffsetPage,
    setCount,
    lastDataVersion,
    externalChangePending,
    saveReloadRef,
  ]);

  useEffect(() => {
    if (!db || initialized.current) return;
    initialized.current = true;
    void Promise.all([api.schemaMap(), api.schemaGraph()])
      .then(([nextSchema, nextGraph]) => {
        setSchema(nextSchema);
        setSchemaGraph(nextGraph);
      })
      .catch(() => {
        setSchema({});
        setSchemaGraph(null);
      });
    const firstTable = db?.tables[0]?.name;
    if (firstTable) void loadTable(firstTable, null);
  }, [db, loadTable, initialized, setSchema, setSchemaGraph]);

  useExternalDataSync({
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
  });
}

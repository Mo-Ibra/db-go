import type { MutableRefObject } from "react";
import type {
  ColumnInfo,
  CountResult,
  FilterOp,
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
import { useSchemaMutations } from "@/features/schema/hooks/useSchemaMutations";
import { useSchemaNavigation } from "@/features/schema/hooks/useSchemaNavigation";
import { useSchemaRefresh } from "@/features/schema/hooks/useSchemaRefresh";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

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
  setColumns: (columns: ColumnInfo[]) => void;
  setForeignKeys: (keys: ForeignKeyInfo[]) => void;
  setCount: (count: CountResult | null) => void;
  setTableDetails: (details: TableDetails | null) => void;
  setTableStats: (stats: TableStats | null) => void;
  setSelectedRow: (row: null) => void;
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
  schemaGraph: SchemaGraph | null;
};

/**
 * Composition root for schema actions:
 * refresh + navigation + mutations.
 */
export function useSchemaActions({
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
  schemaGraph,
}: SchemaActionsOptions) {
  const { refreshAfterSchemaChange, refresh } = useSchemaRefresh({
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
  });

  const { switchView, openForeignKey, openReferencedTable } =
    useSchemaNavigation({
      selected,
      schemaGraph,
      loadDetails,
      loadTable,
      loadOffsetPage,
      guardPending,
      resetTableUi,
      setView,
      setSort,
      setDataFilter,
      setFilterDraft,
      setPageIndex,
      setPage,
      setColumns,
      setForeignKeys,
      setCount,
      setTableStats,
      setHistory,
      setHistoryIndex,
      setSelectedRow,
      setSchemaGraph,
      setError,
    });

  const {
    handleTableCreated,
    requestDesigner,
    requestAlterTable,
    handleAlterApplied,
    handleCreateIndex,
    handleDropIndex,
  } = useSchemaMutations({
    selected,
    guardPending,
    loadTable,
    resetTableUi,
    refreshAfterSchemaChange,
    setShowDesigner,
    setShowAlterTable,
    setTables,
    setSchema,
    setSchemaGraph,
    setView,
    setSort,
    setDataFilter,
    setPageIndex,
    setHistory,
    setHistoryIndex,
    setError,
  });

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

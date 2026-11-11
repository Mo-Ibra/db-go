import { useCallback } from "react";
import { api } from "@/lib/api";
import { getSqlDraft, setSqlDraft } from "@/features/query/sqlDraft";
import type {
  CellValue,
  ColumnInfo,
  CountResult,
  FilterSpec,
  ForeignKeyInfo,
  Page,
  SchemaGraph,
  SortSpec,
  TableStats,
} from "@/types";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

type SchemaNavigationOptions = {
  selected: string | null;
  schemaGraph: SchemaGraph | null;
  loadDetails: (table: string) => Promise<void>;
  loadTable: (table: string, cursor: number | null) => Promise<Page | null>;
  loadOffsetPage: (
    table: string,
    index: number,
    sort: SortSpec | null,
    filter: FilterSpec | null,
  ) => Promise<Page | null>;
  guardPending: () => boolean;
  resetTableUi: () => void;
  setView: (view: View) => void;
  setSort: (sort: SortSpec | null) => void;
  setDataFilter: (filter: FilterSpec | null) => void;
  setFilterDraft: (draft: {
    column: string;
    op: FilterSpec["op"];
    value: string;
  }) => void;
  setPageIndex: (index: number) => void;
  setPage: (page: Page | null) => void;
  setColumns: (columns: ColumnInfo[]) => void;
  setForeignKeys: (keys: ForeignKeyInfo[]) => void;
  setCount: (count: CountResult | null) => void;
  setTableStats: (stats: TableStats | null) => void;
  setHistory: (history: (number | null)[]) => void;
  setHistoryIndex: (index: number) => void;
  setSelectedRow: (row: null) => void;
  setSchemaGraph: (graph: SchemaGraph | null) => void;
  setError: (error: string | null) => void;
};

function quoteIdent(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

/** View switching and cross-table navigation. */
export function useSchemaNavigation({
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
}: SchemaNavigationOptions) {
  const switchView = useCallback(
    (next: View) => {
      if (next === "sql" && !getSqlDraft().trim() && selected) {
        setSqlDraft(`SELECT * FROM ${quoteIdent(selected)} LIMIT 200;`);
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
      selected,
      schemaGraph,
      loadDetails,
      setError,
      setSchemaGraph,
      setSelectedRow,
      setView,
    ],
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

  return { switchView, openForeignKey, openReferencedTable };
}

import { useCallback } from "react";
import { isPagedMode } from "@/lib/table/navigation";
import type {
  ColumnInfo,
  CountResult,
  FilterOp,
  FilterSpec,
  ForeignKeyInfo,
  Page,
  SortSpec,
} from "@/types";
import type { FilterDraft } from "@/features/tables/hooks/useTableDataState";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

type TableNavigationOptions = {
  currentView: View;
  guardPending: () => boolean;
  resetTableUi: () => void;
  resetTableDetails: () => void;
  setView: (view: View) => void;
  setLoading?: never;
  // state
  selected: string | null;
  page: Page | null;
  history: (number | null)[];
  historyIndex: number;
  pageIndex: number;
  sort: SortSpec | null;
  dataFilter: FilterSpec | null;
  filterDraft: FilterDraft;
  // setters
  setSelected: (table: string | null) => void;
  setColumns: (columns: ColumnInfo[]) => void;
  setForeignKeys: (keys: ForeignKeyInfo[]) => void;
  setCount: (count: CountResult | null) => void;
  setPage: (page: Page | null) => void;
  setPageSize: (size: number) => void;
  setSort: (sort: SortSpec | null) => void;
  setDataFilter: (filter: FilterSpec | null) => void;
  setPageIndex: (index: number) => void;
  setFilterDraft: (updater: (current: FilterDraft) => FilterDraft) => void;
  setHistory: (updater: ((current: (number | null)[]) => (number | null)[]) | (number | null)[]) => void;
  setHistoryIndex: (updater: ((current: number) => number) | number) => void;
  // loaders
  loadTable: (
    table: string,
    cursor: number | null,
    size?: number,
  ) => Promise<Page | null>;
  loadOffsetPage: (
    table: string,
    index: number,
    sort: SortSpec | null,
    filter: FilterSpec | null,
    size?: number,
  ) => Promise<Page | null>;
};

function isNullOp(op: FilterOp): boolean {
  return op === "is_null" || op === "not_null";
}

/** Sort, filter, and pagination actions for the table screen. */
export function useTableNavigation({
  currentView,
  guardPending,
  resetTableUi,
  resetTableDetails,
  setView,
  selected,
  page,
  history,
  historyIndex,
  pageIndex,
  sort,
  dataFilter,
  filterDraft,
  setSelected,
  setColumns,
  setForeignKeys,
  setCount,
  setPage,
  setPageSize,
  setSort,
  setDataFilter,
  setPageIndex,
  setFilterDraft,
  setHistory,
  setHistoryIndex,
  loadTable,
  loadOffsetPage,
}: TableNavigationOptions) {
  const pagedMode = isPagedMode(sort, dataFilter);

  const selectTable = useCallback(
    (table: string) => {
      if (table === selected) return;
      if (guardPending()) return;

      setSort(null);
      setDataFilter(null);
      setPageIndex(0);
      resetTableUi();
      setFilterDraft((current) => ({ ...current, value: "" }));
      setPage(null);
      setColumns([]);
      setForeignKeys([]);
      resetTableDetails();
      setCount(null);
      setHistory([null]);
      setHistoryIndex(0);

      if (currentView === "cards") {
        setSelected(table);
        return;
      }

      setView("data");
      void loadTable(table, null);
    },
    [
      currentView,
      guardPending,
      loadTable,
      resetTableDetails,
      resetTableUi,
      selected,
      setSelected,
      setView,
      setSort,
      setDataFilter,
      setPageIndex,
      setFilterDraft,
      setPage,
      setColumns,
      setForeignKeys,
      setCount,
      setHistory,
      setHistoryIndex,
    ],
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
      resetTableUi();
      setPage(null);

      if (nextSort || dataFilter) {
        void loadOffsetPage(selected, 0, nextSort, dataFilter);
      } else {
        setHistory([null]);
        setHistoryIndex(0);
        void loadTable(selected, null);
      }
    },
    [
      selected,
      sort,
      dataFilter,
      loadOffsetPage,
      loadTable,
      guardPending,
      resetTableUi,
      setSort,
      setPageIndex,
      setPage,
      setHistory,
      setHistoryIndex,
    ],
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
    resetTableUi();
    setPage(null);
    void loadOffsetPage(selected, 0, sort, nextFilter);
  }, [
    selected,
    filterDraft,
    sort,
    loadOffsetPage,
    guardPending,
    resetTableUi,
    setDataFilter,
    setPageIndex,
    setPage,
  ]);

  const clearFilter = useCallback(() => {
    if (!selected) return;
    if (guardPending()) return;
    setDataFilter(null);
    setPageIndex(0);
    resetTableUi();
    setPage(null);
    if (sort) {
      void loadOffsetPage(selected, 0, sort, null);
    } else {
      setHistory([null]);
      setHistoryIndex(0);
      void loadTable(selected, null);
    }
  }, [
    selected,
    sort,
    loadOffsetPage,
    loadTable,
    guardPending,
    resetTableUi,
    setDataFilter,
    setPageIndex,
    setPage,
    setHistory,
    setHistoryIndex,
  ]);

  const nextPage = useCallback(async () => {
    if (!selected || !page?.has_more || pagedMode) return;
    if (guardPending()) return;
    const cursor = page.next_cursor;
    if (cursor === null) return;
    const result = await loadTable(selected, cursor);
    if (result) {
      setHistory((current) => [...current, cursor]);
      setHistoryIndex((current) => current + 1);
    }
  }, [
    selected,
    page,
    pagedMode,
    loadTable,
    guardPending,
    setHistory,
    setHistoryIndex,
  ]);

  const prevPage = useCallback(async () => {
    if (!selected || pageIndex <= 0) return;
    if (guardPending()) return;
    if (pagedMode) {
      void loadOffsetPage(selected, pageIndex - 1, sort, dataFilter);
      return;
    }
    if (historyIndex <= 0) return;
    const cursor = history[historyIndex - 1] ?? null;
    await loadTable(selected, cursor);
    setHistoryIndex((current) => Math.max(0, current - 1));
  }, [
    selected,
    pageIndex,
    pagedMode,
    loadOffsetPage,
    sort,
    dataFilter,
    historyIndex,
    history,
    loadTable,
    guardPending,
    setHistoryIndex,
  ]);

  const changePageSize = useCallback(
    (size: number) => {
      setPageSize(size);
      if (!selected) return;
      if (pagedMode) {
        void loadOffsetPage(selected, 0, sort, dataFilter, size);
      } else {
        setHistory([null]);
        setHistoryIndex(0);
        void loadTable(selected, null, size);
      }
    },
    [
      selected,
      pagedMode,
      sort,
      dataFilter,
      loadOffsetPage,
      loadTable,
      setPageSize,
      setHistory,
      setHistoryIndex,
    ],
  );

  const reloadTable = useCallback(
    async (table: string) => {
      if (pagedMode) {
        await loadOffsetPage(table, pageIndex, sort, dataFilter);
      } else {
        await loadTable(table, history[historyIndex]);
      }
    },
    [
      pagedMode,
      pageIndex,
      sort,
      dataFilter,
      history,
      historyIndex,
      loadOffsetPage,
      loadTable,
    ],
  );

  const reloadCurrent = useCallback(async () => {
    if (selected) await reloadTable(selected);
  }, [selected, reloadTable]);

  const clearTable = useCallback(() => {
    setSelected(null);
    setPage(null);
    setColumns([]);
    setCount(null);
  }, [setSelected, setPage, setColumns, setCount]);

  return {
    pagedMode,
    selectTable,
    toggleSort,
    applyFilter,
    clearFilter,
    nextPage,
    prevPage,
    changePageSize,
    reloadTable,
    reloadCurrent,
    clearTable,
  };
}

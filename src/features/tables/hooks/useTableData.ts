import {
  currentPageIndex,
  nextPageDisabled,
  previousPageDisabled,
  rowBounds,
} from "@/lib/table/navigation";
import { useTableDataState } from "@/features/tables/hooks/useTableDataState";
import { useTableNavigation } from "@/features/tables/hooks/useTableNavigation";
import { useTablePageLoader } from "@/features/tables/hooks/useTablePageLoader";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

type TableDataOptions = {
  currentView: View;
  guardPending: () => boolean;
  resetTableUi: () => void;
  resetTableDetails: () => void;
  setView: (view: View) => void;
  setError: (error: string | null) => void;
};

/**
 * Composition root for the table screen:
 * state container + page loading + navigation.
 */
export function useTableData({
  currentView,
  guardPending,
  resetTableUi,
  resetTableDetails,
  setView,
  setError,
}: TableDataOptions) {
  const state = useTableDataState();

  const { loadTable, loadOffsetPage } = useTablePageLoader({
    pageSize: state.pageSize,
    setLoading: state.setLoading,
    setError,
    setColumns: state.setColumns,
    setForeignKeys: state.setForeignKeys,
    setPage: state.setPage,
    setSelected: state.setSelected,
    setPageIndex: state.setPageIndex,
    setCount: state.setCount,
    setFilterDraft: state.setFilterDraft,
    resetTableUi,
  });

  const navigation = useTableNavigation({
    currentView,
    guardPending,
    resetTableUi,
    resetTableDetails,
    setView,
    selected: state.selected,
    page: state.page,
    history: state.history,
    historyIndex: state.historyIndex,
    pageIndex: state.pageIndex,
    sort: state.sort,
    dataFilter: state.dataFilter,
    filterDraft: state.filterDraft,
    setSelected: state.setSelected,
    setColumns: state.setColumns,
    setForeignKeys: state.setForeignKeys,
    setCount: state.setCount,
    setPage: state.setPage,
    setPageSize: state.setPageSize,
    setSort: state.setSort,
    setDataFilter: state.setDataFilter,
    setPageIndex: state.setPageIndex,
    setFilterDraft: state.setFilterDraft,
    setHistory: state.setHistory,
    setHistoryIndex: state.setHistoryIndex,
    loadTable,
    loadOffsetPage,
  });

  const currentPage = currentPageIndex(
    navigation.pagedMode,
    state.pageIndex,
    state.historyIndex,
  );
  const { start: rowStart, end: rowEnd } = rowBounds(
    state.page,
    currentPage,
    state.pageSize,
  );

  return {
    selected: state.selected,
    setSelected: state.setSelected,
    columns: state.columns,
    setColumns: state.setColumns,
    foreignKeys: state.foreignKeys,
    setForeignKeys: state.setForeignKeys,
    page: state.page,
    setPage: state.setPage,
    pageSize: state.pageSize,
    history: state.history,
    setHistory: state.setHistory,
    historyIndex: state.historyIndex,
    setHistoryIndex: state.setHistoryIndex,
    pageIndex: state.pageIndex,
    setPageIndex: state.setPageIndex,
    count: state.count,
    setCount: state.setCount,
    loading: state.loading,
    sort: state.sort,
    setSort: state.setSort,
    dataFilter: state.dataFilter,
    setDataFilter: state.setDataFilter,
    filterDraft: state.filterDraft,
    setFilterDraft: state.setFilterDraft,
    pagedMode: navigation.pagedMode,
    currentPage,
    rowStart,
    rowEnd,
    nextDisabled: nextPageDisabled(state.loading, state.page),
    prevDisabled: previousPageDisabled(
      state.loading,
      navigation.pagedMode,
      state.pageIndex,
      state.historyIndex,
    ),
    loadTable,
    loadOffsetPage,
    selectTable: navigation.selectTable,
    toggleSort: navigation.toggleSort,
    applyFilter: navigation.applyFilter,
    clearFilter: navigation.clearFilter,
    nextPage: navigation.nextPage,
    prevPage: navigation.prevPage,
    changePageSize: navigation.changePageSize,
    reloadTable: navigation.reloadTable,
    reloadCurrent: navigation.reloadCurrent,
    clearTable: navigation.clearTable,
  };
}

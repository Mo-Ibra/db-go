import { useCallback, useRef, useState } from "react";
import { api } from "@/lib/api";
import {
  currentPageIndex,
  isPagedMode,
  nextPageDisabled,
  previousPageDisabled,
  rowBounds,
} from "@/lib/table/navigation";
import type {
  ColumnInfo,
  CountResult,
  FilterOp,
  FilterSpec,
  ForeignKeyInfo,
  Page,
  SortSpec,
} from "@/types";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

type TableDataOptions = {
  currentView: View;
  guardPending: () => boolean;
  resetTableUi: () => void;
  resetTableDetails: () => void;
  setView: (view: View) => void;
  setError: (error: string | null) => void;
};

function isNullOp(op: FilterOp): boolean {
  return op === "is_null" || op === "not_null";
}

function syncFilterColumn(
  current: { column: string; op: FilterOp; value: string },
  columns: ColumnInfo[],
) {
  if (columns.some((column) => column.name === current.column)) return current;
  return { ...current, column: columns[0]?.name ?? "" };
}

export function useTableData({
  currentView,
  guardPending,
  resetTableUi,
  resetTableDetails,
  setView,
  setError,
}: TableDataOptions) {
  const [selected, setSelected] = useState<string | null>(null);
  const [columns, setColumns] = useState<ColumnInfo[]>([]);
  const [foreignKeys, setForeignKeys] = useState<ForeignKeyInfo[]>([]);
  const [page, setPage] = useState<Page | null>(null);
  const [pageSize, setPageSize] = useState(200);
  const [history, setHistory] = useState<(number | null)[]>([null]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [pageIndex, setPageIndex] = useState(0);
  const [count, setCount] = useState<CountResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [sort, setSort] = useState<SortSpec | null>(null);
  const [dataFilter, setDataFilter] = useState<FilterSpec | null>(null);
  const [filterDraft, setFilterDraft] = useState<{
    column: string;
    op: FilterOp;
    value: string;
  }>({ column: "", op: "contains", value: "" });
  const requestId = useRef(0);

  const loadTable = useCallback(
    async (
      table: string,
      cursor: number | null,
      size = pageSize,
    ): Promise<Page | null> => {
      const id = ++requestId.current;
      setLoading(true);
      setError(null);
      try {
        const [nextColumns, nextForeignKeys, result] = await Promise.all([
          api.describeTable(table),
          api.foreignKeys(table),
          api.loadPage(table, cursor, "next", size),
        ]);
        if (id !== requestId.current) return null;

        setColumns(nextColumns);
        setForeignKeys(nextForeignKeys);
        setPage(result);
        setSelected(table);
        setPageIndex(0);
        resetTableUi();
        setFilterDraft((current) => syncFilterColumn(current, nextColumns));

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
          setForeignKeys([]);
        }
        return null;
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    },
    [pageSize, resetTableUi, setError],
  );

  const loadOffsetPage = useCallback(
    async (
      table: string,
      index: number,
      activeSort: SortSpec | null,
      activeFilter: FilterSpec | null,
      size = pageSize,
    ): Promise<Page | null> => {
      const id = ++requestId.current;
      setLoading(true);
      setError(null);
      try {
        const [nextColumns, nextForeignKeys, result] = await Promise.all([
          api.describeTable(table),
          api.foreignKeys(table),
          api.loadTablePage(table, activeFilter, activeSort, index * size, size),
        ]);
        if (id !== requestId.current) return null;

        setColumns(nextColumns);
        setForeignKeys(nextForeignKeys);
        setPage(result);
        setSelected(table);
        setPageIndex(index);
        resetTableUi();
        setFilterDraft((current) => syncFilterColumn(current, nextColumns));

        if (activeFilter) {
          setCount(null);
        } else {
          void api
            .countRows(table)
            .then((nextCount) => {
              if (id === requestId.current) setCount(nextCount);
            })
            .catch(() => {
              if (id === requestId.current) setCount(null);
            });
        }

        return result;
      } catch (cause) {
        if (id === requestId.current) {
          setError(String(cause));
          setPage(null);
          setForeignKeys([]);
        }
        return null;
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    },
    [pageSize, resetTableUi, setError],
  );

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
  }, [selected, page, pagedMode, loadTable, guardPending]);

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
  }, []);

  const currentPage = currentPageIndex(pagedMode, pageIndex, historyIndex);
  const { start: rowStart, end: rowEnd } = rowBounds(page, currentPage, pageSize);

  return {
    selected,
    setSelected,
    columns,
    setColumns,
    foreignKeys,
    setForeignKeys,
    page,
    setPage,
    pageSize,
    history,
    setHistory,
    historyIndex,
    setHistoryIndex,
    pageIndex,
    setPageIndex,
    count,
    setCount,
    loading,
    sort,
    setSort,
    dataFilter,
    setDataFilter,
    filterDraft,
    setFilterDraft,
    pagedMode,
    currentPage,
    rowStart,
    rowEnd,
    nextDisabled: nextPageDisabled(loading, page),
    prevDisabled: previousPageDisabled(loading, pagedMode, pageIndex, historyIndex),
    loadTable,
    loadOffsetPage,
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

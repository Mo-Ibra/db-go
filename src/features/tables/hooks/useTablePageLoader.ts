import { useCallback, useRef } from "react";
import { api } from "@/lib/api";
import type {
  ColumnInfo,
  CountResult,
  FilterSpec,
  ForeignKeyInfo,
  Page,
  SortSpec,
} from "@/types";
import type { FilterDraft } from "@/features/tables/hooks/useTableDataState";

type TablePageLoaderOptions = {
  pageSize: number;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setColumns: (columns: ColumnInfo[]) => void;
  setForeignKeys: (keys: ForeignKeyInfo[]) => void;
  setPage: (page: Page | null) => void;
  setSelected: (table: string) => void;
  setPageIndex: (index: number) => void;
  setCount: (count: CountResult | null) => void;
  setFilterDraft: (updater: (current: FilterDraft) => FilterDraft) => void;
  resetTableUi: () => void;
};

function syncFilterColumn(current: FilterDraft, columns: ColumnInfo[]): FilterDraft {
  if (columns.some((column) => column.name === current.column)) return current;
  return { ...current, column: columns[0]?.name ?? "" };
}

/** Loads keyset pages and offset pages for the table screen. */
export function useTablePageLoader({
  pageSize,
  setLoading,
  setError,
  setColumns,
  setForeignKeys,
  setPage,
  setSelected,
  setPageIndex,
  setCount,
  setFilterDraft,
  resetTableUi,
}: TablePageLoaderOptions) {
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
    [
      pageSize,
      resetTableUi,
      setError,
      setLoading,
      setColumns,
      setForeignKeys,
      setPage,
      setSelected,
      setPageIndex,
      setCount,
      setFilterDraft,
    ],
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
    [
      pageSize,
      resetTableUi,
      setError,
      setLoading,
      setColumns,
      setForeignKeys,
      setPage,
      setSelected,
      setPageIndex,
      setCount,
      setFilterDraft,
    ],
  );

  return { loadTable, loadOffsetPage };
}

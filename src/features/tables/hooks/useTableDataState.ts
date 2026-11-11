import { useState } from "react";
import type {
  ColumnInfo,
  CountResult,
  FilterOp,
  FilterSpec,
  ForeignKeyInfo,
  Page,
  SortSpec,
} from "@/types";

export type FilterDraft = {
  column: string;
  op: FilterOp;
  value: string;
};

/** Plain state container for the table screen (no async work). */
export function useTableDataState() {
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
  const [filterDraft, setFilterDraft] = useState<FilterDraft>({
    column: "",
    op: "contains",
    value: "",
  });

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
    setPageSize,
    history,
    setHistory,
    historyIndex,
    setHistoryIndex,
    pageIndex,
    setPageIndex,
    count,
    setCount,
    loading,
    setLoading,
    sort,
    setSort,
    dataFilter,
    setDataFilter,
    filterDraft,
    setFilterDraft,
  };
}

export type TableDataState = ReturnType<typeof useTableDataState>;

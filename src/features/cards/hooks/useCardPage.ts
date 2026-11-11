import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";
import type { CardTemplate } from "@/features/cards/model";
import type { ColumnInfo, CountResult, FilterSpec, Row, SortSpec } from "@/types";

type UseCardPageOptions = {
  table: string | null;
  columns: ColumnInfo[];
  template: CardTemplate | null;
  page: number;
  pageSize: number;
  search: string;
  sort: SortSpec | null;
};

export function useCardPage({
  table,
  columns,
  template,
  page,
  pageSize,
  search,
  sort,
}: UseCardPageOptions) {
  const [rows, setRows] = useState<Row[]>([]);
  const [count, setCount] = useState<CountResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestId = useRef(0);

  const load = useCallback(async () => {
    if (!table || !template || columns.length === 0) {
      setRows([]);
      setCount(null);
      setLoading(false);
      return;
    }

    const id = ++requestId.current;
    setLoading(true);
    setError(null);
    const searchField =
      template.searchFields[0] ?? template.card.title?.column ?? null;
    const filter: FilterSpec | null =
      search.trim() && searchField
        ? { column: searchField, op: "contains", value: search.trim() }
        : null;

    try {
      const [pageResult, nextCount] = await Promise.all([
        api.loadTablePage(table, filter, sort, page * pageSize, pageSize),
        filter ? Promise.resolve(null) : api.countRows(table),
      ]);
      if (id !== requestId.current) return;
      setRows(pageResult.rows);
      setCount(nextCount);
    } catch (cause) {
      if (id !== requestId.current) return;
      setError(String(cause));
      setRows([]);
      setCount(null);
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  }, [table, template, columns.length, search, sort, page, pageSize]);

  useEffect(() => {
    void load();
  }, [load]);

  return {
    rows,
    count,
    loading,
    error,
    reload: load,
    hasMore: rows.length === pageSize,
  };
}

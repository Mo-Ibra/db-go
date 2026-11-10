import { useCallback, useEffect, useMemo, useState } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";
import {
  clipboardCellText,
  rowSelectionKey,
} from "@/lib/cellValues";
import { copyText } from "@/lib/clipboard";
import type { ColumnInfo, Row } from "@/types";

type RowSelectionOptions = {
  displayRows: Row[];
  columns: ColumnInfo[];
  view: "data" | "structure" | "sql" | "diagram";
  setNotice: (notice: string | null) => void;
  onDeleteRows: (rows: Row[]) => string;
};

export function useRowSelection({
  displayRows,
  columns,
  view,
  setNotice,
  onDeleteRows,
}: RowSelectionOptions) {
  const [selectedRow, setSelectedRow] = useState<{ row: Row; index: number } | null>(null);
  const [selectionKeys, setSelectionKeys] = useState<Set<string>>(new Set());
  const [lastSelectedIndex, setLastSelectedIndex] = useState<number | null>(null);

  const visibleSelectionKeys = useMemo(
    () => displayRows.map((row, index) => rowSelectionKey(row, index)),
    [displayRows],
  );
  const allSelected =
    visibleSelectionKeys.length > 0 &&
    visibleSelectionKeys.every((key) => selectionKeys.has(key));
  const someSelected = visibleSelectionKeys.some((key) => selectionKeys.has(key));

  useEffect(() => {
    setSelectedRow((current) => {
      if (!current || view !== "data") return current;
      const next = displayRows[current.index];
      if (!next) return null;
      return next === current.row ? current : { row: next, index: current.index };
    });
  }, [displayRows, view]);

  const resetSelection = useCallback(() => {
    setSelectionKeys(new Set());
    setLastSelectedIndex(null);
  }, []);

  const resetTableUi = useCallback(() => {
    setSelectedRow(null);
    setSelectionKeys(new Set());
    setLastSelectedIndex(null);
  }, []);

  const clearSelection = useCallback(() => {
    setSelectionKeys(new Set());
    setLastSelectedIndex(null);
  }, []);

  const removeSelectionKey = useCallback((row: Row, index: number) => {
    const key = rowSelectionKey(row, index);
    setSelectionKeys((current) => {
      const next = new Set(current);
      next.delete(key);
      return next;
    });
  }, []);

  const toggleRowSelection = useCallback(
    (row: Row, index: number, event: ReactMouseEvent) => {
      const key = rowSelectionKey(row, index);

      if (event.shiftKey && lastSelectedIndex !== null) {
        const start = Math.min(lastSelectedIndex, index);
        const end = Math.max(lastSelectedIndex, index);
        const range = displayRows
          .slice(start, end + 1)
          .map((candidate, offset) => rowSelectionKey(candidate, start + offset));
        setSelectionKeys((current) => {
          const next = new Set(current);
          range.forEach((rangeKey) => next.add(rangeKey));
          return next;
        });
        setLastSelectedIndex(index);
        return;
      }

      setSelectionKeys((current) => {
        const next = new Set(current);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        return next;
      });
      setLastSelectedIndex(index);
    },
    [displayRows, lastSelectedIndex],
  );

  const toggleAllSelection = useCallback(() => {
    setSelectionKeys((current) => {
      const next = new Set(current);
      if (allSelected) {
        visibleSelectionKeys.forEach((key) => next.delete(key));
      } else {
        visibleSelectionKeys.forEach((key) => next.add(key));
      }
      return next;
    });
    setLastSelectedIndex(null);
  }, [allSelected, visibleSelectionKeys]);

  const copySelectedRows = useCallback(async () => {
    const rows = displayRows.filter((row, index) =>
      selectionKeys.has(rowSelectionKey(row, index)),
    );
    if (rows.length === 0) {
      setNotice("Select rows to copy first.");
      return;
    }
    const sanitize = (value: string) =>
      value.replace(/\t/g, " ").replace(/\r?\n/g, " ");
    const header = columns.map((column) => sanitize(column.name)).join("\t");
    const lines = rows.map((row) =>
      row.cells.map((cell) => sanitize(clipboardCellText(cell))).join("\t"),
    );
    await copyText([header, ...lines].join("\n"));
    setNotice(`Copied ${rows.length} row(s)`);
  }, [displayRows, selectionKeys, columns, setNotice]);

  const deleteSelectedRows = useCallback(() => {
    const rows = displayRows.filter((row, index) =>
      selectionKeys.has(rowSelectionKey(row, index)),
    );
    if (rows.length === 0) return;
    const message = onDeleteRows(rows);
    if (message) setNotice(message);
    clearSelection();
  }, [displayRows, selectionKeys, onDeleteRows, clearSelection, setNotice]);

  return {
    selectedRow,
    setSelectedRow,
    selectionKeys,
    lastSelectedIndex,
    visibleSelectionKeys,
    allSelected,
    someSelected,
    resetSelection,
    resetTableUi,
    clearSelection,
    removeSelectionKey,
    toggleRowSelection,
    toggleAllSelection,
    copySelectedRows,
    deleteSelectedRows,
  };
}

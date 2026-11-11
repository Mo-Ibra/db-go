import { useCallback, useMemo, useState } from "react";
import { mergePendingRows } from "@/lib/table/pendingEdits";
import type { ColumnInfo, Page, Row } from "@/types";
import type {
  PendingInsert,
  PendingUpdate,
  RowEditorState,
} from "@/lib/table/pendingEdits";

type PendingStoreOptions = {
  page: Page | null;
  columns: ColumnInfo[];
};

/**
 * Owns the staged-edit state for the current table page.
 * Pure state container: no API calls, no notifications.
 */
export function usePendingStore({ page, columns }: PendingStoreOptions) {
  const [pendingInserts, setPendingInserts] = useState<PendingInsert[]>([]);
  const [pendingUpdates, setPendingUpdates] = useState<PendingUpdate[]>([]);
  const [pendingDeletes, setPendingDeletes] = useState<number[]>([]);
  const [rowEditor, setRowEditor] = useState<RowEditorState | null>(null);
  const [saving, setSaving] = useState(false);

  const hasPending =
    pendingInserts.length + pendingUpdates.length + pendingDeletes.length > 0;

  const pendingRowIds = useMemo(
    () => new Set(pendingUpdates.map((update) => update.rowid)),
    [pendingUpdates],
  );

  const displayRows = useMemo(
    () =>
      mergePendingRows(
        page?.rows ?? [],
        columns,
        pendingUpdates,
        pendingDeletes,
        pendingInserts,
      ),
    [page, columns, pendingUpdates, pendingDeletes, pendingInserts],
  );

  const discardPending = useCallback(() => {
    setPendingInserts([]);
    setPendingUpdates([]);
    setPendingDeletes([]);
    setRowEditor(null);
  }, []);

  const removePendingRow = useCallback((row: Row) => {
    if (row.pendingKey) {
      setPendingInserts((current) =>
        current.filter((insert) => insert.key !== row.pendingKey),
      );
    } else if (row.rowid !== null) {
      const rowid = row.rowid;
      setPendingUpdates((current) =>
        current.filter((update) => update.rowid !== rowid),
      );
      setPendingDeletes((current) =>
        current.includes(rowid) ? current : [...current, rowid],
      );
    }
  }, []);

  const deleteRows = useCallback((rows: Row[]) => {
    const insertKeys = new Set<string>();
    const rowids = new Set<number>();
    for (const row of rows) {
      if (row.pendingKey) insertKeys.add(row.pendingKey);
      else if (row.rowid !== null) rowids.add(row.rowid);
    }

    if (insertKeys.size > 0) {
      setPendingInserts((current) =>
        current.filter((insert) => !insertKeys.has(insert.key)),
      );
    }
    if (rowids.size > 0) {
      setPendingUpdates((current) =>
        current.filter((update) => !rowids.has(update.rowid)),
      );
      setPendingDeletes((current) => {
        const next = new Set(current);
        rowids.forEach((rowid) => next.add(rowid));
        return [...next];
      });
    }

    const parts = [];
    if (rowids.size > 0) parts.push(`staged ${rowids.size} row(s) for deletion`);
    if (insertKeys.size > 0) parts.push(`removed ${insertKeys.size} pending insert(s)`);
    return parts.join(" · ");
  }, []);

  return {
    pendingInserts,
    setPendingInserts,
    pendingUpdates,
    setPendingUpdates,
    pendingDeletes,
    setPendingDeletes,
    rowEditor,
    setRowEditor,
    saving,
    setSaving,
    hasPending,
    pendingRowIds,
    displayRows,
    discardPending,
    removePendingRow,
    deleteRows,
  };
}

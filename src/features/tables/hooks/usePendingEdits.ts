import { useCallback } from "react";
import { isBlobColumn } from "@/lib/table/pendingEdits";
import type { ColumnInfo, Page, Row } from "@/types";
import { usePendingMutations } from "@/features/tables/hooks/usePendingMutations";
import { useRowSelection } from "@/features/tables/hooks/useRowSelection";

type PendingEditsOptions = {
  page: Page | null;
  columns: ColumnInfo[];
  selected: string | null;
  view: "data" | "structure" | "sql" | "diagram";
  restoringBackup: string | null;
  setNotice: (notice: string | null) => void;
  setError: (error: string | null) => void;
  onAfterSave: () => Promise<void>;
};

export function usePendingEdits({
  page,
  columns,
  selected,
  view,
  restoringBackup,
  setNotice,
  setError,
  onAfterSave,
}: PendingEditsOptions) {
  const mutations = usePendingMutations({
    page,
    columns,
    selected,
    restoringBackup,
    setNotice,
    setError,
    onAfterSave,
  });

  const selection = useRowSelection({
    displayRows: mutations.displayRows,
    columns,
    view,
    setNotice,
    onDeleteRows: mutations.deleteRows,
  });

  const canEditCell = useCallback(
    (row: Row, column: ColumnInfo) => {
      if (view !== "data") return false;
      if (column.primary_key || isBlobColumn(column)) return false;
      return row.pendingKey !== undefined || row.rowid !== null;
    },
    [view],
  );

  const discardChanges = useCallback(() => {
    mutations.discardPending();
    selection.resetTableUi();
    setNotice("Discarded unsaved changes.");
  }, [mutations.discardPending, selection.resetTableUi, setNotice]);

  const editSelectedRow = useCallback(() => {
    const row = selection.selectedRow?.row;
    if (!row) return;
    mutations.setRowEditor({ mode: row.pendingKey ? "insert" : "update", row });
  }, [selection.selectedRow?.row, mutations.setRowEditor]);

  const deleteSelectedRow = useCallback(() => {
    const row = selection.selectedRow?.row;
    if (!row) return;
    mutations.removePendingRow(row);
    selection.removeSelectionKey(row, selection.selectedRow!.index);
    selection.setSelectedRow(null);
  }, [
    selection.selectedRow,
    selection.removeSelectionKey,
    selection.setSelectedRow,
    mutations.removePendingRow,
  ]);

  return {
    ...mutations,
    handleCellCommit: mutations.applyCellCommit,
    ...selection,
    canEditCell,
    discardChanges,
    editSelectedRow,
    deleteSelectedRow,
  };
}

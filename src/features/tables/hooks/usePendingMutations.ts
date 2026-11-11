import type { ColumnInfo, Page } from "@/types";
import { usePendingClipboard } from "@/features/tables/hooks/usePendingClipboard";
import { usePendingCommit } from "@/features/tables/hooks/usePendingCommit";
import { usePendingStore } from "@/features/tables/hooks/usePendingStore";

type PendingMutationsOptions = {
  page: Page | null;
  columns: ColumnInfo[];
  selected: string | null;
  restoringBackup: string | null;
  setNotice: (notice: string | null) => void;
  setError: (error: string | null) => void;
  onAfterSave: () => Promise<void>;
};

/**
 * Composition root for staged table edits:
 * store (state) + clipboard (cell/paste) + commit (guard/save).
 */
export function usePendingMutations({
  page,
  columns,
  selected,
  restoringBackup,
  setNotice,
  setError,
  onAfterSave,
}: PendingMutationsOptions) {
  const store = usePendingStore({ page, columns });

  const clipboard = usePendingClipboard({
    columns,
    displayRows: store.displayRows,
    setPendingInserts: store.setPendingInserts,
    setPendingUpdates: store.setPendingUpdates,
    setNotice,
    setError,
  });

  const commit = usePendingCommit({
    selected,
    columns,
    restoringBackup,
    saving: store.saving,
    setSaving: store.setSaving,
    hasPending: store.hasPending,
    pendingInserts: store.pendingInserts,
    pendingUpdates: store.pendingUpdates,
    pendingDeletes: store.pendingDeletes,
    setPendingInserts: store.setPendingInserts,
    setPendingUpdates: store.setPendingUpdates,
    setPendingDeletes: store.setPendingDeletes,
    rowEditor: store.rowEditor,
    setRowEditor: store.setRowEditor,
    setNotice,
    setError,
    onAfterSave,
  });

  return {
    pendingInserts: store.pendingInserts,
    pendingUpdates: store.pendingUpdates,
    pendingDeletes: store.pendingDeletes,
    rowEditor: store.rowEditor,
    setRowEditor: store.setRowEditor,
    saving: store.saving,
    hasPending: store.hasPending,
    pendingRowIds: store.pendingRowIds,
    displayRows: store.displayRows,
    guardPending: commit.guardPending,
    discardPending: store.discardPending,
    applyCellCommit: clipboard.applyCellCommit,
    pasteIntoCell: clipboard.pasteIntoCell,
    saveChanges: commit.saveChanges,
    submitRowEditor: commit.submitRowEditor,
    removePendingRow: store.removePendingRow,
    deleteRows: store.deleteRows,
  };
}

import { useCallback } from "react";
import { api } from "@/lib/api";
import {
  makePendingKey,
  rowToValues,
  valuesEqual,
} from "@/lib/table/pendingEdits";
import type { ColumnInfo, JsonObject, RowUpdate } from "@/types";
import type {
  PendingInsert,
  PendingUpdate,
  RowEditorState,
} from "@/lib/table/pendingEdits";

type PendingCommitOptions = {
  selected: string | null;
  columns: ColumnInfo[];
  restoringBackup: string | null;
  saving: boolean;
  setSaving: (saving: boolean) => void;
  hasPending: boolean;
  pendingInserts: PendingInsert[];
  pendingUpdates: PendingUpdate[];
  pendingDeletes: number[];
  setPendingInserts: (
    updater: (current: PendingInsert[]) => PendingInsert[],
  ) => void;
  setPendingUpdates: (
    updater: (current: PendingUpdate[]) => PendingUpdate[],
  ) => void;
  setPendingDeletes: (updater: (current: number[]) => number[]) => void;
  rowEditor: RowEditorState | null;
  setRowEditor: (state: RowEditorState | null) => void;
  setNotice: (notice: string | null) => void;
  setError: (error: string | null) => void;
  onAfterSave: () => Promise<void>;
};

/** Guards and persistence for staged edits. */
export function usePendingCommit({
  selected,
  columns,
  restoringBackup,
  saving,
  setSaving,
  hasPending,
  pendingInserts,
  pendingUpdates,
  pendingDeletes,
  setPendingInserts,
  setPendingUpdates,
  setPendingDeletes,
  rowEditor,
  setRowEditor,
  setNotice,
  setError,
  onAfterSave,
}: PendingCommitOptions) {
  const guardPending = useCallback((): boolean => {
    if (saving || restoringBackup !== null) {
      setNotice(
        restoringBackup !== null
          ? "A restore is in progress. Wait for it to finish."
          : "A save is in progress. Wait for it to finish.",
      );
      return true;
    }
    if (!hasPending) return false;
    setNotice("Save or discard your pending changes first.");
    return true;
  }, [hasPending, saving, restoringBackup, setNotice]);

  const saveChanges = useCallback(async () => {
    if (!selected || !hasPending || saving) return;
    setSaving(true);
    setError(null);
    try {
      const updates: RowUpdate[] = pendingUpdates.map((update) => ({
        rowid: update.rowid,
        values: update.values,
      }));
      const inserts = pendingInserts.map((insert) => insert.values);
      const result = await api.saveChanges(selected, updates, inserts, pendingDeletes);

      setPendingInserts(() => []);
      setPendingUpdates(() => []);
      setPendingDeletes(() => []);
      setRowEditor(null);

      const parts = [
        `Saved ${result.affected} change${result.affected === 1 ? "" : "s"}`,
        `session ${result.session_id}`,
      ];
      if (result.backup_path) parts.push(`backup ${result.backup_path}`);
      setNotice(parts.join(" · "));

      await onAfterSave();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setSaving(false);
    }
  }, [
    selected,
    hasPending,
    saving,
    setSaving,
    pendingUpdates,
    pendingInserts,
    pendingDeletes,
    setPendingInserts,
    setPendingUpdates,
    setPendingDeletes,
    setRowEditor,
    onAfterSave,
    setNotice,
    setError,
  ]);

  const submitRowEditor = useCallback(
    (values: JsonObject) => {
      const editor = rowEditor;
      if (!editor) return;

      if (editor.mode === "insert") {
        const pendingKey = editor.row?.pendingKey;
        if (pendingKey) {
          setPendingInserts((current) =>
            current.map((insert) =>
              insert.key === pendingKey ? { ...insert, values } : insert,
            ),
          );
        } else {
          setPendingInserts((current) => [
            ...current,
            { key: makePendingKey(), values },
          ]);
        }
        setRowEditor(null);
        return;
      }

      const row = editor.row;
      if (row.pendingKey) {
        setPendingInserts((current) =>
          current.map((insert) =>
            insert.key === row.pendingKey ? { ...insert, values } : insert,
          ),
        );
      } else if (row.rowid !== null) {
        const rowid = row.rowid;
        setPendingUpdates((current) => {
          const existing = current.find((update) => update.rowid === rowid);
          const before = existing?.before ?? rowToValues(columns, row);
          const changed: JsonObject = {};
          for (const [name, value] of Object.entries(values)) {
            if (!valuesEqual(value, before[name])) changed[name] = value;
          }
          if (Object.keys(changed).length === 0) {
            return current.filter((update) => update.rowid !== rowid);
          }
          if (existing) {
            return current.map((update) =>
              update.rowid === rowid ? { ...update, values: changed } : update,
            );
          }
          return [...current, { rowid, values: changed, before }];
        });
      }
      setRowEditor(null);
    },
    [rowEditor, columns, setPendingInserts, setPendingUpdates, setRowEditor],
  );

  return { guardPending, saveChanges, submitRowEditor };
}

import { useCallback, useMemo, useState } from "react";
import { api } from "@/lib/api";
import {
  parseCellInput,
  parseClipboardRows,
} from "@/lib/cellValues";
import {
  isBlobColumn,
  makePendingKey,
  mergePendingRows,
  mergePendingUpdate,
  rowToValues,
  valuesEqual,
} from "@/lib/table/pendingEdits";
import type {
  ColumnInfo,
  JsonObject,
  Page,
  Row,
  RowUpdate,
} from "@/types";
import type {
  PendingInsert,
  PendingUpdate,
  RowEditorState,
} from "@/lib/table/pendingEdits";

type PendingMutationsOptions = {
  page: Page | null;
  columns: ColumnInfo[];
  selected: string | null;
  restoringBackup: string | null;
  setNotice: (notice: string | null) => void;
  setError: (error: string | null) => void;
  onAfterSave: () => Promise<void>;
};

export function usePendingMutations({
  page,
  columns,
  selected,
  restoringBackup,
  setNotice,
  setError,
  onAfterSave,
}: PendingMutationsOptions) {
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

  const discardPending = useCallback(() => {
    setPendingInserts([]);
    setPendingUpdates([]);
    setPendingDeletes([]);
    setRowEditor(null);
  }, []);

  const applyCellCommit = useCallback(
    (
      row: Row,
      _rowIndex: number,
      column: ColumnInfo,
      _columnIndex: number,
      text: string,
    ): string | null => {
      let value: unknown;
      try {
        value = parseCellInput(column, text);
      } catch (cause) {
        return cause instanceof Error ? cause.message : String(cause);
      }

      if (row.pendingKey) {
        setPendingInserts((current) =>
          current.map((insert) =>
            insert.key === row.pendingKey
              ? { ...insert, values: { ...insert.values, [column.name]: value } }
              : insert,
          ),
        );
        return null;
      }
      if (row.rowid === null) return "This row cannot be edited.";

      const rowid = row.rowid;
      setPendingUpdates((current) =>
        mergePendingUpdate(
          current,
          rowid,
          { [column.name]: value },
          rowToValues(columns, row),
        ),
      );
      return null;
    },
    [columns],
  );

  const pasteIntoCell = useCallback(
    (text: string, cell: { rowIndex: number; columnIndex: number }) => {
      const matrix = parseClipboardRows(text);
      if (matrix.length === 0) return;

      const targetColumns = columns.slice(cell.columnIndex);
      if (targetColumns.length === 0) return;

      const firstRow = matrix[0];
      const headerMatches =
        matrix.length > 1 &&
        firstRow.length > 0 &&
        targetColumns
          .slice(0, firstRow.length)
          .every(
            (column, index) =>
              firstRow[index]?.trim().toLowerCase() === column.name.toLowerCase(),
          );
      const dataRows = headerMatches ? matrix.slice(1) : matrix;
      if (dataRows.length === 0) return;

      const targetRows = displayRows.slice(
        cell.rowIndex,
        cell.rowIndex + dataRows.length,
      );
      if (targetRows.length === 0) return;

      const changes: {
        row: Row;
        rowid: number | null;
        pendingKey?: string;
        values: JsonObject;
      }[] = [];

      for (let rowIndex = 0; rowIndex < targetRows.length; rowIndex += 1) {
        const row = targetRows[rowIndex];
        const pastedRow = dataRows[rowIndex];
        if (!pastedRow) break;
        const values: JsonObject = {};
        for (
          let columnIndex = 0;
          columnIndex < Math.min(pastedRow.length, targetColumns.length);
          columnIndex += 1
        ) {
          const column = targetColumns[columnIndex];
          if (column.primary_key || isBlobColumn(column)) {
            setNotice(`Cannot paste into ${column.name}.`);
            return;
          }
          try {
            values[column.name] = parseCellInput(column, pastedRow[columnIndex]);
          } catch (cause) {
            setError(cause instanceof Error ? cause.message : String(cause));
            return;
          }
        }
        if (Object.keys(values).length > 0) {
          changes.push({
            row,
            rowid: row.rowid,
            pendingKey: row.pendingKey,
            values,
          });
        }
      }

      if (changes.length === 0) return;

      const insertChanges = new Map<string, JsonObject>();
      for (const change of changes) {
        if (!change.pendingKey) continue;
        insertChanges.set(change.pendingKey, {
          ...(insertChanges.get(change.pendingKey) ?? {}),
          ...change.values,
        });
      }
      if (insertChanges.size > 0) {
        setPendingInserts((current) =>
          current.map((insert) => {
            const values = insertChanges.get(insert.key);
            return values
              ? { ...insert, values: { ...insert.values, ...values } }
              : insert;
          }),
        );
      }

      const existingChanges = changes.filter(
        (change) => !change.pendingKey && change.rowid !== null,
      );
      if (existingChanges.length > 0) {
        setPendingUpdates((current) => {
          let next = current;
          for (const change of existingChanges) {
            const rowid = change.rowid;
            if (rowid === null) continue;
            const currentUpdate = next.find((update) => update.rowid === rowid);
            const before =
              currentUpdate?.before ?? rowToValues(columns, change.row);
            const merged: JsonObject = {
              ...(currentUpdate?.values ?? {}),
              ...change.values,
            };
            const changed: JsonObject = {};
            for (const [name, candidate] of Object.entries(merged)) {
              if (!valuesEqual(candidate, before[name])) changed[name] = candidate;
            }
            if (Object.keys(changed).length === 0) {
              next = next.filter((update) => update.rowid !== rowid);
            } else if (currentUpdate) {
              next = next.map((update) =>
                update.rowid === rowid ? { ...update, values: changed } : update,
              );
            } else {
              next = [...next, { rowid, values: changed, before }];
            }
          }
          return next;
        });
      }

      setNotice(`Pasted ${changes.length} row(s) into pending changes`);
    },
    [columns, displayRows, setNotice, setError],
  );

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

      setPendingInserts([]);
      setPendingUpdates([]);
      setPendingDeletes([]);
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
    pendingUpdates,
    pendingInserts,
    pendingDeletes,
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
          setPendingInserts((current) => [...current, { key: makePendingKey(), values }]);
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
    [rowEditor, columns],
  );

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
    pendingUpdates,
    pendingDeletes,
    rowEditor,
    setRowEditor,
    saving,
    hasPending,
    pendingRowIds,
    displayRows,
    guardPending,
    discardPending,
    applyCellCommit,
    pasteIntoCell,
    saveChanges,
    submitRowEditor,
    removePendingRow,
    deleteRows,
  };
}

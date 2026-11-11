import { useCallback } from "react";
import { parseCellInput, parseClipboardRows } from "@/lib/cellValues";
import {
  isBlobColumn,
  mergePendingUpdate,
  rowToValues,
  valuesEqual,
} from "@/lib/table/pendingEdits";
import type { ColumnInfo, JsonObject, Row } from "@/types";
import type {
  PendingInsert,
  PendingUpdate,
} from "@/lib/table/pendingEdits";

type PendingClipboardOptions = {
  columns: ColumnInfo[];
  displayRows: Row[];
  setPendingInserts: (
    updater: (current: PendingInsert[]) => PendingInsert[],
  ) => void;
  setPendingUpdates: (
    updater: (current: PendingUpdate[]) => PendingUpdate[],
  ) => void;
  setNotice: (notice: string | null) => void;
  setError: (error: string | null) => void;
};

/** Cell edits and clipboard paste, both staged into the pending store. */
export function usePendingClipboard({
  columns,
  displayRows,
  setPendingInserts,
  setPendingUpdates,
  setNotice,
  setError,
}: PendingClipboardOptions) {
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
    [columns, setPendingInserts, setPendingUpdates],
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
    [
      columns,
      displayRows,
      setNotice,
      setError,
      setPendingInserts,
      setPendingUpdates,
    ],
  );

  return { applyCellCommit, pasteIntoCell };
}

import { useCallback } from "react";
import type { Row } from "@/types";
import { DataGrid } from "@/features/tables/components/DataGrid";
import { RowDetail } from "@/features/tables/components/RowDetail";
import { Copy, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import {
  useWorkspacePendingSlice,
  useWorkspaceQuerySlice,
  useWorkspaceSelector,
  useWorkspaceTableSlice,
} from "@/app/providers/WorkspaceContext";

export function ResultView() {
  const view = useWorkspaceSelector((workspace) => workspace.view);
  const setError = useWorkspaceSelector((workspace) => workspace.setError);
  const detailColumns = useWorkspaceSelector(
    (workspace) => workspace.detailColumns,
  );
  const openForeignKey = useWorkspaceSelector(
    (workspace) => workspace.openForeignKey,
  );
  const { columns, foreignKeys, page, loading, sort, toggleSort } =
    useWorkspaceTableSlice();
  const {
    selectedRow,
    setSelectedRow,
    selectionKeys,
    pendingRowIds,
    displayRows,
    allSelected,
    someSelected,
    clearSelection,
    toggleRowSelection,
    toggleAllSelection,
    copySelectedRows,
    deleteSelectedRows,
    handleCellCommit,
    canEditCell,
    pasteIntoCell,
    editSelectedRow,
    deleteSelectedRow,
  } = useWorkspacePendingSlice();
  const { queryResult, queryRunning } = useWorkspaceQuerySlice();

  const handleRowClick = useCallback(
    (_row: Row, index: number) => {
      const row = displayRows[index];
      if (row) setSelectedRow({ row, index });
    },
    [displayRows, setSelectedRow],
  );
  const handleQueryRowClick = useCallback(
    (row: Row, index: number) => setSelectedRow({ row, index }),
    [setSelectedRow],
  );
  const handleCopy = useCallback(() => void copySelectedRows(), [copySelectedRows]);
  const handleClipboardError = useCallback(
    (message: string) => setError(message),
    [setError],
  );
  const handleCloseDetail = useCallback(
    () => setSelectedRow(null),
    [setSelectedRow],
  );
  const handleNavigateForeignKey = useCallback(
    (
      foreignKey: Parameters<typeof openForeignKey>[0],
      cell: Parameters<typeof openForeignKey>[1],
    ) => void openForeignKey(foreignKey, cell),
    [openForeignKey],
  );

  return (
    (view === "data" || view === "sql") && (
      <div className="flex min-h-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col">
          {view === "data" ? (
            loading && !page ? (
              <div className="grid flex-1 place-items-center text-xs text-muted">
                Loading rows...
              </div>
            ) : (
              <>
                {selectionKeys.size > 0 && (
                  <div
                    data-testid="selection-bar"
                    className="flex h-9 flex-none items-center justify-between border-b border-line bg-accent/[0.04] px-3"
                  >
                    <span className="text-[11px] font-semibold text-[#7ee787]">
                      {selectionKeys.size} selected
                    </span>
                    <div className="flex items-center gap-1.5">
                      <Button
                        variant="ghost"
                        size="sm"
                        icon={Copy}
                        onClick={() => void copySelectedRows()}
                        data-testid="copy-selection"
                      >
                        Copy TSV
                      </Button>
                      <Button
                        variant="danger"
                        size="sm"
                        icon={Trash2}
                        onClick={deleteSelectedRows}
                        data-testid="delete-selection"
                      >
                        Delete
                      </Button>
                      <IconButton
                        icon={X}
                        label="Clear selection"
                        size="sm"
                        onClick={clearSelection}
                        data-testid="clear-selection"
                      />
                    </div>
                  </div>
                )}
                <div className="flex min-h-0 flex-1 flex-col">
                  <DataGrid
                    columns={columns}
                    rows={displayRows}
                    sort={sort}
                    onSort={toggleSort}
                    onRowClick={handleRowClick}
                    selectedIndex={selectedRow?.index ?? null}
                    dirtyRowIds={pendingRowIds}
                    selectionKeys={selectionKeys}
                    allSelected={allSelected}
                    someSelected={someSelected}
                    onToggleRow={toggleRowSelection}
                    onToggleAll={toggleAllSelection}
                    onCopy={handleCopy}
                    onPaste={pasteIntoCell}
                    onClipboardError={handleClipboardError}
                    onEditError={handleClipboardError}
                    canEditCell={canEditCell}
                    onCommitCell={handleCellCommit}
                  />
                </div>
              </>
            )
          ) : queryRunning ? (
            <div className="grid flex-1 place-items-center text-xs text-muted">
              Running query...
            </div>
          ) : queryResult ? (
            <DataGrid
              columns={queryResult.columns}
              rows={queryResult.rows}
              onRowClick={handleQueryRowClick}
              selectedIndex={selectedRow?.index ?? null}
            />
          ) : (
            <div className="grid flex-1 place-items-center text-xs text-muted">
              Run a query to see results
            </div>
          )}
        </div>
        {selectedRow && detailColumns.length > 0 && (
          <RowDetail
            columns={detailColumns}
            row={selectedRow.row}
            index={selectedRow.index}
            onClose={handleCloseDetail}
            pending={
              view === "data" &&
              (selectedRow.row.pendingKey !== undefined ||
                (selectedRow.row.rowid !== null &&
                  pendingRowIds.has(selectedRow.row.rowid)))
            }
            onEdit={
              view === "data" &&
                (selectedRow.row.pendingKey !== undefined ||
                  selectedRow.row.rowid !== null)
                ? editSelectedRow
                : undefined
            }
            onDelete={
              view === "data" &&
                (selectedRow.row.pendingKey !== undefined ||
                  selectedRow.row.rowid !== null)
                ? deleteSelectedRow
                : undefined
            }
            foreignKeys={view === "data" ? foreignKeys : undefined}
            onNavigateForeignKey={
              view === "data" ? handleNavigateForeignKey : undefined
            }
          />
        )}
      </div>
    )
  );
}

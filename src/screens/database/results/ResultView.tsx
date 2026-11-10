import { DataGrid } from "@/features/tables/components/DataGrid";
import { RowDetail } from "@/features/tables/components/RowDetail";
import { Copy, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { useWorkspace } from "@/app/providers/WorkspaceContext";

export function ResultView() {
  const {
    setError,
    view,
    columns,
    foreignKeys,
    page,
    loading,
    sort,
    toggleSort,
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
    queryResult,
    queryRunning,
    detailColumns,
    openForeignKey,
  } = useWorkspace();

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
                    onRowClick={(_, index) =>
                      setSelectedRow({ row: displayRows[index], index })
                    }
                    selectedIndex={selectedRow?.index ?? null}
                    dirtyRowIds={pendingRowIds}
                    selectionKeys={selectionKeys}
                    allSelected={allSelected}
                    someSelected={someSelected}
                    onToggleRow={toggleRowSelection}
                    onToggleAll={toggleAllSelection}
                    onCopy={() => void copySelectedRows()}
                    onPaste={pasteIntoCell}
                    onClipboardError={(message) => setError(message)}
                    onEditError={(message) => setError(message)}
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
              onRowClick={(row, index) => setSelectedRow({ row, index })}
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
            onClose={() => setSelectedRow(null)}
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
              view === "data"
                ? (foreignKey, cell) => void openForeignKey(foreignKey, cell)
                : undefined
            }
          />
        )}
      </div>
    )
  );
}

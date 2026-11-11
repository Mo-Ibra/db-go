import { createContext, useContextSelector } from "use-context-selector";
import type { ReactNode } from "react";
import type { useWorkspaceController } from "@/features/workspace/hooks/useWorkspaceController";

export type WorkspaceController = ReturnType<typeof useWorkspaceController>;

const WorkspaceContext = createContext<WorkspaceController>(null!);

export function WorkspaceProvider({
  value,
  children,
}: {
  value: WorkspaceController;
  children: ReactNode;
}) {
  return (
    <WorkspaceContext.Provider value={value}>
      {children}
    </WorkspaceContext.Provider>
  );
}

export function useWorkspace(): WorkspaceController {
  return useContextSelector(WorkspaceContext, (workspace) => workspace);
}

export function useWorkspaceSelector<T>(
  selector: (workspace: WorkspaceController) => T,
): T {
  return useContextSelector(WorkspaceContext, selector);
}

export function useWorkspaceNavigationSlice() {
  return {
    view: useWorkspaceSelector((workspace) => workspace.view),
    switchView: useWorkspaceSelector((workspace) => workspace.switchView),
  };
}

export function useWorkspaceSessionSlice() {
  return {
    db: useWorkspaceSelector((workspace) => workspace.db),
    sessionLoading: useWorkspaceSelector((workspace) => workspace.sessionLoading),
    error: useWorkspaceSelector((workspace) => workspace.error),
    setError: useWorkspaceSelector((workspace) => workspace.setError),
    notice: useWorkspaceSelector((workspace) => workspace.notice),
    setNotice: useWorkspaceSelector((workspace) => workspace.setNotice),
    recentDatabases: useWorkspaceSelector((workspace) => workspace.recentDatabases),
    missingRecents: useWorkspaceSelector((workspace) => workspace.missingRecents),
    clearRecentDatabases: useWorkspaceSelector(
      (workspace) => workspace.clearRecentDatabases,
    ),
    openRecentDatabase: useWorkspaceSelector(
      (workspace) => workspace.openRecentDatabase,
    ),
    removeRecentDatabase: useWorkspaceSelector(
      (workspace) => workspace.removeRecentDatabase,
    ),
    openDb: useWorkspaceSelector((workspace) => workspace.openDb),
    createNewDatabase: useWorkspaceSelector(
      (workspace) => workspace.createNewDatabase,
    ),
  };
}

export function useWorkspaceShellSlice() {
  return {
    db: useWorkspaceSelector((workspace) => workspace.db),
    tables: useWorkspaceSelector((workspace) => workspace.tables),
    setTables: useWorkspaceSelector((workspace) => workspace.setTables),
    view: useWorkspaceSelector((workspace) => workspace.view),
    setView: useWorkspaceSelector((workspace) => workspace.setView),
    schema: useWorkspaceSelector((workspace) => workspace.schema),
    setSchema: useWorkspaceSelector((workspace) => workspace.setSchema),
    schemaGraph: useWorkspaceSelector((workspace) => workspace.schemaGraph),
    setSchemaGraph: useWorkspaceSelector((workspace) => workspace.setSchemaGraph),
    showDesigner: useWorkspaceSelector((workspace) => workspace.showDesigner),
    setShowDesigner: useWorkspaceSelector((workspace) => workspace.setShowDesigner),
    showAlterTable: useWorkspaceSelector((workspace) => workspace.showAlterTable),
    setShowAlterTable: useWorkspaceSelector(
      (workspace) => workspace.setShowAlterTable,
    ),
    busy: useWorkspaceSelector((workspace) => workspace.busy),
    detailColumns: useWorkspaceSelector((workspace) => workspace.detailColumns),
  };
}

export function useWorkspaceTableSlice() {
  return {
    selected: useWorkspaceSelector((workspace) => workspace.selected),
    setSelected: useWorkspaceSelector((workspace) => workspace.setSelected),
    columns: useWorkspaceSelector((workspace) => workspace.columns),
    setColumns: useWorkspaceSelector((workspace) => workspace.setColumns),
    foreignKeys: useWorkspaceSelector((workspace) => workspace.foreignKeys),
    setForeignKeys: useWorkspaceSelector((workspace) => workspace.setForeignKeys),
    page: useWorkspaceSelector((workspace) => workspace.page),
    setPage: useWorkspaceSelector((workspace) => workspace.setPage),
    pageSize: useWorkspaceSelector((workspace) => workspace.pageSize),
    history: useWorkspaceSelector((workspace) => workspace.history),
    historyIndex: useWorkspaceSelector((workspace) => workspace.historyIndex),
    pageIndex: useWorkspaceSelector((workspace) => workspace.pageIndex),
    setPageIndex: useWorkspaceSelector((workspace) => workspace.setPageIndex),
    count: useWorkspaceSelector((workspace) => workspace.count),
    setCount: useWorkspaceSelector((workspace) => workspace.setCount),
    loading: useWorkspaceSelector((workspace) => workspace.loading),
    sort: useWorkspaceSelector((workspace) => workspace.sort),
    setSort: useWorkspaceSelector((workspace) => workspace.setSort),
    dataFilter: useWorkspaceSelector((workspace) => workspace.dataFilter),
    setDataFilter: useWorkspaceSelector((workspace) => workspace.setDataFilter),
    filterDraft: useWorkspaceSelector((workspace) => workspace.filterDraft),
    setFilterDraft: useWorkspaceSelector((workspace) => workspace.setFilterDraft),
    pagedMode: useWorkspaceSelector((workspace) => workspace.pagedMode),
    currentPage: useWorkspaceSelector((workspace) => workspace.currentPage),
    rowStart: useWorkspaceSelector((workspace) => workspace.rowStart),
    rowEnd: useWorkspaceSelector((workspace) => workspace.rowEnd),
    nextDisabled: useWorkspaceSelector((workspace) => workspace.nextDisabled),
    prevDisabled: useWorkspaceSelector((workspace) => workspace.prevDisabled),
    loadTable: useWorkspaceSelector((workspace) => workspace.loadTable),
    loadOffsetPage: useWorkspaceSelector((workspace) => workspace.loadOffsetPage),
    selectTable: useWorkspaceSelector((workspace) => workspace.selectTable),
    toggleSort: useWorkspaceSelector((workspace) => workspace.toggleSort),
    applyFilter: useWorkspaceSelector((workspace) => workspace.applyFilter),
    clearFilter: useWorkspaceSelector((workspace) => workspace.clearFilter),
    nextPage: useWorkspaceSelector((workspace) => workspace.nextPage),
    prevPage: useWorkspaceSelector((workspace) => workspace.prevPage),
    changePageSize: useWorkspaceSelector((workspace) => workspace.changePageSize),
    reloadCurrent: useWorkspaceSelector((workspace) => workspace.reloadCurrent),
  };
}

export function useWorkspacePendingSlice() {
  return {
    selectedRow: useWorkspaceSelector((workspace) => workspace.selectedRow),
    setSelectedRow: useWorkspaceSelector((workspace) => workspace.setSelectedRow),
    selectionKeys: useWorkspaceSelector((workspace) => workspace.selectionKeys),
    pendingInserts: useWorkspaceSelector((workspace) => workspace.pendingInserts),
    pendingUpdates: useWorkspaceSelector((workspace) => workspace.pendingUpdates),
    pendingDeletes: useWorkspaceSelector((workspace) => workspace.pendingDeletes),
    rowEditor: useWorkspaceSelector((workspace) => workspace.rowEditor),
    setRowEditor: useWorkspaceSelector((workspace) => workspace.setRowEditor),
    saving: useWorkspaceSelector((workspace) => workspace.saving),
    hasPending: useWorkspaceSelector((workspace) => workspace.hasPending),
    pendingRowIds: useWorkspaceSelector((workspace) => workspace.pendingRowIds),
    displayRows: useWorkspaceSelector((workspace) => workspace.displayRows),
    allSelected: useWorkspaceSelector((workspace) => workspace.allSelected),
    someSelected: useWorkspaceSelector((workspace) => workspace.someSelected),
    guardPending: useWorkspaceSelector((workspace) => workspace.guardPending),
    resetTableUi: useWorkspaceSelector((workspace) => workspace.resetTableUi),
    discardChanges: useWorkspaceSelector((workspace) => workspace.discardChanges),
    clearSelection: useWorkspaceSelector((workspace) => workspace.clearSelection),
    toggleRowSelection: useWorkspaceSelector(
      (workspace) => workspace.toggleRowSelection,
    ),
    toggleAllSelection: useWorkspaceSelector(
      (workspace) => workspace.toggleAllSelection,
    ),
    copySelectedRows: useWorkspaceSelector(
      (workspace) => workspace.copySelectedRows,
    ),
    deleteSelectedRows: useWorkspaceSelector(
      (workspace) => workspace.deleteSelectedRows,
    ),
    handleCellCommit: useWorkspaceSelector(
      (workspace) => workspace.handleCellCommit,
    ),
    canEditCell: useWorkspaceSelector((workspace) => workspace.canEditCell),
    pasteIntoCell: useWorkspaceSelector((workspace) => workspace.pasteIntoCell),
    saveChanges: useWorkspaceSelector((workspace) => workspace.saveChanges),
    submitRowEditor: useWorkspaceSelector(
      (workspace) => workspace.submitRowEditor,
    ),
    editSelectedRow: useWorkspaceSelector(
      (workspace) => workspace.editSelectedRow,
    ),
    deleteSelectedRow: useWorkspaceSelector(
      (workspace) => workspace.deleteSelectedRow,
    ),
  };
}

export function useWorkspaceQuerySlice() {
  return {
    queryResult: useWorkspaceSelector((workspace) => workspace.queryResult),
    queryRunning: useWorkspaceSelector((workspace) => workspace.queryRunning),
    queryPlan: useWorkspaceSelector((workspace) => workspace.queryPlan),
    setQueryPlan: useWorkspaceSelector((workspace) => workspace.setQueryPlan),
    explainRunning: useWorkspaceSelector((workspace) => workspace.explainRunning),
    queryLimit: useWorkspaceSelector((workspace) => workspace.queryLimit),
    setQueryLimit: useWorkspaceSelector((workspace) => workspace.setQueryLimit),
    queryHistory: useWorkspaceSelector((workspace) => workspace.queryHistory),
    runQuery: useWorkspaceSelector((workspace) => workspace.runQuery),
    explainQuery: useWorkspaceSelector((workspace) => workspace.explainQuery),
    changeSqlText: useWorkspaceSelector((workspace) => workspace.changeSqlText),
  };
}

export function useWorkspaceSchemaSlice() {
  return {
    tableDetails: useWorkspaceSelector((workspace) => workspace.tableDetails),
    setTableDetails: useWorkspaceSelector(
      (workspace) => workspace.setTableDetails,
    ),
    tableStats: useWorkspaceSelector((workspace) => workspace.tableStats),
    setTableStats: useWorkspaceSelector((workspace) => workspace.setTableStats),
    detailsLoading: useWorkspaceSelector(
      (workspace) => workspace.detailsLoading,
    ),
    loadDetails: useWorkspaceSelector((workspace) => workspace.loadDetails),
    resetTableDetails: useWorkspaceSelector(
      (workspace) => workspace.resetTableDetails,
    ),
    requestDesigner: useWorkspaceSelector(
      (workspace) => workspace.requestDesigner,
    ),
    requestAlterTable: useWorkspaceSelector(
      (workspace) => workspace.requestAlterTable,
    ),
    handleAlterApplied: useWorkspaceSelector(
      (workspace) => workspace.handleAlterApplied,
    ),
    handleCreateIndex: useWorkspaceSelector(
      (workspace) => workspace.handleCreateIndex,
    ),
    handleDropIndex: useWorkspaceSelector(
      (workspace) => workspace.handleDropIndex,
    ),
    openReferencedTable: useWorkspaceSelector(
      (workspace) => workspace.openReferencedTable,
    ),
  };
}

export function useWorkspaceDialogSlice() {
  return {
    showMigrations: useWorkspaceSelector((workspace) => workspace.showMigrations),
    setShowMigrations: useWorkspaceSelector(
      (workspace) => workspace.setShowMigrations,
    ),
    migrations: useWorkspaceSelector((workspace) => workspace.migrations),
    migrationsLoading: useWorkspaceSelector(
      (workspace) => workspace.migrationsLoading,
    ),
    migrationsApplying: useWorkspaceSelector(
      (workspace) => workspace.migrationsApplying,
    ),
    migrationsError: useWorkspaceSelector(
      (workspace) => workspace.migrationsError,
    ),
    showBackups: useWorkspaceSelector((workspace) => workspace.showBackups),
    setShowBackups: useWorkspaceSelector(
      (workspace) => workspace.setShowBackups,
    ),
    showImport: useWorkspaceSelector((workspace) => workspace.showImport),
    setShowImport: useWorkspaceSelector((workspace) => workspace.setShowImport),
    showD1Export: useWorkspaceSelector((workspace) => workspace.showD1Export),
    setShowD1Export: useWorkspaceSelector(
      (workspace) => workspace.setShowD1Export,
    ),
    backups: useWorkspaceSelector((workspace) => workspace.backups),
    backupsLoading: useWorkspaceSelector((workspace) => workspace.backupsLoading),
    backupsError: useWorkspaceSelector((workspace) => workspace.backupsError),
    loadMigrations: useWorkspaceSelector((workspace) => workspace.loadMigrations),
    loadBackups: useWorkspaceSelector((workspace) => workspace.loadBackups),
    openMigrations: useWorkspaceSelector((workspace) => workspace.openMigrations),
    handleApplyMigrations: useWorkspaceSelector(
      (workspace) => workspace.handleApplyMigrations,
    ),
    openBackups: useWorkspaceSelector((workspace) => workspace.openBackups),
    handleCreateBackup: useWorkspaceSelector(
      (workspace) => workspace.handleCreateBackup,
    ),
    requestImport: useWorkspaceSelector((workspace) => workspace.requestImport),
    handleImported: useWorkspaceSelector((workspace) => workspace.handleImported),
    requestD1Export: useWorkspaceSelector(
      (workspace) => workspace.requestD1Export,
    ),
    handleD1Exported: useWorkspaceSelector(
      (workspace) => workspace.handleD1Exported,
    ),
    handleRestoreBackup: useWorkspaceSelector(
      (workspace) => workspace.handleRestoreBackup,
    ),
    creatingBackup: useWorkspaceSelector((workspace) => workspace.creatingBackup),
    restoringBackup: useWorkspaceSelector(
      (workspace) => workspace.restoringBackup,
    ),
    exportFormat: useWorkspaceSelector((workspace) => workspace.exportFormat),
    setExportFormat: useWorkspaceSelector(
      (workspace) => workspace.setExportFormat,
    ),
    exporting: useWorkspaceSelector((workspace) => workspace.exporting),
  };
}

export function useWorkspaceActionSlice() {
  return {
    exportCurrent: useWorkspaceSelector((workspace) => workspace.exportCurrent),
    refresh: useWorkspaceSelector((workspace) => workspace.refresh),
    openForeignKey: useWorkspaceSelector((workspace) => workspace.openForeignKey),
    handleTableCreated: useWorkspaceSelector(
      (workspace) => workspace.handleTableCreated,
    ),
    refreshAfterSchemaChange: useWorkspaceSelector(
      (workspace) => workspace.refreshAfterSchemaChange,
    ),
  };
}

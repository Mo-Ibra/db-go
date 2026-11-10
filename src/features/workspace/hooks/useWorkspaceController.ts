import { useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";
import type {
  ExportFormat,
  SchemaGraph,
  SchemaMap,
  TableInfo,
} from "@/types";
import type { useDatabaseSession } from "@/features/connection/hooks/useDatabaseSession";
import { usePendingEdits } from "@/features/tables/hooks/usePendingEdits";
import { useTableData } from "@/features/tables/hooks/useTableData";
import { useProjectDialogs } from "@/features/project/hooks/useProjectDialogs";
import { useQueryEditor } from "@/features/query/hooks/useQueryEditor";
import { useTableDetails } from "@/features/schema/hooks/useTableDetails";
import { useExport } from "@/features/workspace/hooks/useExport";
import { useSchemaActions } from "@/features/schema/hooks/useSchemaActions";
import { useExternalDataSync } from "@/features/workspace/hooks/useExternalDataSync";
import { useOpenDatabaseShortcut } from "@/features/workspace/hooks/useOpenDatabaseShortcut";

type DatabaseSession = ReturnType<typeof useDatabaseSession>;

export function useWorkspaceController(session: DatabaseSession) {
  const {
    db,
    loading: sessionLoading,
    error,
    setError,
    notice,
    setNotice,
    showDesignerOnOpen,
    recentDatabases,
    missingRecents,
    clearRecentDatabases,
    openRecentDatabase,
    removeRecentDatabase,
    openDb,
    createNewDatabase,
  } = session;
  const [tables, setTables] = useState<TableInfo[]>(db?.tables ?? []);
  const [filterText, setFilterText] = useState("");
  const [showDesigner, setShowDesigner] = useState(showDesignerOnOpen);
  const [view, setView] = useState<"data" | "structure" | "sql" | "diagram">("data");
  const [schema, setSchema] = useState<SchemaMap>({});
  const [schemaGraph, setSchemaGraph] = useState<SchemaGraph | null>(null);
  const [showAlterTable, setShowAlterTable] = useState(false);
  const [creatingBackup, setCreatingBackup] = useState(false);
  const [restoringBackup, setRestoringBackup] = useState<string | null>(null);
  const [exportFormat, setExportFormat] = useState<ExportFormat>("csv");
  const [exporting, setExporting] = useState(false);
  const initialized = useRef(false);
  const lastDataVersion = useRef<number | null>(null);
  const externalChangePending = useRef(false);
  const saveReloadRef = useRef<() => Promise<void>>(async () => {});
  const pendingGuardRef = useRef<() => boolean>(() => false);
  const pendingResetRef = useRef<() => void>(() => {});


  const {
    tableDetails,
    setTableDetails,
    tableStats,
    setTableStats,
    detailsLoading,
    loadDetails,
    resetTableDetails,
  } = useTableDetails({ setError });

  const {
    selected,
    setSelected,
    columns,
    setColumns,
    foreignKeys,
    setForeignKeys,
    page,
    setPage,
    pageSize,
    history,
    setHistory,
    historyIndex,
    setHistoryIndex,
    pageIndex,
    setPageIndex,
    count,
    setCount,
    loading,
    sort,
    setSort,
    dataFilter,
    setDataFilter,
    filterDraft,
    setFilterDraft,
    pagedMode,
    currentPage,
    rowStart,
    rowEnd,
    nextDisabled,
    prevDisabled,
    loadTable,
    loadOffsetPage,
    selectTable,
    toggleSort,
    applyFilter,
    clearFilter,
    nextPage,
    prevPage,
    changePageSize,
    reloadCurrent,
    reloadTable,
    clearTable,
  } = useTableData({
    guardPending: () => pendingGuardRef.current(),
    resetTableUi: () => pendingResetRef.current(),
    resetTableDetails,
    setView,
    setError,
  });

  const busy = loading || sessionLoading;

  const {
    selectedRow,
    setSelectedRow,
    selectionKeys,
    pendingInserts,
    pendingUpdates,
    pendingDeletes,
    rowEditor,
    setRowEditor,
    saving,
    hasPending,
    pendingRowIds,
    displayRows,
    allSelected,
    someSelected,
    guardPending,
    resetTableUi,
    discardChanges,
    clearSelection,
    toggleRowSelection,
    toggleAllSelection,
    copySelectedRows,
    deleteSelectedRows,
    handleCellCommit,
    canEditCell,
    pasteIntoCell,
    saveChanges,
    submitRowEditor,
    editSelectedRow,
    deleteSelectedRow,
  } = usePendingEdits({
    page,
    columns,
    selected,
    view,
    restoringBackup,
    setNotice,
    setError,
    onAfterSave: () => saveReloadRef.current(),
  });
  pendingGuardRef.current = guardPending;
  pendingResetRef.current = resetTableUi;

  const {
    sqlText,
    setSqlText,
    queryResult,
    queryRunning,
    queryPlan,
    setQueryPlan,
    explainRunning,
    queryLimit,
    setQueryLimit,
    queryHistory,
    runQuery,
    explainQuery,
    changeSqlText,
  } = useQueryEditor({ setView, setSelectedRow, setError });

  useEffect(() => {
    saveReloadRef.current = async () => {
      try {
        lastDataVersion.current = await api.dataVersion();
      } catch {
        // The explicit reload below still refreshes the view.
      }
      externalChangePending.current = false;
      if (!selected) return;
      if (pagedMode) {
        await loadOffsetPage(selected, pageIndex, sort, dataFilter);
      } else {
        await loadTable(selected, history[historyIndex]);
      }
      void api.countRows(selected).then(setCount).catch(() => {});
    };
  }, [
    selected,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    history,
    historyIndex,
    loadTable,
    loadOffsetPage,
  ]);

  useEffect(() => {
    if (!db || initialized.current) return;
    initialized.current = true;
    void Promise.all([api.schemaMap(), api.schemaGraph()])
      .then(([nextSchema, nextGraph]) => {
        setSchema(nextSchema);
        setSchemaGraph(nextGraph);
      })
      .catch(() => {
        setSchema({});
        setSchemaGraph(null);
      });
    const firstTable = db.tables[0]?.name;
    if (firstTable) void loadTable(firstTable, null);
  }, [db, loadTable]);

  const {
    handleTableCreated,
    requestDesigner,
    refreshAfterSchemaChange,
    refresh,
    switchView,
    requestAlterTable,
    handleAlterApplied,
    handleCreateIndex,
    handleDropIndex,
    openForeignKey,
    openReferencedTable,
  } = useSchemaActions({
    setShowDesigner,
    setShowAlterTable,
    setTables,
    setSchema,
    setSchemaGraph,
    setView,
    setSort,
    setDataFilter,
    setFilterDraft,
    setPageIndex,
    resetTableUi,
    setHistory,
    setHistoryIndex,
    setSelected,
    setPage,
    setColumns,
    setForeignKeys,
    setCount,
    setTableDetails,
    setTableStats,
    setSelectedRow,
    setSqlText,
    setError,
    setNotice,
    guardPending,
    loadTable,
    loadOffsetPage,
    loadDetails,
    reloadCurrent,
    runQuery,
    lastDataVersion,
    externalChangePending,
    selected,
    view,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    history,
    historyIndex,
    sqlText,
    schemaGraph,
  });

  const {
    showMigrations,
    setShowMigrations,
    migrations,
    migrationsLoading,
    migrationsApplying,
    migrationsError,
    showBackups,
    setShowBackups,
    showImport,
    setShowImport,
    showD1Export,
    setShowD1Export,
    backups,
    backupsLoading,
    backupsError,
    loadMigrations,
    loadBackups,
    openMigrations,
    handleApplyMigrations,
    openBackups,
    handleCreateBackup,
    requestImport,
    handleImported,
    requestD1Export,
    handleD1Exported,
    handleRestoreBackup,
  } = useProjectDialogs({
    guardPending,
    refreshAfterSchemaChange,
    openReferencedTable,
    selected,
    setNotice,
    creatingBackup,
    setCreatingBackup,
    restoringBackup,
    setRestoringBackup,
  });

  useOpenDatabaseShortcut(openDb);

  useExternalDataSync({
    dbPath: db?.path ?? null,
    view,
    selected,
    pagedMode,
    pageIndex,
    sort,
    dataFilter,
    history,
    historyIndex,
    loading,
    queryRunning,
    saving,
    restoringBackup,
    hasPending,
    lastDataVersion,
    externalChangePending,
    setNotice,
    setTables,
    setSchema,
    setSchemaGraph,
    setSelected,
    setPage,
    setColumns,
    setCount,
    setTableDetails,
    setTableStats,
    loadDetails,
    loadTable,
    loadOffsetPage,
  });

  const exportCurrent = useExport({
    exporting,
    setExporting,
    guardPending,
    view,
    selected,
    sqlText,
    exportFormat,
    dataFilter,
    sort,
    setError,
    setNotice,
  });

  const visibleTables = tables.filter((table) =>
    table.name.toLowerCase().includes(filterText.trim().toLowerCase()),
  );
  const detailColumns = view === "data" ? columns : (queryResult?.columns ?? []);


  return {
    db,
    sessionLoading,
    error,
    setError,
    notice,
    setNotice,
    recentDatabases,
    missingRecents,
    clearRecentDatabases,
    openRecentDatabase,
    removeRecentDatabase,
    openDb,
    createNewDatabase,
    tables,
    setTables,
    filterText,
    setFilterText,
    showDesigner,
    setShowDesigner,
    view,
    setView,
    schema,
    setSchema,
    schemaGraph,
    setSchemaGraph,
    showAlterTable,
    setShowAlterTable,
    creatingBackup,
    restoringBackup,
    exportFormat,
    setExportFormat,
    exporting,
    selected,
    setSelected,
    columns,
    setColumns,
    foreignKeys,
    setForeignKeys,
    page,
    setPage,
    pageSize,
    history,
    setHistory,
    historyIndex,
    setHistoryIndex,
    pageIndex,
    setPageIndex,
    count,
    setCount,
    loading,
    sort,
    setSort,
    dataFilter,
    setDataFilter,
    filterDraft,
    setFilterDraft,
    pagedMode,
    currentPage,
    rowStart,
    rowEnd,
    nextDisabled,
    prevDisabled,
    loadTable,
    loadOffsetPage,
    selectTable,
    toggleSort,
    applyFilter,
    clearFilter,
    nextPage,
    prevPage,
    changePageSize,
    reloadCurrent,
    reloadTable,
    clearTable,
    selectedRow,
    setSelectedRow,
    selectionKeys,
    pendingInserts,
    pendingUpdates,
    pendingDeletes,
    rowEditor,
    setRowEditor,
    saving,
    hasPending,
    pendingRowIds,
    displayRows,
    allSelected,
    someSelected,
    guardPending,
    resetTableUi,
    discardChanges,
    clearSelection,
    toggleRowSelection,
    toggleAllSelection,
    copySelectedRows,
    deleteSelectedRows,
    handleCellCommit,
    canEditCell,
    pasteIntoCell,
    saveChanges,
    submitRowEditor,
    editSelectedRow,
    deleteSelectedRow,
    sqlText,
    setSqlText,
    queryResult,
    queryRunning,
    queryPlan,
    setQueryPlan,
    explainRunning,
    queryLimit,
    setQueryLimit,
    queryHistory,
    runQuery,
    explainQuery,
    changeSqlText,
    tableDetails,
    setTableDetails,
    tableStats,
    setTableStats,
    detailsLoading,
    loadDetails,
    resetTableDetails,
    showMigrations,
    setShowMigrations,
    migrations,
    migrationsLoading,
    migrationsApplying,
    migrationsError,
    showBackups,
    setShowBackups,
    showImport,
    setShowImport,
    showD1Export,
    setShowD1Export,
    backups,
    backupsLoading,
    backupsError,
    loadMigrations,
    loadBackups,
    openMigrations,
    handleApplyMigrations,
    openBackups,
    handleCreateBackup,
    requestImport,
    handleImported,
    requestD1Export,
    handleD1Exported,
    handleRestoreBackup,
    visibleTables,
    busy,
    detailColumns,
    refreshAfterSchemaChange,
    exportCurrent,
    refresh,
    switchView,
    requestDesigner,
    requestAlterTable,
    handleAlterApplied,
    handleCreateIndex,
    handleDropIndex,
    openForeignKey,
    openReferencedTable,
    handleTableCreated,
  };
}

import { RowEditor } from "@/features/tables/components/RowEditor";
import { AlterTableDialog } from "@/features/schema/components/AlterTableDialog";
import { MigrationsDialog } from "@/features/project/components/MigrationsDialog";
import { BackupsDialog } from "@/features/project/components/BackupsDialog";
import { ImportDialog } from "@/features/project/components/ImportDialog";
import { D1ExportDialog } from "@/features/project/components/D1ExportDialog";
import { TableDesigner } from "@/features/schema/components/TableDesigner";
import { basename } from "@/lib/path";
import {
  useWorkspaceActionSlice,
  useWorkspaceDialogSlice,
  useWorkspacePendingSlice,
  useWorkspaceSchemaSlice,
  useWorkspaceSelector,
  useWorkspaceShellSlice,
  useWorkspaceTableSlice,
} from "@/app/providers/WorkspaceContext";

export function AppDialogs() {
  const db = useWorkspaceSelector((workspace) => workspace.db);
  const tables = useWorkspaceSelector((workspace) => workspace.tables);
  const schema = useWorkspaceSelector((workspace) => workspace.schema);
  const { selected, columns } = useWorkspaceTableSlice();
  const { rowEditor, setRowEditor, submitRowEditor } =
    useWorkspacePendingSlice();
  const { showDesigner, setShowDesigner, showAlterTable, setShowAlterTable } =
    useWorkspaceShellSlice();
  const { tableDetails, handleAlterApplied } = useWorkspaceSchemaSlice();
  const { handleTableCreated } = useWorkspaceActionSlice();
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
    handleApplyMigrations,
    handleCreateBackup,
    handleImported,
    handleD1Exported,
    handleRestoreBackup,
    creatingBackup,
    restoringBackup,
  } = useWorkspaceDialogSlice();

  return (
    <>
      <RowEditor
        open={rowEditor !== null}
        mode={rowEditor?.mode ?? "insert"}
        columns={columns}
        initial={rowEditor?.row ?? null}
        onClose={() => setRowEditor(null)}
        onSubmit={submitRowEditor}
      />

      <AlterTableDialog
        open={showAlterTable}
        table={selected ?? ""}
        columns={tableDetails?.name === selected ? tableDetails.columns : columns}
        schema={schema}
        onClose={() => setShowAlterTable(false)}
        onApplied={handleAlterApplied}
      />

      <MigrationsDialog
        open={showMigrations}
        migrations={migrations}
        loading={migrationsLoading}
        applying={migrationsApplying}
        error={migrationsError}
        onClose={() => setShowMigrations(false)}
        onRefresh={() => void loadMigrations()}
        onApply={() => void handleApplyMigrations()}
      />

      <BackupsDialog
        open={showBackups}
        backups={backups}
        loading={backupsLoading}
        error={backupsError}
        restoring={restoringBackup}
        creating={creatingBackup}
        onClose={() => setShowBackups(false)}
        onRefresh={() => void loadBackups()}
        onCreateBackup={() => void handleCreateBackup()}
        onRestore={(backup) => void handleRestoreBackup(backup)}
      />

      <ImportDialog
        open={showImport}
        tables={tables}
        schema={schema}
        selectedTable={selected}
        onClose={() => setShowImport(false)}
        onImported={handleImported}
      />

      <D1ExportDialog
        open={showD1Export}
        defaultName={
          db
            ? db.project_path
              ? basename(db.project_path)
              : basename(db.path).replace(/\.[^.]+$/, "")
            : "my-db"
        }
        onClose={() => setShowD1Export(false)}
        onExported={handleD1Exported}
      />

      <TableDesigner
        open={showDesigner}
        tables={tables}
        schema={schema}
        onClose={() => setShowDesigner(false)}
        onCreated={(table) => void handleTableCreated(table)}
      />
    </>
  );
}

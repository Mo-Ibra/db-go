import { useCallback, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { api } from "@/lib/api";
import type {
  BackupInfo,
  D1ExportResult,
  ImportResult,
  MigrationInfo,
} from "@/types";

type ProjectDialogsOptions = {
  guardPending: () => boolean;
  refreshAfterSchemaChange: (
    table: string | null,
    message: string,
  ) => Promise<void>;
  openReferencedTable: (table: string) => void;
  selected: string | null;
  setNotice: (notice: string | null) => void;
  creatingBackup: boolean;
  setCreatingBackup: (creating: boolean) => void;
  restoringBackup: string | null;
  setRestoringBackup: (path: string | null) => void;
};

export function useProjectDialogs({
  guardPending,
  refreshAfterSchemaChange,
  openReferencedTable,
  selected,
  setNotice,
  creatingBackup,
  setCreatingBackup,
  restoringBackup,
  setRestoringBackup,
}: ProjectDialogsOptions) {
  const [showMigrations, setShowMigrations] = useState(false);
  const [migrations, setMigrations] = useState<MigrationInfo[]>([]);
  const [migrationsLoading, setMigrationsLoading] = useState(false);
  const [migrationsApplying, setMigrationsApplying] = useState(false);
  const [migrationsError, setMigrationsError] = useState<string | null>(null);
  const [showBackups, setShowBackups] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [showD1Export, setShowD1Export] = useState(false);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backupsLoading, setBackupsLoading] = useState(false);
  const [backupsError, setBackupsError] = useState<string | null>(null);

  const loadMigrations = useCallback(async () => {
    setMigrationsLoading(true);
    setMigrationsError(null);
    try {
      setMigrations(await api.listMigrations());
    } catch (cause) {
      setMigrationsError(String(cause));
    } finally {
      setMigrationsLoading(false);
    }
  }, []);

  const loadBackups = useCallback(async () => {
    setBackupsLoading(true);
    setBackupsError(null);
    try {
      setBackups(await api.listBackups());
    } catch (cause) {
      setBackupsError(String(cause));
    } finally {
      setBackupsLoading(false);
    }
  }, []);

  const openMigrations = useCallback(() => {
    if (guardPending()) return;
    setShowMigrations(true);
    void loadMigrations();
  }, [guardPending, loadMigrations]);

  const handleApplyMigrations = useCallback(async () => {
    if (guardPending()) return;
    setMigrationsApplying(true);
    setMigrationsError(null);
    try {
      const result = await api.applyMigrations();
      await loadMigrations();
      if (result.applied.length === 0) {
        setNotice("No pending migrations.");
        return;
      }
      const names = result.applied.map((migration) => migration.name).join(", ");
      await refreshAfterSchemaChange(
        selected,
        `Applied ${result.applied.length} migration(s): ${names}`,
      );
    } catch (cause) {
      setMigrationsError(String(cause));
      try {
        setMigrations(await api.listMigrations());
      } catch {
        // Keep the original error visible.
      }
    } finally {
      setMigrationsApplying(false);
    }
  }, [guardPending, loadMigrations, refreshAfterSchemaChange, selected, setNotice]);

  const openBackups = useCallback(() => {
    if (guardPending()) return;
    setShowBackups(true);
    void loadBackups();
  }, [guardPending, loadBackups]);

  const handleCreateBackup = useCallback(async () => {
    if (creatingBackup || restoringBackup !== null) return;
    setCreatingBackup(true);
    setBackupsError(null);
    try {
      const path = await api.createBackup();
      await loadBackups();
      setNotice(`Backup created at ${path}`);
    } catch (cause) {
      setBackupsError(String(cause));
    } finally {
      setCreatingBackup(false);
    }
  }, [creatingBackup, restoringBackup, loadBackups, setNotice]);

  const requestImport = useCallback(() => {
    if (guardPending()) return;
    setShowImport(true);
  }, [guardPending]);

  const handleImported = useCallback(
    async (result: ImportResult) => {
      setShowImport(false);
      const ignored =
        result.ignored_columns.length > 0
          ? ` · ignored ${result.ignored_columns.join(", ")}`
          : "";
      const created = result.created_table ? " · created table" : "";
      await refreshAfterSchemaChange(
        result.table,
        `Imported ${result.rows.toLocaleString()} row(s) into ${result.table} · session ${result.session_id}${created}${ignored}`,
      );
      openReferencedTable(result.table);
    },
    [refreshAfterSchemaChange, openReferencedTable],
  );

  const requestD1Export = useCallback(() => {
    if (guardPending()) return;
    setShowD1Export(true);
  }, [guardPending]);

  const handleD1Exported = useCallback(
    (result: D1ExportResult) => {
      setShowD1Export(false);
      const warnings =
        result.warnings.length > 0
          ? ` · ${result.warnings.length} warning${result.warnings.length === 1 ? "" : "s"}`
          : "";
      setNotice(
        `D1 export written to ${result.output_dir} · ${result.rows.toLocaleString()} row(s) · ${result.data_files.length} data file${result.data_files.length === 1 ? "" : "s"}${warnings}`,
      );
    },
    [setNotice],
  );

  const handleRestoreBackup = useCallback(
    async (backup: BackupInfo) => {
      if (guardPending()) return;
      const confirmed = await confirm(
        `Restore ${backup.name}? This replaces the current database and cannot be undone unless you take a backup first. Changes made after this snapshot will be lost.`,
        { title: "Restore backup?", kind: "warning" },
      );
      if (!confirmed) return;

      setRestoringBackup(backup.path);
      setBackupsError(null);
      try {
        const result = await api.restoreBackup(backup.path);
        await loadBackups();
        await refreshAfterSchemaChange(
          selected,
          `Restored ${backup.name} · ${result.tables} table(s)`,
        );
      } catch (cause) {
        setBackupsError(String(cause));
        try {
          setBackups(await api.listBackups());
        } catch {
          // Keep the original error visible.
        }
      } finally {
        setRestoringBackup(null);
      }
    },
    [guardPending, loadBackups, refreshAfterSchemaChange, selected],
  );

  const resetProjectDialogs = useCallback(() => {
    setShowMigrations(false);
    setMigrations([]);
    setMigrationsError(null);
    setMigrationsApplying(false);
    setMigrationsLoading(false);
    setShowBackups(false);
    setShowImport(false);
    setShowD1Export(false);
    setBackups([]);
    setBackupsError(null);
    setBackupsLoading(false);
  }, []);

  return {
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
    creatingBackup,
    restoringBackup,
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
    resetProjectDialogs,
  };
}

import { useCallback, useEffect, useRef, useState } from "react";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { api } from "@/lib/api";
import { useRecentDatabases } from "@/features/connection/hooks/useRecentDatabases";
import type { RecentDatabase } from "@/features/connection/hooks/useRecentDatabases";
import type { DbInfo } from "@/types";

export function useDatabaseSession() {
  const [db, setDb] = useState<DbInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [showDesignerOnOpen, setShowDesignerOnOpen] = useState(false);
  const bootstrapped = useRef(false);
  const pendingGuard = useRef<() => boolean>(() => false);
  const {
    recentDatabases,
    missingRecents,
    setMissingRecents,
    rememberDatabase,
    removeRecentDatabase,
    clearRecentDatabases,
  } = useRecentDatabases();

  const registerPendingGuard = useCallback((guard: () => boolean) => {
    pendingGuard.current = guard;
  }, []);

  const openPath = useCallback(
    async (path: string): Promise<boolean> => {
      setLoading(true);
      setError(null);
      setShowDesignerOnOpen(false);
      try {
        const info = await api.openDatabase(path);
        setDb(info);
        rememberDatabase(info);
        if (info.agent_files_error) {
          setNotice(`Agent files: ${info.agent_files_error}`);
        } else if (info.agent_files?.created) {
          setNotice(`Agent files created in ${info.agent_files.directory}`);
        }
        return true;
      } catch (cause) {
        setError(String(cause));
        return false;
      } finally {
        setLoading(false);
      }
    },
    [rememberDatabase],
  );

  const openRecentDatabase = useCallback(
    async (entry: RecentDatabase) => {
      if (missingRecents.has(entry.path)) return;
      const opened = await openPath(entry.path);
      if (!opened) {
        setMissingRecents((current) => {
          const next = new Set(current);
          next.add(entry.path);
          return next;
        });
      }
    },
    [missingRecents, openPath, setMissingRecents],
  );

  const createNewDatabase = useCallback(async () => {
    if (pendingGuard.current()) return;
    try {
      const path = await open({
        directory: true,
        multiple: false,
        title: "Choose a folder for the database project",
      });
      if (!path || Array.isArray(path)) return;

      setLoading(true);
      setError(null);
      try {
        let info: DbInfo;
        try {
          info = await api.createProject(path, false);
        } catch (cause) {
          const message = String(cause);
          if (!message.toLowerCase().includes("already exists")) throw cause;
          const replace = await confirm(
            "A database project already exists in that folder. Replace it? A timestamped backup will be kept.",
            { title: "Replace project?", kind: "warning" },
          );
          if (!replace) return;
          info = await api.createProject(path, true);
        }

        setShowDesignerOnOpen(true);
        setDb(info);
        rememberDatabase(info);
        if (info.backup_path) {
          setNotice(`Existing database backed up to ${info.backup_path}`);
        }
      } finally {
        setLoading(false);
      }
    } catch (cause) {
      setError(String(cause));
    }
  }, [rememberDatabase]);

  const openDb = useCallback(async () => {
    if (pendingGuard.current()) return;
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: "SQLite database", extensions: ["db", "sqlite", "sqlite3", "d1"] },
        ],
      });
      if (!path || Array.isArray(path)) return;
      await openPath(path);
    } catch (cause) {
      setError(String(cause));
    }
  }, [openPath]);

  useEffect(() => {
    if (bootstrapped.current) return;
    bootstrapped.current = true;
    void api
      .initialDatabase()
      .then((path) => {
        if (path) void openPath(path);
      })
      .catch(() => {});
  }, [openPath]);

  return {
    db,
    loading,
    error,
    setError,
    notice,
    setNotice,
    showDesignerOnOpen,
    recentDatabases,
    missingRecents,
    openPath,
    openRecentDatabase,
    removeRecentDatabase,
    clearRecentDatabases,
    openDb,
    createNewDatabase,
    registerPendingGuard,
  };
}

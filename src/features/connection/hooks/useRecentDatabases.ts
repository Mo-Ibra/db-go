import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { basename } from "@/lib/path";
import type { DbInfo } from "@/types";

const RECENT_DATABASES_KEY = "dbgo:recent-databases";
const RECENT_DATABASE_LIMIT = 10;

export type RecentDatabase = {
  path: string;
  name: string;
  openedAt: number;
};

function loadRecentDatabases(): RecentDatabase[] {
  try {
    const raw = localStorage.getItem(RECENT_DATABASES_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (item): item is RecentDatabase =>
          item !== null &&
          typeof item === "object" &&
          typeof (item as RecentDatabase).path === "string" &&
          typeof (item as RecentDatabase).name === "string" &&
          typeof (item as RecentDatabase).openedAt === "number",
      )
      .slice(0, RECENT_DATABASE_LIMIT);
  } catch {
    return [];
  }
}

function persistRecentDatabases(entries: RecentDatabase[]) {
  try {
    localStorage.setItem(RECENT_DATABASES_KEY, JSON.stringify(entries));
  } catch {
    // Recent databases are a convenience; ignore storage failures.
  }
}

export function useRecentDatabases() {
  const [recentDatabases, setRecentDatabases] = useState<RecentDatabase[]>(
    loadRecentDatabases,
  );
  const [missingRecents, setMissingRecents] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (recentDatabases.length === 0) {
      setMissingRecents(new Set());
      return;
    }
    let cancelled = false;
    void Promise.all(
      recentDatabases.map(async (entry) => ({
        path: entry.path,
        exists: await api.pathExists(entry.path).catch(() => true),
      })),
    ).then((results) => {
      if (cancelled) return;
      setMissingRecents(
        new Set(
          results.filter((item) => !item.exists).map((item) => item.path),
        ),
      );
    });
    return () => {
      cancelled = true;
    };
  }, [recentDatabases]);

  const rememberDatabase = useCallback((info: DbInfo) => {
    const fileName = basename(info.path);
    const name = info.project_path ? basename(info.project_path) : fileName;
    const entry: RecentDatabase = {
      path: info.path,
      name: name || fileName,
      openedAt: Date.now(),
    };
    setRecentDatabases((current) => {
      const next = [
        entry,
        ...current.filter((item) => item.path !== entry.path),
      ].slice(0, RECENT_DATABASE_LIMIT);
      persistRecentDatabases(next);
      return next;
    });
  }, []);

  const removeRecentDatabase = useCallback((path: string) => {
    setRecentDatabases((current) => {
      const next = current.filter((item) => item.path !== path);
      persistRecentDatabases(next);
      return next;
    });
    setMissingRecents((current) => {
      const next = new Set(current);
      next.delete(path);
      return next;
    });
  }, []);

  const clearRecentDatabases = useCallback(() => {
    setRecentDatabases([]);
    setMissingRecents(new Set());
    persistRecentDatabases([]);
  }, []);

  return {
    recentDatabases,
    missingRecents,
    setMissingRecents,
    rememberDatabase,
    removeRecentDatabase,
    clearRecentDatabases,
  };
}

import { useCallback } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "@/lib/api";
import type { ExportFormat, FilterSpec, SortSpec } from "@/types";

type View = "data" | "structure" | "sql" | "diagram";

type ExportOptions = {
  exporting: boolean;
  setExporting: (exporting: boolean) => void;
  guardPending: () => boolean;
  view: View;
  selected: string | null;
  sqlText: string;
  exportFormat: ExportFormat;
  dataFilter: FilterSpec | null;
  sort: SortSpec | null;
  setError: (error: string | null) => void;
  setNotice: (notice: string | null) => void;
};

export function useExport({
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
}: ExportOptions) {
  return useCallback(async () => {
    if (exporting) return;
    if (guardPending()) return;

    const isData = view === "data";
    const isSql = view === "sql";
    if (isData && !selected) return;
    if (isSql && !sqlText.trim()) return;
    if (!isData && !isSql) return;

    const base =
      ((isData ? selected : "query") ?? "export")
        .replace(/[^A-Za-z0-9_-]+/g, "_")
        .slice(0, 40) || "export";
    try {
      const path = await save({
        defaultPath: `${base}-${new Date().toISOString().slice(0, 10)}.${exportFormat}`,
        filters: [{ name: exportFormat.toUpperCase(), extensions: [exportFormat] }],
      });
      if (!path || Array.isArray(path)) return;

      setExporting(true);
      setError(null);
      const result = await api.exportData({
        table: isData ? selected : null,
        query: isSql ? sqlText : null,
        filter: isData ? dataFilter : null,
        sort: isData ? sort : null,
        format: exportFormat,
        path,
      });
      setNotice(
        `Exported ${result.rows.toLocaleString()} row(s) to ${result.path}`,
      );
    } catch (cause) {
      setError(String(cause));
    } finally {
      setExporting(false);
    }
  }, [
    exporting,
    guardPending,
    view,
    selected,
    sqlText,
    exportFormat,
    dataFilter,
    sort,
    setError,
    setNotice,
    setExporting,
  ]);
}

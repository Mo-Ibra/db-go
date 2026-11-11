import { useCallback } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { api } from "@/lib/api";
import type {
  FilterSpec,
  Page,
  SchemaChange,
  SchemaGraph,
  SchemaMap,
  SortSpec,
  TableInfo,
} from "@/types";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

type SchemaMutationsOptions = {
  selected: string | null;
  guardPending: () => boolean;
  loadTable: (table: string, cursor: number | null) => Promise<Page | null>;
  resetTableUi: () => void;
  refreshAfterSchemaChange: (
    table: string | null,
    message: string,
  ) => Promise<void>;
  setShowDesigner: (show: boolean) => void;
  setShowAlterTable: (show: boolean) => void;
  setTables: (tables: TableInfo[]) => void;
  setSchema: (schema: SchemaMap) => void;
  setSchemaGraph: (graph: SchemaGraph | null) => void;
  setView: (view: View) => void;
  setSort: (sort: SortSpec | null) => void;
  setDataFilter: (filter: FilterSpec | null) => void;
  setPageIndex: (index: number) => void;
  setHistory: (history: (number | null)[]) => void;
  setHistoryIndex: (index: number) => void;
  setError: (error: string | null) => void;
};

/** Table creation, design, and index mutations. */
export function useSchemaMutations({
  selected,
  guardPending,
  loadTable,
  resetTableUi,
  refreshAfterSchemaChange,
  setShowDesigner,
  setShowAlterTable,
  setTables,
  setSchema,
  setSchemaGraph,
  setView,
  setSort,
  setDataFilter,
  setPageIndex,
  setHistory,
  setHistoryIndex,
  setError,
}: SchemaMutationsOptions) {
  const handleTableCreated = useCallback(
    async (table: string) => {
      setShowDesigner(false);
      try {
        const [nextTables, nextSchema, nextGraph] = await Promise.all([
          api.listTables(),
          api.schemaMap(),
          api.schemaGraph(),
        ]);
        setTables(nextTables);
        setSchema(nextSchema);
        setSchemaGraph(nextGraph);
        setView("data");
        setSort(null);
        setDataFilter(null);
        setPageIndex(0);
        resetTableUi();
        setHistory([null]);
        setHistoryIndex(0);
        void loadTable(table, null);
      } catch (cause) {
        setError(String(cause));
      }
    },
    [
      loadTable,
      resetTableUi,
      setDataFilter,
      setError,
      setHistory,
      setHistoryIndex,
      setPageIndex,
      setSchema,
      setSchemaGraph,
      setShowDesigner,
      setSort,
      setTables,
      setView,
    ],
  );

  const requestDesigner = useCallback(() => {
    if (guardPending()) return;
    setShowDesigner(true);
  }, [guardPending, setShowDesigner]);

  const requestAlterTable = useCallback(() => {
    if (!selected) return;
    if (guardPending()) return;
    setShowAlterTable(true);
  }, [selected, guardPending, setShowAlterTable]);

  const handleAlterApplied = useCallback(
    async (change: SchemaChange) => {
      const backup = change.backup_path ? ` · backup ${change.backup_path}` : "";
      await refreshAfterSchemaChange(
        selected,
        `Applied schema change · migration ${change.migration_name}${backup}`,
      );
    },
    [selected, refreshAfterSchemaChange],
  );

  const handleCreateIndex = useCallback(
    async (name: string, columns: string[], unique: boolean) => {
      if (!selected) throw new Error("No table selected.");
      if (guardPending()) throw new Error("Save or discard pending changes first.");
      const change = await api.createIndex(selected, name, columns, unique);
      await refreshAfterSchemaChange(
        selected,
        `Created index ${name} · migration ${change.migration_name}`,
      );
    },
    [selected, guardPending, refreshAfterSchemaChange],
  );

  const handleDropIndex = useCallback(
    async (name: string) => {
      if (!selected) throw new Error("No table selected.");
      if (guardPending()) throw new Error("Save or discard pending changes first.");
      const confirmed = await confirm(
        `Drop index ${name}? A timestamped backup is kept before the change.`,
        { title: "Drop index?", kind: "warning" },
      );
      if (!confirmed) return;
      const change = await api.dropIndex(name);
      await refreshAfterSchemaChange(
        selected,
        `Dropped index ${name} · migration ${change.migration_name}`,
      );
    },
    [selected, guardPending, refreshAfterSchemaChange],
  );

  return {
    handleTableCreated,
    requestDesigner,
    requestAlterTable,
    handleAlterApplied,
    handleCreateIndex,
    handleDropIndex,
  };
}

import { useRef, useState } from "react";
import type { ExportFormat, SchemaGraph, SchemaMap, TableInfo } from "@/types";

type View = "data" | "structure" | "sql" | "diagram" | "cards";

/** Local UI state owned by the workspace shell. */
export function useWorkspaceState(
  initialTables: TableInfo[],
  showDesignerOnOpen: boolean,
) {
  const [tables, setTables] = useState<TableInfo[]>(initialTables);
  const [showDesigner, setShowDesigner] = useState(showDesignerOnOpen);
  const [view, setView] = useState<View>("data");
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

  return {
    tables,
    setTables,
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
    setCreatingBackup,
    restoringBackup,
    setRestoringBackup,
    exportFormat,
    setExportFormat,
    exporting,
    setExporting,
    initialized,
    lastDataVersion,
    externalChangePending,
    saveReloadRef,
    pendingGuardRef,
    pendingResetRef,
  };
}

export type WorkspaceState = ReturnType<typeof useWorkspaceState>;

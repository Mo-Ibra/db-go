import {
  useWorkspaceQuerySlice,
  useWorkspaceSchemaSlice,
  useWorkspaceSelector,
  useWorkspaceTableSlice,
} from "@/app/providers/WorkspaceContext";

export function StatusFooter() {
  const db = useWorkspaceSelector((workspace) => workspace.db);
  const view = useWorkspaceSelector((workspace) => workspace.view);
  const schemaGraph = useWorkspaceSelector(
    (workspace) => workspace.schemaGraph,
  );
  const { tableDetails } = useWorkspaceSchemaSlice();
  const { selected, page, pagedMode, rowStart, rowEnd } =
    useWorkspaceTableSlice();
  const { queryResult } = useWorkspaceQuerySlice();

  return (
    <footer className="flex h-[26px] flex-none items-center justify-between gap-4 border-t border-line bg-panel px-3 font-mono text-[10.5px] text-muted-2">
      <span className="max-w-[60%] truncate" title={db?.path ?? undefined}>
        {db ? db.path : "No database open"}
      </span>
      <span className="whitespace-nowrap">
        {view === "diagram"
          ? `${schemaGraph?.tables.length ?? 0} tables · ${
              schemaGraph?.relationships.length ?? 0
            } relationships`
          : view === "structure"
            ? tableDetails?.name === selected
              ? `${tableDetails.columns.length} columns · ${tableDetails.indexes.length} indexes · ${tableDetails.foreign_keys.length} foreign keys`
              : "structure"
            : view === "sql"
              ? queryResult
                ? `query · ${queryResult.rows.length.toLocaleString()} rows${
                    queryResult.truncated ? " · truncated" : ""
                  }`
                : "read-only SQL"
              : selected && page
                ? page.rows.length === 0
                  ? "0 rows"
                  : `rows ${rowStart.toLocaleString()}–${rowEnd.toLocaleString()} · ${
                      pagedMode ? "filtered/sorted" : "keyset pagination"
                    }`
                : selected
                  ? pagedMode
                    ? "filtered/sorted"
                    : "keyset pagination"
                  : "SQLite"}
      </span>
    </footer>
  );
}

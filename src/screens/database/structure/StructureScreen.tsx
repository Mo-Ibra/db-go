import { StructureView } from "@/features/schema/components/StructureView";
import {
  useWorkspaceSchemaSlice,
  useWorkspaceTableSlice,
} from "@/app/providers/WorkspaceContext";
import { ViewToolbar } from "@/screens/database/data/ViewToolbar";

export function StructureScreen() {
  const { selected } = useWorkspaceTableSlice();
  const {
    tableDetails,
    tableStats,
    detailsLoading,
    loadDetails,
    requestAlterTable,
    handleCreateIndex,
    handleDropIndex,
    openReferencedTable,
  } = useWorkspaceSchemaSlice();

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <ViewToolbar />
      {selected ? (
        <StructureView
          table={selected}
          details={tableDetails?.name === selected ? tableDetails : null}
          stats={tableStats?.name === selected ? tableStats : null}
          loading={detailsLoading}
          onRefresh={() => void loadDetails(selected)}
          onAlter={requestAlterTable}
          onCreateIndex={handleCreateIndex}
          onDropIndex={handleDropIndex}
          onOpenReferencedTable={openReferencedTable}
        />
      ) : (
        <div className="grid flex-1 place-items-center text-xs text-muted">
          Select a table to inspect its structure.
        </div>
      )}
    </div>
  );
}

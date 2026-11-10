import { lazy, Suspense } from "react";
import { Network } from "lucide-react";
import { useWorkspace } from "@/app/providers/WorkspaceContext";
import { Badge } from "@/components/ui/Badge";

const DiagramView = lazy(() =>
  import("@/features/diagram/components/DiagramView").then((module) => ({
    default: module.DiagramView,
  })),
);

export function DiagramScreen() {
  const {
    db,
    schemaGraph,
    selected,
    guardPending,
    setView,
    loadTable,
    requestDesigner,
    requestAlterTable,
  } = useWorkspace();

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex h-12 flex-none items-center gap-2 border-b border-line px-3.5">
        <Network size={15} className="text-muted-2" />
        <span className="font-mono text-[13px] font-semibold">Schema diagram</span>
        <Badge tone="subtle">{schemaGraph?.tables.length ?? 0} tables</Badge>
        <Badge tone="subtle">{schemaGraph?.relationships.length ?? 0} relations</Badge>
      </div>
      {!schemaGraph ? (
        <div className="grid flex-1 place-items-center text-xs text-muted">
          Loading diagram...
        </div>
      ) : (
        <Suspense
          fallback={
            <div className="grid flex-1 place-items-center text-xs text-muted">
              Loading diagram...
            </div>
          }
        >
          <DiagramView
            graph={schemaGraph}
            dbPath={db?.path ?? ""}
            onOpenTable={(table) => {
              if (guardPending()) return;
              setView("data");
              void loadTable(table, null);
            }}
            onCreateTable={requestDesigner}
            onEditTable={requestAlterTable}
            canEditTable={Boolean(selected)}
          />
        </Suspense>
      )}
    </div>
  );
}

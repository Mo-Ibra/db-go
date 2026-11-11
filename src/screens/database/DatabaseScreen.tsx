import { Table2 } from "lucide-react";
import {
  useWorkspaceSchemaSlice,
  useWorkspaceSelector,
  useWorkspaceTableSlice,
} from "@/app/providers/WorkspaceContext";
import { StatusBanners } from "@/components/feedback/StatusBanners";
import { StatusFooter } from "@/components/layout/StatusFooter";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { CardsScreen } from "@/screens/database/cards/CardsScreen";
import { DataScreen } from "@/screens/database/data/DataScreen";
import { DiagramScreen } from "@/screens/database/diagram/DiagramScreen";
import { SqlScreen } from "@/screens/database/sql/SqlScreen";
import { StructureScreen } from "@/screens/database/structure/StructureScreen";

export function DatabaseScreen() {
  const tables = useWorkspaceSelector((workspace) => workspace.tables);
  const view = useWorkspaceSelector((workspace) => workspace.view);
  const { selected } = useWorkspaceTableSlice();
  const { requestDesigner } = useWorkspaceSchemaSlice();

  return (
    <main className="flex min-h-0 min-w-0 flex-1 flex-col">
      <StatusBanners />

      {view === "sql" ? (
        <SqlScreen />
      ) : view === "diagram" ? (
        <DiagramScreen />
      ) : view === "cards" ? (
        <CardsScreen />
      ) : !selected ? (
        tables.length === 0 ? (
          <EmptyState
            icon={Table2}
            title="No tables yet"
            description="Create your first table to start shaping the database."
            action={
              <Button variant="primary" size="lg" onClick={requestDesigner}>
                Create your first table
              </Button>
            }
          />
        ) : (
          <EmptyState
            icon={Table2}
            title="Select a table"
            description="Pick a table from the sidebar to browse its rows."
          />
        )
      ) : view === "data" ? (
        <DataScreen />
      ) : (
        <StructureScreen />
      )}

      <StatusFooter />
    </main>
  );
}

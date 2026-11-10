import type { useDatabaseSession } from "@/features/connection/hooks/useDatabaseSession";
import { AppHeader } from "@/components/layout/AppHeader";
import { TableSidebar } from "@/components/layout/TableSidebar";
import { AppDialogs } from "@/features/workspace/components/AppDialogs";
import { NavigationRail } from "@/app/navigation/NavigationRail";
import { WorkspaceProvider } from "@/app/providers/WorkspaceContext";
import { DatabaseScreen } from "@/screens/database/DatabaseScreen";
import { useWorkspaceController } from "@/features/workspace/hooks/useWorkspaceController";

type DatabaseSession = ReturnType<typeof useDatabaseSession>;

export function DatabaseWorkspace({ session }: { session: DatabaseSession }) {
  const workspace = useWorkspaceController(session);
  const showSidebar = workspace.view === "data" || workspace.view === "structure";

  return (
    <WorkspaceProvider value={workspace}>
      <div className="flex h-screen flex-col">
        <AppHeader />
        <div className="flex min-h-0 flex-1">
          <NavigationRail />
          {showSidebar ? <TableSidebar /> : null}
          <DatabaseScreen />
        </div>
        <AppDialogs />
      </div>
    </WorkspaceProvider>
  );
}

import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import type { useWorkspaceController } from "@/features/workspace/hooks/useWorkspaceController";

export type WorkspaceController = ReturnType<typeof useWorkspaceController>;

const WorkspaceContext = createContext<WorkspaceController | null>(null);

export function WorkspaceProvider({
  value,
  children,
}: {
  value: WorkspaceController;
  children: ReactNode;
}) {
  return (
    <WorkspaceContext.Provider value={value}>
      {children}
    </WorkspaceContext.Provider>
  );
}

export function useWorkspace(): WorkspaceController {
  const workspace = useContext(WorkspaceContext);
  if (!workspace) {
    throw new Error("useWorkspace must be used inside WorkspaceProvider");
  }
  return workspace;
}

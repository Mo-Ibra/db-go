import { Database, LayoutGrid, ListTree, Network, TerminalSquare } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/cn";
import { useWorkspaceNavigationSlice } from "@/app/providers/WorkspaceContext";

const screens: { id: "data" | "cards" | "structure" | "diagram" | "sql"; label: string; icon: LucideIcon }[] = [
  { id: "data", label: "Data", icon: Database },
  { id: "cards", label: "Cards", icon: LayoutGrid },
  { id: "structure", label: "Structure", icon: ListTree },
  { id: "diagram", label: "Diagram", icon: Network },
  { id: "sql", label: "SQL", icon: TerminalSquare },
];

export function NavigationRail() {
  const { view, switchView } = useWorkspaceNavigationSlice();

  return (
    <nav
      aria-label="Workspace"
      className="flex w-[52px] flex-none flex-col items-center gap-1 border-r border-line bg-panel/40 py-2"
    >
      {screens.map((screen) => {
        const Icon = screen.icon;
        const active = view === screen.id;
        return (
          <button
            key={screen.id}
            type="button"
            aria-label={screen.label}
            aria-current={active ? "page" : undefined}
            title={screen.label}
            onClick={() => switchView(screen.id)}
            className={cn(
              "grid h-9 w-9 place-items-center rounded-lg border transition-colors",
              active
                ? "border-accent/30 bg-accent/10 text-[#7ee787]"
                : "border-transparent text-muted-2 hover:border-line-strong hover:bg-panel-2 hover:text-fg",
            )}
          >
            <Icon size={17} strokeWidth={1.9} />
          </button>
        );
      })}
    </nav>
  );
}

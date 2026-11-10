import { Plus, Search, Table2 } from "lucide-react";
import { cn } from "@/lib/cn";
import { Badge } from "@/components/ui/Badge";
import { IconButton } from "@/components/ui/IconButton";
import { useWorkspace } from "@/app/providers/WorkspaceContext";

export function TableSidebar() {
  const {
    tables,
    filterText,
    setFilterText,
    db,
    visibleTables,
    selected,
    selectTable,
    requestDesigner,
    hasPending,
  } = useWorkspace();

  return (
    <aside className="flex w-[268px] min-w-[220px] max-w-[360px] flex-col border-r border-line bg-panel/60">
      <div className="flex h-11 flex-none items-center justify-between px-3.5">
        <span className="text-[10.5px] font-bold uppercase tracking-[0.14em] text-muted">
          Tables
        </span>
        <div className="flex items-center gap-2">
          <Badge tone="neutral">{tables.length}</Badge>
          <IconButton
            icon={Plus}
            label={hasPending ? "Save or discard pending changes first" : "New table"}
            size="sm"
            onClick={requestDesigner}
            disabled={hasPending}
          />
        </div>
      </div>

      <label className="relative mx-3 mb-2.5 block">
        <Search
          size={13}
          className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted-2"
        />
        <input
          className="h-[30px] w-full rounded-[7px] border border-line bg-ink pl-7 pr-2.5 text-[12.5px] text-fg outline-none placeholder:text-muted-2 focus:border-accent focus:ring-2 focus:ring-accent/20"
          placeholder="Filter tables..."
          value={filterText}
          onChange={(event) => setFilterText(event.target.value)}
          aria-label="Filter tables"
        />
      </label>

      <div className="min-h-0 flex-1 overflow-auto px-2 pb-2">
        {visibleTables.length === 0 ? (
          <div className="px-2.5 py-4 text-xs text-muted-2">No tables</div>
        ) : null}
        {visibleTables.map((table) => (
          <button
            key={table.name}
            className={cn(
              "group flex w-full items-center gap-2 rounded-md border border-transparent px-2 py-1.5 text-left text-[12.5px] text-fg hover:bg-panel-2",
              selected === table.name && "border-accent/25 bg-accent/10 text-[#7ee787]",
            )}
            onClick={() => selectTable(table.name)}
            title={table.name}
          >
            <Table2
              size={14}
              strokeWidth={1.8}
              className={selected === table.name ? "text-accent" : "text-muted-2"}
            />
            <span className="truncate font-mono text-xs">{table.name}</span>
          </button>
        ))}
      </div>

      <div className="flex-none border-t border-line px-3 py-2 text-[10.5px] text-muted-2">
        {db ? (
          <span className="flex items-center gap-1.5 truncate font-mono" title={db.path}>
            <span className="h-1.5 w-1.5 flex-none rounded-full bg-accent shadow-[0_0_8px_rgba(45,212,191,0.6)]" />
            {db.path.split("/").pop()}
          </span>
        ) : null}
      </div>
    </aside>
  );
}

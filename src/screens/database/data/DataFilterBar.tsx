import { Check, Funnel, X } from "lucide-react";
import type { FilterOp } from "@/types";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Select } from "@/components/ui/Select";
import { FILTER_OPS, isNullOp } from "@/components/ui/constants";
import { useWorkspace } from "@/app/providers/WorkspaceContext";

export function DataFilterBar() {
  const {
    columns,
    sort,
    dataFilter,
    filterDraft,
    setFilterDraft,
    applyFilter,
    clearFilter,
  } = useWorkspace();

  return (
    <div className="flex h-10 flex-none items-center gap-2 border-b border-line bg-panel/40 px-3">
      <Funnel size={13} className="text-muted-2" />
      <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted">
        Filter
      </span>
      <Select
        value={filterDraft.column}
        onChange={(event) =>
          setFilterDraft((current) => ({ ...current, column: event.target.value }))
        }
        disabled={columns.length === 0}
        aria-label="Filter column"
      >
        {columns.map((column) => (
          <option key={column.name} value={column.name}>
            {column.name}
          </option>
        ))}
      </Select>
      <Select
        value={filterDraft.op}
        onChange={(event) =>
          setFilterDraft((current) => ({
            ...current,
            op: event.target.value as FilterOp,
          }))
        }
        aria-label="Filter operator"
      >
        {FILTER_OPS.map((op) => (
          <option key={op.value} value={op.value}>
            {op.label}
          </option>
        ))}
      </Select>
      <input
        className="h-7 w-[200px] rounded-md border border-line bg-ink px-2 text-xs text-fg outline-none placeholder:text-muted-2 focus:border-accent disabled:opacity-40"
        placeholder="value"
        value={filterDraft.value}
        onChange={(event) =>
          setFilterDraft((current) => ({ ...current, value: event.target.value }))
        }
        onKeyDown={(event) => {
          if (event.key === "Enter") applyFilter();
        }}
        disabled={isNullOp(filterDraft.op)}
        aria-label="Filter value"
      />
      <Button
        variant="primary"
        size="sm"
        icon={Check}
        onClick={applyFilter}
        disabled={!filterDraft.column}
      >
        Apply
      </Button>
      {dataFilter ? (
        <>
          <IconButton icon={X} label="Clear filter" size="sm" onClick={clearFilter} />
          <Badge tone="accent">filtered</Badge>
        </>
      ) : null}
      {sort ? (
        <Badge tone="subtle">
          sorted by {sort.column} {sort.direction}
        </Badge>
      ) : null}
    </div>
  );
}

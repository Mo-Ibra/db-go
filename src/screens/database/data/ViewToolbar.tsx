import { ChevronLeft, ChevronRight, Download, Plus } from "lucide-react";
import type { ExportFormat } from "@/types";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Select } from "@/components/ui/Select";
import { PAGE_SIZES } from "@/components/ui/constants";
import { useWorkspace } from "@/app/providers/WorkspaceContext";

export function ViewToolbar() {
  const {
    view,
    schemaGraph,
    exportFormat,
    setExportFormat,
    exporting,
    selected,
    columns,
    pageSize,
    count,
    dataFilter,
    currentPage,
    nextDisabled,
    prevDisabled,
    nextPage,
    prevPage,
    changePageSize,
    setRowEditor,
    tableDetails,
    exportCurrent,
  } = useWorkspace();

  return (
    <div className="flex h-12 flex-none items-center justify-between gap-3 border-b border-line px-3.5">
      <div className="flex min-w-0 items-center gap-2">
        {view === "diagram" ? (
          <>
            <span className="truncate font-mono text-[13px] font-semibold">Schema</span>
            <Badge>{schemaGraph?.tables.length ?? 0} tables</Badge>
            <Badge>{schemaGraph?.relationships.length ?? 0} relations</Badge>
          </>
        ) : (
          <>
            <span
              data-testid="table-title"
              className="truncate font-mono text-[13px] font-semibold"
            >
              {selected}
            </span>
            <Badge>
              {view === "structure" && tableDetails?.name === selected
                ? tableDetails.columns.length
                : columns.length}{" "}
              columns
            </Badge>
            {view === "structure" && tableDetails?.name === selected ? (
              <Badge>{tableDetails.indexes.length} indexes</Badge>
            ) : null}
            {count && !dataFilter && view === "data" ? (
              <Badge>
                {count.capped
                  ? `${count.count.toLocaleString()}+`
                  : count.count.toLocaleString()}{" "}
                rows
              </Badge>
            ) : null}
          </>
        )}
      </div>

      <div className="flex items-center gap-2">
        {view === "data" ? (
          <>
            <Button
              variant="primary"
              size="sm"
              icon={Plus}
              onClick={() => setRowEditor({ mode: "insert", row: null })}
              disabled={columns.length === 0}
              data-testid="add-row"
              title="Add a row as a pending change"
            >
              Row
            </Button>
            <label className="flex items-center gap-1.5 text-[10.5px] font-semibold uppercase tracking-[0.08em] text-muted">
              Rows
              <Select
                value={pageSize}
                onChange={(event) => changePageSize(Number(event.target.value))}
                aria-label="Page size"
              >
                {PAGE_SIZES.map((size) => (
                  <option key={size} value={size}>
                    {size}
                  </option>
                ))}
              </Select>
            </label>
            <div className="flex items-center gap-1">
              <IconButton
                icon={ChevronLeft}
                label="Previous page"
                size="sm"
                onClick={() => void prevPage()}
                disabled={prevDisabled}
              />
              <span className="min-w-[58px] text-center text-[11px] tabular-nums text-muted">
                Page {currentPage + 1}
              </span>
              <IconButton
                icon={ChevronRight}
                label="Next page"
                size="sm"
                onClick={() => void nextPage()}
                disabled={nextDisabled}
              />
            </div>
            <span className="mx-1 h-4 w-px bg-line" />
            <Select
              value={exportFormat}
              onChange={(event) => setExportFormat(event.target.value as ExportFormat)}
              aria-label="Export format"
            >
              <option value="csv">CSV</option>
              <option value="json">JSON</option>
            </Select>
            <Button
              variant="ghost"
              size="sm"
              icon={Download}
              onClick={() => void exportCurrent()}
              disabled={exporting || !selected}
              data-testid="export-data"
              title="Export the current table view"
            >
              {exporting ? "Exporting..." : "Export"}
            </Button>
          </>
        ) : view === "structure" ? (
          <Badge>structure</Badge>
        ) : view === "sql" ? (
          <Badge>read-only</Badge>
        ) : (
          <Badge>schema</Badge>
        )}
      </div>
    </div>
  );
}

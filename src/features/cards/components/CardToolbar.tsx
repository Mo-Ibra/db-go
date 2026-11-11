import {
  ArrowDown,
  ArrowUp,
  ChevronLeft,
  ChevronRight,
  Eye,
  LayoutGrid,
  Pencil,
  RefreshCw,
  Search,
} from "lucide-react";
import type { ColumnInfo, CountResult, SortSpec } from "@/types";
import type { CardTemplate } from "@/features/cards/model";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Select } from "@/components/ui/Select";

type CardToolbarProps = {
  templates: CardTemplate[];
  activeTemplateId: string | null;
  onTemplateChange: (id: string) => void;
  columns: ColumnInfo[];
  search: string;
  onSearchChange: (value: string) => void;
  sort: SortSpec | null;
  onSortChange: (sort: SortSpec | null) => void;
  page: number;
  onPageChange: (page: number) => void;
  pageSize: number;
  onPageSizeChange: (size: number) => void;
  count: CountResult | null;
  loading: boolean;
  onRefresh: () => void;
  designing: boolean;
  onToggleDesign: () => void;
};

const PAGE_SIZES = [12, 24, 48];

export function CardToolbar({
  templates,
  activeTemplateId,
  onTemplateChange,
  columns,
  search,
  onSearchChange,
  sort,
  onSortChange,
  page,
  onPageChange,
  pageSize,
  onPageSizeChange,
  count,
  loading,
  onRefresh,
  designing,
  onToggleDesign,
}: CardToolbarProps) {
  const totalPages = count ? Math.max(1, Math.ceil(count.count / pageSize)) : null;
  const activeTemplate = templates.find((template) => template.id === activeTemplateId);

  return (
    <div className="flex flex-none flex-col gap-2 border-b border-line bg-panel/30 px-3.5 py-2.5">
      <div className="flex flex-wrap items-center gap-2">
        <div className="flex items-center gap-2">
          <LayoutGrid size={15} className="text-muted-2" />
          <Select
            value={activeTemplateId ?? ""}
            onChange={(event) => onTemplateChange(event.target.value)}
            aria-label="Card template"
            className="w-[150px]"
          >
            {templates.map((template) => (
              <option key={template.id} value={template.id}>
                {template.name}
              </option>
            ))}
          </Select>
          {activeTemplate ? <Badge>{activeTemplate.layout}</Badge> : null}
        </div>

        <label className="relative min-w-[220px] flex-1">
          <Search
            size={13}
            className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted-2"
          />
          <input
            className="field-input w-full pl-7"
            value={search}
            onChange={(event) => onSearchChange(event.target.value)}
            placeholder="Search cards..."
            aria-label="Search cards"
          />
        </label>

        <div className="flex items-center gap-1.5">
          <Select
            value={sort?.column ?? ""}
            onChange={(event) => {
              const column = event.target.value;
              onSortChange(column ? { column, direction: sort?.direction ?? "asc" } : null);
            }}
            aria-label="Sort by"
            className="w-[150px]"
          >
            <option value="">Default order</option>
            {columns.map((column) => (
              <option key={column.name} value={column.name}>
                {column.name}
              </option>
            ))}
          </Select>
          <IconButton
            icon={sort?.direction === "desc" ? ArrowDown : ArrowUp}
            label={sort?.direction === "desc" ? "Sort descending" : "Sort ascending"}
            size="sm"
            onClick={() => {
              if (!sort) return;
              onSortChange({
                column: sort.column,
                direction: sort.direction === "asc" ? "desc" : "asc",
              });
            }}
            disabled={!sort}
          />
        </div>

        <div className="flex items-center gap-1.5">
          <span className="text-[10.5px] font-semibold uppercase tracking-[0.08em] text-muted">
            Per page
          </span>
          <Select
            value={pageSize}
            onChange={(event) => onPageSizeChange(Number(event.target.value))}
            aria-label="Cards per page"
          >
            {PAGE_SIZES.map((size) => (
              <option key={size} value={size}>
                {size}
              </option>
            ))}
          </Select>
        </div>

        <Button
          variant={designing ? "primary" : "secondary"}
          size="sm"
          icon={designing ? Eye : Pencil}
          onClick={onToggleDesign}
        >
          {designing ? "Preview" : "Design"}
        </Button>
        <IconButton
          icon={RefreshCw}
          label="Refresh cards"
          size="sm"
          onClick={onRefresh}
          disabled={loading}
        />
      </div>

      <div className="flex items-center justify-between gap-2">
        <span className="text-[10.5px] text-muted-2">
          {count
            ? `${count.capped ? `${count.count.toLocaleString()}+` : count.count.toLocaleString()} records`
            : search.trim()
              ? "Filtered results"
              : "Records"}
        </span>
        <div className="flex items-center gap-1.5">
          <Button
            variant="ghost"
            size="sm"
            icon={ChevronLeft}
            onClick={() => onPageChange(Math.max(0, page - 1))}
            disabled={page <= 0 || loading}
          >
            Previous
          </Button>
          <span className="min-w-[90px] text-center text-[11px] tabular-nums text-muted">
            Page {page + 1}
            {totalPages ? ` of ${totalPages}` : ""}
          </span>
          <Button
            variant="ghost"
            size="sm"
            iconRight={ChevronRight}
            onClick={() => onPageChange(page + 1)}
            disabled={loading || (totalPages !== null && page + 1 >= totalPages)}
          >
            Next
          </Button>
        </div>
      </div>
    </div>
  );
}

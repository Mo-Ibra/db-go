import { Columns3 } from "lucide-react";
import type { ColumnInfo } from "@/types";
import type { CardTemplate } from "@/features/cards/model";
import { fieldLabel } from "@/features/cards/templates";
import { Select } from "@/components/ui/Select";
import { cn } from "@/lib/cn";
import { DesignerSection, FieldChips } from "./shared";

export function LayoutSettings({
  template,
  columns,
  onUpdate,
}: {
  template: CardTemplate;
  columns: ColumnInfo[];
  onUpdate: (template: CardTemplate) => void;
}) {
  return (
    <>
      <DesignerSection
        title="Grid"
        description="Choose how many cards fit across the page."
      >
        <div className="grid grid-cols-4 gap-1.5">
          {[1, 2, 3, 4].map((count) => (
            <button
              key={count}
              type="button"
              onClick={() => onUpdate({ ...template, columns: count as 1 | 2 | 3 | 4 })}
              className={cn(
                "flex h-12 flex-col items-center justify-center gap-1 rounded-lg border text-[10.5px] font-semibold transition-colors",
                template.columns === count
                  ? "border-accent/40 bg-accent/10 text-accent"
                  : "border-line bg-ink/40 text-muted hover:border-line-strong hover:bg-panel-2",
              )}
            >
              <Columns3 size={15} />
              {count}
            </button>
          ))}
        </div>
        <label className="flex flex-col gap-1">
          <span className="field-label">Cards per page</span>
          <Select
            value={template.pageSize}
            onChange={(event) =>
              onUpdate({ ...template, pageSize: Number(event.target.value) })
            }
          >
            <option value={12}>12 cards</option>
            <option value={24}>24 cards</option>
            <option value={48}>48 cards</option>
          </Select>
        </label>
      </DesignerSection>

      <DesignerSection
        title="Search"
        description="Fields used by the search box at the top of the Cards screen."
      >
        <FieldChips
          label="Search fields"
          help="Search checks these fields first. Keep it focused."
          columns={columns}
          fields={template.searchFields.map((column) => ({
            column,
            label: fieldLabel(column),
          }))}
          onChange={(fields) =>
            onUpdate({
              ...template,
              searchFields: fields.map((field) => field.column),
            })
          }
          max={4}
          kind="text"
          emptyText="Using the title field."
        />
      </DesignerSection>

      <DesignerSection
        title="Default sort"
        description="The order used when someone opens this template."
      >
        <div className="grid grid-cols-[1fr_120px] gap-2">
          <Select
            value={template.sort?.column ?? ""}
            onChange={(event) => {
              const column = event.target.value;
              onUpdate({
                ...template,
                sort: column
                  ? { column, direction: template.sort?.direction ?? "asc" }
                  : null,
              });
            }}
          >
            <option value="">No default sort</option>
            {columns.map((column) => (
              <option key={column.name} value={column.name}>
                {fieldLabel(column.name)}
              </option>
            ))}
          </Select>
          <Select
            value={template.sort?.direction ?? "asc"}
            onChange={(event) =>
              onUpdate({
                ...template,
                sort: template.sort
                  ? {
                      column: template.sort.column,
                      direction: event.target.value as "asc" | "desc",
                    }
                  : null,
              })
            }
            disabled={!template.sort}
          >
            <option value="asc">Ascending</option>
            <option value="desc">Descending</option>
          </Select>
        </div>
      </DesignerSection>
    </>
  );
}

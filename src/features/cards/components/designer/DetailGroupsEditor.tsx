import { Plus, Trash2, X } from "lucide-react";
import type { ColumnInfo } from "@/types";
import type { CardTemplate, FieldRef } from "@/features/cards/model";
import { fieldRefFor, fieldLabel } from "@/features/cards/templates";
import { fieldText } from "@/features/cards/renderers";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Select } from "@/components/ui/Select";
import { DesignerSection, isBlob } from "./shared";

export function DetailGroupsEditor({
  template,
  columns,
  updateDetailGroups,
}: {
  template: CardTemplate;
  columns: ColumnInfo[];
  updateDetailGroups: (groups: CardTemplate["detail"]["groups"]) => void;
}) {
  const renameGroup = (groupId: string, title: string) => {
    updateDetailGroups(
      template.detail.groups.map((group) =>
        group.id === groupId ? { ...group, title } : group,
      ),
    );
  };

  const removeGroupField = (groupId: string, column: string) => {
    updateDetailGroups(
      template.detail.groups.map((group) =>
        group.id === groupId
          ? {
              ...group,
              fields: group.fields.filter((field) => field.column !== column),
            }
          : group,
      ),
    );
  };

  const addGroupField = (groupId: string, field: FieldRef) => {
    updateDetailGroups(
      template.detail.groups.map((group) =>
        group.id === groupId
          ? {
              ...group,
              fields: group.fields.some((item) => item.column === field.column)
                ? group.fields
                : [...group.fields, field],
            }
          : group,
      ),
    );
  };

  return (
    <DesignerSection
      title="Detail page sections"
      description="These sections appear when someone opens a full record page. Group related fields together."
    >
      {template.detail.groups.map((group) => {
        const available = columns.filter(
          (column) =>
            !isBlob(column) &&
            !group.fields.some((field) => field.column === column.name),
        );
        return (
          <div
            key={group.id}
            className="rounded-xl border border-line bg-ink/40 p-3"
          >
            <div className="flex items-center gap-2">
              <input
                className="field-input flex-1 text-[12px] font-semibold"
                value={group.title}
                onChange={(event) => renameGroup(group.id, event.target.value)}
                aria-label="Detail group title"
              />
              <IconButton
                icon={Trash2}
                label="Delete group"
                variant="danger"
                size="sm"
                onClick={() =>
                  updateDetailGroups(
                    template.detail.groups.filter((item) => item.id !== group.id),
                  )
                }
              />
            </div>
            <div className="mt-2 flex flex-wrap gap-1.5">
              {group.fields.length === 0 ? (
                <span className="text-[10.5px] text-muted-2">
                  No fields in this section.
                </span>
              ) : (
                group.fields.map((field) => (
                  <span
                    key={field.column}
                    className="inline-flex items-center gap-1 rounded-full border border-line bg-panel-2 py-0.5 pl-2 pr-1 text-[10.5px] text-fg"
                  >
                    {fieldText(field)}
                    <button
                      type="button"
                      aria-label={`Remove ${fieldText(field)}`}
                      className="grid h-4 w-4 place-items-center rounded-full text-muted-2 hover:bg-panel-3 hover:text-fg"
                      onClick={() => removeGroupField(group.id, field.column)}
                    >
                      <X size={10} />
                    </button>
                  </span>
                ))
              )}
            </div>
            <Select
              className="mt-2 w-full"
              value=""
              onChange={(event) => {
                const column = columns.find(
                  (item) => item.name === event.target.value,
                );
                const field = fieldRefFor(column);
                if (field) addGroupField(group.id, field);
              }}
              disabled={available.length === 0}
            >
              <option value="">Add a field to this section...</option>
              {available.map((column) => (
                <option key={column.name} value={column.name}>
                  {fieldLabel(column.name)}
                </option>
              ))}
            </Select>
          </div>
        );
      })}
      <Button
        variant="secondary"
        size="sm"
        icon={Plus}
        onClick={() =>
          updateDetailGroups([
            ...template.detail.groups,
            {
              id: `group-${Date.now()}`,
              title: "New section",
              fields: [],
            },
          ])
        }
      >
        Add section
      </Button>
    </DesignerSection>
  );
}

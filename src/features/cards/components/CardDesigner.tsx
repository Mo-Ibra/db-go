import { useEffect, useState } from "react";
import { Copy, Plus, Trash2 } from "lucide-react";
import type { ColumnInfo } from "@/types";
import type { CardLayout, CardTemplate, FieldRef } from "@/features/cards/model";
import { fieldRefFor, fieldLabel } from "@/features/cards/templates";
import { IconButton } from "@/components/ui/IconButton";
import { Select } from "@/components/ui/Select";

type CardDesignerProps = {
  template: CardTemplate;
  templates: CardTemplate[];
  columns: ColumnInfo[];
  onUpdate: (template: CardTemplate) => void;
  onChangeLayout: (layout: CardLayout) => void;
  onRename: (name: string) => void;
  onCreate: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
};

const LAYOUTS: { value: CardLayout; label: string }[] = [
  { value: "article", label: "Article" },
  { value: "profile", label: "Profile" },
  { value: "compact", label: "Compact" },
  { value: "metric", label: "Metric" },
  { value: "keyValue", label: "Key / Value" },
];

function isBlob(column: ColumnInfo): boolean {
  return column.type_name.toUpperCase().includes("BLOB");
}

function isNumeric(column: ColumnInfo): boolean {
  return /INT|REAL|NUM|FLOAT|DOUBLE|DEC/i.test(column.type_name);
}

function FieldSelect({
  label,
  columns,
  field,
  onChange,
  kind = "any",
}: {
  label: string;
  columns: ColumnInfo[];
  field?: FieldRef;
  onChange: (field: FieldRef | undefined) => void;
  kind?: "any" | "text" | "numeric" | "blob";
}) {
  const options = columns.filter((column) => {
    if (kind === "text") return !isBlob(column);
    if (kind === "numeric") return isNumeric(column);
    if (kind === "blob") return isBlob(column);
    return true;
  });

  return (
    <label className="flex flex-col gap-1">
      <span className="field-label">{label}</span>
      <Select
        value={field?.column ?? ""}
        onChange={(event) => {
          const column = columns.find((item) => item.name === event.target.value);
          onChange(fieldRefFor(column));
        }}
      >
        <option value="">None</option>
        {options.map((column) => (
          <option key={column.name} value={column.name}>
            {fieldLabel(column.name)}
          </option>
        ))}
      </Select>
    </label>
  );
}

function FieldMultiSelect({
  label,
  columns,
  fields,
  onChange,
  max = 6,
  kind = "any",
}: {
  label: string;
  columns: ColumnInfo[];
  fields: FieldRef[];
  onChange: (fields: FieldRef[]) => void;
  max?: number;
  kind?: "any" | "text" | "numeric";
}) {
  const options = columns.filter((column) => {
    if (kind === "text") return !isBlob(column);
    if (kind === "numeric") return isNumeric(column);
    return true;
  });

  const toggle = (column: ColumnInfo) => {
    const exists = fields.some((field) => field.column === column.name);
    if (exists) {
      onChange(fields.filter((field) => field.column !== column.name));
      return;
    }
    if (fields.length >= max) return;
    const field = fieldRefFor(column);
    if (field) onChange([...fields, field]);
  };

  return (
    <div className="flex flex-col gap-1.5">
      <span className="field-label">{label}</span>
      <div className="max-h-[150px] overflow-auto rounded-md border border-line bg-ink p-2">
        {options.map((column) => {
          const checked = fields.some((field) => field.column === column.name);
          return (
            <label
              key={column.name}
              className="flex cursor-pointer items-center gap-2 rounded px-1 py-0.5 text-[11px] text-muted hover:bg-panel-2 hover:text-fg"
            >
              <input
                type="checkbox"
                className="h-3.5 w-3.5 accent-accent"
                checked={checked}
                onChange={() => toggle(column)}
              />
              <span className="truncate">{fieldLabel(column.name)}</span>
            </label>
          );
        })}
      </div>
    </div>
  );
}

export function CardDesigner({
  template,
  templates,
  columns,
  onUpdate,
  onChangeLayout,
  onRename,
  onCreate,
  onDuplicate,
  onDelete,
}: CardDesignerProps) {
  const [name, setName] = useState(template.name);

  useEffect(() => {
    setName(template.name);
  }, [template.id, template.name]);

  const updateCard = (patch: Partial<CardTemplate["card"]>) =>
    onUpdate({ ...template, card: { ...template.card, ...patch } });

  const setField = (key: keyof CardTemplate["card"], field: FieldRef | undefined) =>
    updateCard({ [key]: field });

  const updateGroupTitle = (groupId: string, title: string) => {
    onUpdate({
      ...template,
      detail: {
        ...template.detail,
        groups: template.detail.groups.map((group) =>
          group.id === groupId ? { ...group, title } : group,
        ),
      },
    });
  };

  const toggleGroupField = (groupId: string, column: ColumnInfo) => {
    onUpdate({
      ...template,
      detail: {
        ...template.detail,
        groups: template.detail.groups.map((group) => {
          if (group.id !== groupId) return group;
          const exists = group.fields.some((field) => field.column === column.name);
          if (exists) {
            return {
              ...group,
              fields: group.fields.filter((field) => field.column !== column.name),
            };
          }
          const field = fieldRefFor(column);
          return field ? { ...group, fields: [...group.fields, field] } : group;
        }),
      },
    });
  };

  return (
    <aside className="flex w-[350px] min-w-[300px] flex-none flex-col overflow-auto border-r border-line bg-panel/50">
      <div className="border-b border-line p-4">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted-2">
            Template
          </span>
          <span className="text-[10px] text-muted-2">Saved automatically</span>
        </div>
        <input
          className="field-input mt-2 w-full"
          value={name}
          onChange={(event) => setName(event.target.value)}
          onBlur={() => {
            const trimmed = name.trim();
            if (trimmed && trimmed !== template.name) onRename(trimmed);
            if (!trimmed) setName(template.name);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") event.currentTarget.blur();
          }}
          aria-label="Template name"
        />
        <div className="mt-2 grid grid-cols-[1fr_auto] gap-2">
          <Select
            value={template.layout}
            onChange={(event) => onChangeLayout(event.target.value as CardLayout)}
            aria-label="Card layout"
          >
            {LAYOUTS.map((layout) => (
              <option key={layout.value} value={layout.value}>
                {layout.label}
              </option>
            ))}
          </Select>
          <div className="flex gap-1">
            <IconButton icon={Plus} label="New template" onClick={onCreate} />
            <IconButton icon={Copy} label="Duplicate template" onClick={onDuplicate} />
            <IconButton
              icon={Trash2}
              label="Delete template"
              variant="danger"
              onClick={onDelete}
              disabled={templates.length <= 1}
            />
          </div>
        </div>
      </div>

      <div className="space-y-4 border-b border-line p-4">
        <h3 className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted">
          Card slots
        </h3>

        <FieldSelect
          label="Title"
          columns={columns}
          field={template.card.title}
          onChange={(field) => setField("title", field)}
          kind="text"
        />
        <FieldSelect
          label="Subtitle"
          columns={columns}
          field={template.card.subtitle}
          onChange={(field) => setField("subtitle", field)}
          kind="text"
        />

        {template.layout === "metric" ? (
          <>
            <FieldSelect
              label="Metric value"
              columns={columns}
              field={template.card.value}
              onChange={(field) => setField("value", field)}
              kind="numeric"
            />
            <label className="flex flex-col gap-1">
              <span className="field-label">Metric label</span>
              <input
                className="field-input"
                value={template.card.valueLabel ?? ""}
                onChange={(event) => updateCard({ valueLabel: event.target.value })}
                placeholder="Revenue"
              />
            </label>
            <FieldSelect
              label="Footnote"
              columns={columns}
              field={template.card.footnote}
              onChange={(field) => setField("footnote", field)}
              kind="text"
            />
          </>
        ) : (
          <FieldSelect
            label="Description"
            columns={columns}
            field={template.card.description}
            onChange={(field) => setField("description", field)}
            kind="text"
          />
        )}

        {template.layout === "profile" ? (
          <FieldSelect
            label="Image"
            columns={columns}
            field={template.card.image}
            onChange={(field) => setField("image", field)}
            kind="blob"
          />
        ) : null}

        {template.layout === "keyValue" ? (
          <FieldMultiSelect
            label="Key / value fields"
            columns={columns}
            fields={template.card.fields}
            onChange={(fields) => updateCard({ fields })}
            max={8}
            kind="text"
          />
        ) : null}

        <FieldMultiSelect
          label="Badges"
          columns={columns}
          fields={template.card.badges}
          onChange={(fields) => updateCard({ badges: fields })}
          max={4}
          kind="text"
        />

        <FieldMultiSelect
          label="Metadata"
          columns={columns}
          fields={template.card.metadata}
          onChange={(fields) => updateCard({ metadata: fields })}
          max={4}
          kind="text"
        />

        <div className="grid grid-cols-2 gap-2">
          <label className="flex flex-col gap-1">
            <span className="field-label">Grid columns</span>
            <Select
              value={template.columns}
              onChange={(event) =>
                onUpdate({
                  ...template,
                  columns: Number(event.target.value) as 1 | 2 | 3 | 4,
                })
              }
            >
              <option value={1}>1</option>
              <option value={2}>2</option>
              <option value={3}>3</option>
              <option value={4}>4</option>
            </Select>
          </label>
          <label className="flex flex-col gap-1">
            <span className="field-label">Cards per page</span>
            <Select
              value={template.pageSize}
              onChange={(event) =>
                onUpdate({ ...template, pageSize: Number(event.target.value) })
              }
            >
              <option value={12}>12</option>
              <option value={24}>24</option>
              <option value={48}>48</option>
            </Select>
          </label>
        </div>
      </div>

      <div className="space-y-3 p-4">
        <h3 className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted">
          Detail groups
        </h3>
        {template.detail.groups.map((group) => (
          <div key={group.id} className="rounded-lg border border-line bg-ink/40 p-2.5">
            <input
              className="field-input w-full"
              value={group.title}
              onChange={(event) => updateGroupTitle(group.id, event.target.value)}
              aria-label="Detail group title"
            />
            <div className="mt-2 max-h-[120px] overflow-auto">
              {columns
                .filter((column) => !isBlob(column))
                .map((column) => (
                  <label
                    key={column.name}
                    className="flex cursor-pointer items-center gap-2 rounded px-1 py-0.5 text-[11px] text-muted hover:bg-panel-2 hover:text-fg"
                  >
                    <input
                      type="checkbox"
                      className="h-3.5 w-3.5 accent-accent"
                      checked={group.fields.some(
                        (field) => field.column === column.name,
                      )}
                      onChange={() => toggleGroupField(group.id, column)}
                    />
                    <span className="truncate">{fieldLabel(column.name)}</span>
                  </label>
                ))}
            </div>
          </div>
        ))}
      </div>
    </aside>
  );
}

import { useEffect, useState } from "react";
import {
  AlignLeft,
  Columns3,
  Copy,
  Eye,
  Heading,
  Image,
  LayoutGrid,
  List,
  Plus,
  Rows3,
  Tag,
  Trash2,
  Type,
  X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ColumnInfo } from "@/types";
import type { CardLayout, CardTemplate, FieldRef } from "@/features/cards/model";
import { fieldRefFor, fieldLabel } from "@/features/cards/templates";
import { fieldText } from "@/features/cards/renderers";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Select } from "@/components/ui/Select";
import { cn } from "@/lib/cn";

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
  onPreviewRecord: () => void;
  canPreviewRecord: boolean;
  tab: DesignerTab;
  onTabChange: (tab: DesignerTab) => void;
};

export type DesignerTab = "card" | "details" | "layout";
type FieldKind = "any" | "text" | "numeric" | "blob";

const LAYOUTS: {
  value: CardLayout;
  label: string;
  description: string;
  icon: LucideIcon;
}[] = [
  {
    value: "article",
    label: "Article",
    description: "Title, summary, labels",
    icon: AlignLeft,
  },
  {
    value: "profile",
    label: "Profile",
    description: "Avatar, identity, contact",
    icon: Image,
  },
  {
    value: "compact",
    label: "Compact",
    description: "Dense one-line records",
    icon: Rows3,
  },
  {
    value: "metric",
    label: "Metric",
    description: "One big number",
    icon: Type,
  },
  {
    value: "keyValue",
    label: "Key / Value",
    description: "Label and value grid",
    icon: List,
  },
];

function isBlob(column: ColumnInfo): boolean {
  return column.type_name.toUpperCase().includes("BLOB");
}

function isNumeric(column: ColumnInfo): boolean {
  return /INT|REAL|NUM|FLOAT|DOUBLE|DEC/i.test(column.type_name);
}

function filteredColumns(columns: ColumnInfo[], kind: FieldKind): ColumnInfo[] {
  if (kind === "text") return columns.filter((column) => !isBlob(column));
  if (kind === "numeric") return columns.filter(isNumeric);
  if (kind === "blob") return columns.filter(isBlob);
  return columns;
}

function DesignerSection({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="border-b border-line/70 px-4 py-4 last:border-b-0">
      <h3 className="text-[11px] font-bold uppercase tracking-[0.12em] text-fg">
        {title}
      </h3>
      {description ? (
        <p className="mt-1 text-[11.5px] leading-relaxed text-muted-2">
          {description}
        </p>
      ) : null}
      <div className="mt-3 space-y-3">{children}</div>
    </section>
  );
}

function SlotRow({
  icon: Icon,
  label,
  help,
  columns,
  field,
  onChange,
  kind = "any",
}: {
  icon: LucideIcon;
  label: string;
  help: string;
  columns: ColumnInfo[];
  field?: FieldRef;
  onChange: (field: FieldRef | undefined) => void;
  kind?: FieldKind;
}) {
  const options = filteredColumns(columns, kind);
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_190px] items-start gap-3">
      <div className="min-w-0">
        <div className="flex items-center gap-1.5 text-[12px] font-medium text-fg">
          <Icon size={13} className="text-muted-2" />
          {label}
        </div>
        <p className="mt-0.5 text-[10.5px] leading-relaxed text-muted-2">{help}</p>
      </div>
      <Select
        value={field?.column ?? ""}
        onChange={(event) => {
          const column = columns.find((item) => item.name === event.target.value);
          onChange(fieldRefFor(column));
        }}
        aria-label={label}
      >
        <option value="">None</option>
        {options.map((column) => (
          <option key={column.name} value={column.name}>
            {fieldLabel(column.name)}
          </option>
        ))}
      </Select>
    </div>
  );
}

function FieldChips({
  label,
  help,
  columns,
  fields,
  onChange,
  max = 6,
  kind = "any",
  emptyText = "No fields selected yet.",
}: {
  label: string;
  help: string;
  columns: ColumnInfo[];
  fields: FieldRef[];
  onChange: (fields: FieldRef[]) => void;
  max?: number;
  kind?: FieldKind;
  emptyText?: string;
}) {
  const options = filteredColumns(columns, kind).filter(
    (column) => !fields.some((field) => field.column === column.name),
  );

  return (
    <div>
      <div className="text-[12px] font-medium text-fg">{label}</div>
      <p className="mt-0.5 text-[10.5px] leading-relaxed text-muted-2">{help}</p>
      <div className="mt-2 flex min-h-[30px] flex-wrap gap-1.5 rounded-lg border border-line bg-ink/50 p-2">
        {fields.length === 0 ? (
          <span className="px-1 py-0.5 text-[11px] text-muted-2">{emptyText}</span>
        ) : (
          fields.map((field) => (
            <span
              key={field.column}
              className="inline-flex items-center gap-1 rounded-full border border-accent/25 bg-accent/10 py-0.5 pl-2 pr-1 text-[10.5px] text-accent"
            >
              {fieldText(field)}
              <button
                type="button"
                aria-label={`Remove ${fieldText(field)}`}
                className="grid h-4 w-4 place-items-center rounded-full text-accent/70 hover:bg-accent/20 hover:text-accent"
                onClick={() =>
                  onChange(fields.filter((item) => item.column !== field.column))
                }
              >
                <X size={10} />
              </button>
            </span>
          ))
        )}
      </div>
      <div className="mt-2 flex items-center gap-2">
        <Select
          value=""
          onChange={(event) => {
            const column = columns.find((item) => item.name === event.target.value);
            const field = fieldRefFor(column);
            if (field) onChange([...fields, field]);
          }}
          disabled={options.length === 0 || fields.length >= max}
          aria-label={`Add ${label.toLowerCase()}`}
        >
          <option value="">
            {fields.length >= max ? `Limit reached (${max})` : "Add a field..."}
          </option>
          {options.map((column) => (
            <option key={column.name} value={column.name}>
              {fieldLabel(column.name)}
            </option>
          ))}
        </Select>
        <span className="text-[10px] text-muted-2">
          {fields.length}/{max}
        </span>
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
  onPreviewRecord,
  canPreviewRecord,
  tab,
  onTabChange,
}: CardDesignerProps) {
  const [name, setName] = useState(template.name);

  useEffect(() => {
    setName(template.name);
  }, [template.id, template.name]);

  const updateCard = (patch: Partial<CardTemplate["card"]>) =>
    onUpdate({ ...template, card: { ...template.card, ...patch } });

  const updateDetailGroups = (
    groups: CardTemplate["detail"]["groups"],
  ) => onUpdate({ ...template, detail: { ...template.detail, groups } });

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

  const tabs: { id: DesignerTab; label: string; icon: LucideIcon }[] = [
    { id: "card", label: "Card", icon: LayoutGrid },
    { id: "details", label: "Detail page", icon: List },
    { id: "layout", label: "Layout", icon: Columns3 },
  ];

  return (
    <aside className="flex w-[390px] min-w-[340px] flex-none flex-col border-r border-line bg-panel/40">
      <div className="border-b border-line p-4">
        <div className="flex items-center justify-between gap-2">
          <div className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted-2">
            Card designer
          </div>
          <span className="text-[10px] text-muted-2">Saves automatically</span>
        </div>
        <input
          className="field-input mt-2 w-full text-[13px] font-semibold"
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
        <div className="mt-2 flex items-center gap-1.5">
          <Button variant="secondary" size="sm" icon={Plus} onClick={onCreate}>
            New
          </Button>
          <Button variant="ghost" size="sm" icon={Copy} onClick={onDuplicate}>
            Duplicate
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={Eye}
            onClick={onPreviewRecord}
            disabled={!canPreviewRecord}
          >
            Preview page
          </Button>
          <IconButton
            icon={Trash2}
            label="Delete template"
            variant="danger"
            size="sm"
            onClick={onDelete}
            disabled={templates.length <= 1}
          />
        </div>
        <div className="mt-3 grid grid-cols-3 gap-1 rounded-lg border border-line bg-ink p-1">
          {tabs.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                className={cn(
                  "flex h-8 items-center justify-center gap-1.5 rounded-md text-[11px] font-semibold transition-colors",
                  tab === item.id
                    ? "bg-panel-3 text-fg shadow-sm"
                    : "text-muted hover:bg-panel-2 hover:text-fg",
                )}
                onClick={() => onTabChange(item.id)}
              >
                <Icon size={13} />
                {item.label}
              </button>
            );
          })}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {tab === "card" ? (
          <>
            <DesignerSection
              title="Choose a card style"
              description="Pick the layout that best matches how this data should be read."
            >
              <div className="grid grid-cols-2 gap-2">
                {LAYOUTS.map((layout) => {
                  const Icon = layout.icon;
                  const active = template.layout === layout.value;
                  return (
                    <button
                      key={layout.value}
                      type="button"
                      onClick={() => onChangeLayout(layout.value)}
                      className={cn(
                        "flex items-start gap-2 rounded-lg border p-2.5 text-left transition-colors",
                        active
                          ? "border-accent/40 bg-accent/10"
                          : "border-line bg-ink/40 hover:border-line-strong hover:bg-panel-2",
                      )}
                    >
                      <Icon
                        size={15}
                        className={active ? "text-accent" : "text-muted-2"}
                      />
                      <span>
                        <span className="block text-[11.5px] font-semibold text-fg">
                          {layout.label}
                        </span>
                        <span className="mt-0.5 block text-[10px] leading-relaxed text-muted-2">
                          {layout.description}
                        </span>
                      </span>
                    </button>
                  );
                })}
              </div>
            </DesignerSection>

            <DesignerSection
              title="Card content"
              description="Only these fields appear on the card. Everything else stays on the detail page."
            >
              <SlotRow
                icon={Heading}
                label="Title"
                help="The main heading people scan first."
                columns={columns}
                field={template.card.title}
                onChange={(field) => updateCard({ title: field })}
                kind="text"
              />
              <SlotRow
                icon={Type}
                label="Subtitle"
                help="Short context, such as city, category, or role."
                columns={columns}
                field={template.card.subtitle}
                onChange={(field) => updateCard({ subtitle: field })}
                kind="text"
              />
              {template.layout === "metric" ? (
                <>
                  <SlotRow
                    icon={Type}
                    label="Metric value"
                    help="The big number shown in the center of the card."
                    columns={columns}
                    field={template.card.value}
                    onChange={(field) => updateCard({ value: field })}
                    kind="numeric"
                  />
                  <label className="flex flex-col gap-1">
                    <span className="flex items-center gap-1.5 text-[12px] font-medium text-fg">
                      <Tag size={13} className="text-muted-2" />
                      Metric label
                    </span>
                    <span className="text-[10.5px] text-muted-2">
                      Describes the number, for example “Revenue” or “Employees”.
                    </span>
                    <input
                      className="field-input mt-1"
                      value={template.card.valueLabel ?? ""}
                      onChange={(event) =>
                        updateCard({ valueLabel: event.target.value })
                      }
                      placeholder="Revenue"
                    />
                  </label>
                  <SlotRow
                    icon={AlignLeft}
                    label="Footnote"
                    help="Small context under the number."
                    columns={columns}
                    field={template.card.footnote}
                    onChange={(field) => updateCard({ footnote: field })}
                    kind="text"
                  />
                </>
              ) : (
                <SlotRow
                  icon={AlignLeft}
                  label="Description"
                  help="A short summary. Full text appears on the detail page."
                  columns={columns}
                  field={template.card.description}
                  onChange={(field) => updateCard({ description: field })}
                  kind="text"
                />
              )}
              {template.layout === "profile" ? (
                <SlotRow
                  icon={Image}
                  label="Image"
                  help="Avatar or logo slot. Image rendering arrives in a later phase."
                  columns={columns}
                  field={template.card.image}
                  onChange={(field) => updateCard({ image: field })}
                  kind="blob"
                />
              ) : null}
            </DesignerSection>

            <DesignerSection
              title="Labels and facts"
              description="Badges are short labels. Metadata appears as key facts in the card footer."
            >
              {template.layout === "keyValue" ? (
                <FieldChips
                  label="Key / value fields"
                  help="The label/value pairs shown inside the card."
                  columns={columns}
                  fields={template.card.fields}
                  onChange={(fields) => updateCard({ fields })}
                  max={8}
                  kind="text"
                />
              ) : null}
              <FieldChips
                label="Badges"
                help="Small pills such as status, sector, type, or category."
                columns={columns}
                fields={template.card.badges}
                onChange={(fields) => updateCard({ badges: fields })}
                max={4}
                kind="text"
              />
              <FieldChips
                label="Metadata"
                help="Short facts in the card footer, such as employees or country."
                columns={columns}
                fields={template.card.metadata}
                onChange={(fields) => updateCard({ metadata: fields })}
                max={4}
                kind="text"
              />
            </DesignerSection>
          </>
        ) : null}

        {tab === "details" ? (
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
                      onChange={(event) =>
                        renameGroup(group.id, event.target.value)
                      }
                      aria-label="Detail group title"
                    />
                    <IconButton
                      icon={Trash2}
                      label="Delete group"
                      variant="danger"
                      size="sm"
                      onClick={() =>
                        updateDetailGroups(
                          template.detail.groups.filter(
                            (item) => item.id !== group.id,
                          ),
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
                            onClick={() =>
                              removeGroupField(group.id, field.column)
                            }
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
        ) : null}

        {tab === "layout" ? (
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
                    onClick={() =>
                      onUpdate({
                        ...template,
                        columns: count as 1 | 2 | 3 | 4,
                      })
                    }
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
                    onUpdate({
                      ...template,
                      pageSize: Number(event.target.value),
                    })
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
                        ? {
                            column,
                            direction: template.sort?.direction ?? "asc",
                          }
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
        ) : null}
      </div>
    </aside>
  );
}

import { X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ColumnInfo } from "@/types";
import type { FieldRef } from "@/features/cards/model";
import { fieldRefFor, fieldLabel } from "@/features/cards/templates";
import { fieldText } from "@/features/cards/renderers";
import { Select } from "@/components/ui/Select";

export type FieldKind = "any" | "text" | "numeric" | "blob";

export function isBlob(column: ColumnInfo): boolean {
  return column.type_name.toUpperCase().includes("BLOB");
}

export function isNumeric(column: ColumnInfo): boolean {
  return /INT|REAL|NUM|FLOAT|DOUBLE|DEC/i.test(column.type_name);
}

export function filteredColumns(
  columns: ColumnInfo[],
  kind: FieldKind,
): ColumnInfo[] {
  if (kind === "text") return columns.filter((column) => !isBlob(column));
  if (kind === "numeric") return columns.filter(isNumeric);
  if (kind === "blob") return columns.filter(isBlob);
  return columns;
}

export function DesignerSection({
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

export function SlotRow({
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

export function FieldChips({
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

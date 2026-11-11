import type { ColumnInfo } from "@/types";
import type {
  CardLayout,
  CardTemplate,
  CardTemplateStore,
  FieldFormat,
  FieldRef,
} from "./model";

const STORAGE_PREFIX = "dbgo:cards:v1";

const LABELS: Record<string, string> = {
  id: "ID",
  revenue_usd: "Revenue (USD)",
  founded_year: "Founded",
  created_at: "Created",
  updated_at: "Updated",
  company_id: "Company ID",
};

export function cardTemplateStorageKey(dbPath: string, table: string): string {
  return `${STORAGE_PREFIX}:${dbPath}:${table}`;
}

export function fieldLabel(name: string): string {
  if (LABELS[name]) return LABELS[name];
  return name
    .split("_")
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function isBlob(column: ColumnInfo): boolean {
  return column.type_name.toUpperCase().includes("BLOB");
}

function isNumeric(column: ColumnInfo): boolean {
  return /INT|REAL|NUM|FLOAT|DOUBLE|DEC/i.test(column.type_name);
}

function isText(column: ColumnInfo): boolean {
  return /TEXT|CHAR|CLOB|JSON/i.test(column.type_name);
}

function formatFor(column: ColumnInfo): FieldFormat {
  const name = column.name.toLowerCase();
  if (name.includes("email")) return "email";
  if (name.includes("url") || name.includes("website")) return "url";
  if (name.includes("date") || name.endsWith("_at")) return "date";
  if (name === "active" || name.startsWith("is_") || name.startsWith("has_")) {
    return "boolean";
  }
  if (
    name.includes("revenue") ||
    name.includes("amount") ||
    name.includes("price") ||
    name.includes("cost") ||
    name.includes("salary") ||
    name.includes("budget") ||
    name.includes("balance")
  ) {
    return "currency";
  }
  if (isNumeric(column)) return "number";
  return "text";
}

export function fieldRefFor(column: ColumnInfo | undefined): FieldRef | undefined {
  if (!column) return undefined;
  return {
    column: column.name,
    label: fieldLabel(column.name),
    format: formatFor(column),
  };
}

function findColumn(columns: ColumnInfo[], ...names: string[]): ColumnInfo | undefined {
  for (const name of names) {
    const found = columns.find((column) => column.name.toLowerCase() === name);
    if (found) return found;
  }
  return undefined;
}

export function guessLayout(columns: ColumnInfo[]): CardLayout {
  const hasImage = columns.some(
    (column) =>
      isBlob(column) &&
      /image|logo|avatar|photo|picture|cover/i.test(column.name),
  );
  const hasLongText = columns.some(
    (column) =>
      isText(column) &&
      /description|body|summary|content|notes|bio|details|excerpt/i.test(column.name),
  );
  const numericCount = columns.filter(isNumeric).length;
  const scalarCount = columns.filter((column) => !isBlob(column)).length;

  if (hasImage) return "profile";
  if (hasLongText) return "article";
  if (numericCount >= 2 && scalarCount <= 8) return "metric";
  if (scalarCount > 6 && !hasLongText) return "keyValue";
  return "compact";
}

function buildDetailGroups(
  columns: ColumnInfo[],
  title?: FieldRef,
  subtitle?: FieldRef,
  description?: FieldRef,
) {
  const used = new Set<string>();
  const take = (column: ColumnInfo | undefined) => {
    if (!column || used.has(column.name)) return undefined;
    used.add(column.name);
    return fieldRefFor(column);
  };

  const overview = [title, subtitle, description].filter(Boolean) as FieldRef[];
  overview.forEach((field) => used.add(field.column));

  const contactColumns = ["website", "email", "phone", "country", "city"]
    .map((name) => findColumn(columns, name))
    .map((column) => take(column))
    .filter(Boolean) as FieldRef[];

  const systemColumns = ["id", "created_at", "updated_at"]
    .map((name) => findColumn(columns, name))
    .map((column) => take(column))
    .filter(Boolean) as FieldRef[];

  const details = columns
    .map((column) => take(column))
    .filter(Boolean) as FieldRef[];

  return [
    { id: "overview", title: "Overview", fields: overview },
    { id: "details", title: "Details", fields: details },
    { id: "contact", title: "Contact", fields: contactColumns },
    { id: "system", title: "System", fields: systemColumns },
  ].filter((group) => group.fields.length > 0);
}

export function buildTemplates(table: string, columns: ColumnInfo[]): CardTemplateStore {
  const titleColumn =
    findColumn(columns, "name", "title", "label", "subject") ??
    columns.find((column) => isText(column) && !isBlob(column));
  const subtitleColumn =
    findColumn(columns, "city", "sector", "category", "type", "status", "role") ??
    columns.find(
      (column) => isText(column) && column.name !== titleColumn?.name,
    );
  const descriptionColumn =
    findColumn(
      columns,
      "description",
      "summary",
      "body",
      "content",
      "notes",
      "bio",
      "excerpt",
    ) ?? columns.find((column) => isText(column) && column.name.includes("desc"));
  const imageColumn = columns.find(
    (column) =>
      isBlob(column) &&
      /image|logo|avatar|photo|picture|cover/i.test(column.name),
  );
  const valueColumn =
    columns.find(
      (column) =>
        isNumeric(column) &&
        /revenue|amount|total|count|score|value|price|salary|budget|cost/i.test(
          column.name,
        ),
    ) ?? columns.find(isNumeric);
  const statusColumn = findColumn(columns, "status", "active", "state", "published");
  const badgeColumns = ["sector", "category", "type", "tags", "status"]
    .map((name) => findColumn(columns, name))
    .filter((column): column is ColumnInfo => Boolean(column));
  const metadataColumns = columns
    .filter(
      (column) =>
        !column.primary_key &&
        !isBlob(column) &&
        column.name !== titleColumn?.name &&
        column.name !== descriptionColumn?.name &&
        column.name !== statusColumn?.name,
    )
    .slice(0, 4);

  const title = fieldRefFor(titleColumn);
  const subtitle = fieldRefFor(subtitleColumn);
  const description = fieldRefFor(descriptionColumn);
  const image = fieldRefFor(imageColumn);
  const status = fieldRefFor(statusColumn);
  const badges = [status, ...badgeColumns.map((column) => fieldRefFor(column))]
    .filter(Boolean)
    .filter(
      (field, index, all) =>
        all.findIndex((candidate) => candidate?.column === field?.column) === index,
    ) as FieldRef[];
  const metadata = metadataColumns.map((column) => fieldRefFor(column)).filter(Boolean) as FieldRef[];
  const value = fieldRefFor(valueColumn);
  const scalarFields = columns
    .filter(
      (column) =>
        !isBlob(column) &&
        column.name !== titleColumn?.name &&
        column.name !== imageColumn?.name &&
        !["id", "created_at", "updated_at"].includes(column.name),
    )
    .slice(0, 6)
    .map((column) => fieldRefFor(column))
    .filter(Boolean) as FieldRef[];

  const detail = {
    groups: buildDetailGroups(columns, title, subtitle, description),
    hidden: columns.filter(isBlob).map((column) => column.name),
  };

  const base = {
    table,
    detail,
    columns: 3 as const,
    pageSize: 12,
    searchFields: [title?.column, subtitle?.column, description?.column].filter(
      Boolean,
    ) as string[],
    sort: null,
  };

  const templates: CardTemplate[] = [
    {
      ...base,
      id: `${table}-article`,
      name: "Article",
      layout: "article",
      card: {
        title,
        subtitle,
        description,
        badges,
        metadata: metadata.slice(0, 3),
        fields: [],
      },
    },
    {
      ...base,
      id: `${table}-profile`,
      name: "Profile",
      layout: "profile",
      card: {
        title,
        subtitle,
        description,
        image,
        badges,
        metadata: metadata.slice(0, 4),
        fields: [],
      },
    },
    {
      ...base,
      id: `${table}-compact`,
      name: "Compact",
      layout: "compact",
      card: {
        title,
        subtitle,
        badges,
        metadata: metadata.slice(0, 2),
        fields: [],
      },
    },
    {
      ...base,
      id: `${table}-metric`,
      name: "Metric",
      layout: "metric",
      card: {
        title: subtitle ?? title,
        subtitle: title,
        value,
        valueLabel: value ? fieldLabel(value.column) : undefined,
        footnote: title,
        badges,
        metadata: [],
        fields: [],
      },
    },
    {
      ...base,
      id: `${table}-key-value`,
      name: "Key-value",
      layout: "keyValue",
      card: {
        title,
        subtitle,
        badges,
        metadata: [],
        fields: scalarFields,
      },
    },
  ];

  const defaultLayout = guessLayout(columns);
  const defaultTemplateId =
    templates.find((template) => template.layout === defaultLayout)?.id ??
    templates[0]?.id ??
    null;

  return { version: 1, defaultTemplateId, templates };
}

export function loadTemplateStore(key: string): CardTemplateStore | null {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const parsed: unknown = JSON.parse(raw);
    if (
      !parsed ||
      typeof parsed !== "object" ||
      (parsed as CardTemplateStore).version !== 1 ||
      !Array.isArray((parsed as CardTemplateStore).templates)
    ) {
      return null;
    }
    return parsed as CardTemplateStore;
  } catch {
    return null;
  }
}

export function saveTemplateStore(key: string, store: CardTemplateStore): void {
  try {
    localStorage.setItem(key, JSON.stringify(store));
  } catch {
    // Templates are a convenience; ignore storage failure.
  }
}

export function findColumnByName(
  columns: ColumnInfo[],
  name: string,
): ColumnInfo | undefined {
  return columns.find((column) => column.name === name);
}

export function recordObject(
  columns: ColumnInfo[],
  row: import("@/types").Row,
): Record<string, import("@/types").CellValue> {
  const values: Record<string, import("@/types").CellValue> = {};
  columns.forEach((column, index) => {
    values[column.name] = row.cells[index] ?? { kind: "null" };
  });
  return values;
}

export function missingFieldNames(columns: ColumnInfo[], fields: FieldRef[]): string[] {
  return fields
    .map((field) => field.column)
    .filter((name) => !columns.some((column) => column.name === name));
}

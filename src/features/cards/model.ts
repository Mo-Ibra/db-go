export type CardLayout = "article" | "profile" | "compact" | "metric" | "keyValue";

export type FieldFormat =
  | "text"
  | "number"
  | "currency"
  | "date"
  | "boolean"
  | "url"
  | "email";

export type FieldRef = {
  column: string;
  label?: string;
  format?: FieldFormat;
};

export type CardConfig = {
  title?: FieldRef;
  subtitle?: FieldRef;
  description?: FieldRef;
  image?: FieldRef;
  value?: FieldRef;
  valueLabel?: string;
  comparison?: FieldRef;
  footnote?: FieldRef;
  badges: FieldRef[];
  metadata: FieldRef[];
  fields: FieldRef[];
};

export type DetailGroup = {
  id: string;
  title: string;
  fields: FieldRef[];
};

export type CardTemplate = {
  id: string;
  name: string;
  table: string;
  layout: CardLayout;
  card: CardConfig;
  detail: {
    groups: DetailGroup[];
    hidden: string[];
  };
  columns: 1 | 2 | 3 | 4;
  pageSize: number;
  searchFields: string[];
  sort: {
    column: string;
    direction: "asc" | "desc";
  } | null;
};

export type CardTemplateStore = {
  version: 1;
  defaultTemplateId: string | null;
  templates: CardTemplate[];
};

export type CardRecord = {
  row: import("@/types").Row;
  index: number;
  values: Record<string, import("@/types").CellValue>;
};

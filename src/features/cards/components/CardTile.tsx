import type { ColumnInfo, Row } from "@/types";
import type { CardTemplate, FieldRef } from "@/features/cards/model";
import { formatCardValue, fieldText, cardInitial } from "@/features/cards/renderers";
import { recordObject } from "@/features/cards/templates";
import { Badge } from "@/components/ui/Badge";
import { cn } from "@/lib/cn";

type CardTileProps = {
  template: CardTemplate;
  columns: ColumnInfo[];
  row: Row;
  onClick: () => void;
};

function valueOf(
  values: ReturnType<typeof recordObject>,
  field: FieldRef | undefined,
): string {
  if (!field) return "";
  return formatCardValue(values[field.column], field);
}

function CardBadges({
  fields,
  values,
}: {
  fields: FieldRef[];
  values: ReturnType<typeof recordObject>;
}) {
  return (
    <>
      {fields.slice(0, 3).map((field) => {
        const value = valueOf(values, field);
        if (!value || value === "—") return null;
        return <Badge key={field.column}>{value}</Badge>;
      })}
    </>
  );
}

function CardMetadata({
  fields,
  values,
}: {
  fields: FieldRef[];
  values: ReturnType<typeof recordObject>;
}) {
  return (
    <div className="flex flex-wrap gap-x-3 gap-y-1 text-[10.5px] text-muted-2">
      {fields.slice(0, 4).map((field) => {
        const value = valueOf(values, field);
        if (!value || value === "—") return null;
        return (
          <span key={field.column} className="inline-flex items-center gap-1">
            <span>{fieldText(field)}</span>
            <span className="text-muted">{value}</span>
          </span>
        );
      })}
    </div>
  );
}

export function CardTile({ template, columns, row, onClick }: CardTileProps) {
  const values = recordObject(columns, row);
  const card = template.card;
  const title = valueOf(values, card.title) || "Untitled";
  const subtitle = valueOf(values, card.subtitle);
  const description = valueOf(values, card.description);
  const footnote = valueOf(values, card.footnote);

  const shell = cn(
    "group flex h-full cursor-pointer flex-col gap-2.5 rounded-xl border border-line bg-panel/70 p-4 text-left shadow-sm transition-all hover:-translate-y-0.5 hover:border-accent/35 hover:bg-panel-2 hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40",
  );

  const content = (() => {
    switch (template.layout) {
      case "compact":
        return (
          <>
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <h3 className="truncate font-mono text-[13px] font-semibold text-fg">
                  {title}
                </h3>
                {subtitle ? (
                  <p className="mt-0.5 truncate text-[11.5px] text-muted">
                    {subtitle}
                  </p>
                ) : null}
              </div>
              <div className="flex flex-none gap-1">
                <CardBadges fields={card.badges} values={values} />
              </div>
            </div>
            <CardMetadata fields={card.metadata} values={values} />
          </>
        );

      case "profile":
        return (
          <>
            <div className="flex items-start gap-3">
              <div className="grid h-11 w-11 flex-none place-items-center rounded-full border border-accent/25 bg-accent/10 text-sm font-bold text-accent">
                {cardInitial(title)}
              </div>
              <div className="min-w-0 flex-1">
                <h3 className="truncate text-[14px] font-semibold text-fg">{title}</h3>
                {subtitle ? (
                  <p className="mt-0.5 truncate text-[11.5px] text-muted">
                    {subtitle}
                  </p>
                ) : null}
                <div className="mt-1.5 flex flex-wrap gap-1">
                  <CardBadges fields={card.badges} values={values} />
                </div>
              </div>
            </div>
            {description ? (
              <p className="line-clamp-3 text-[12px] leading-relaxed text-muted">
                {description}
              </p>
            ) : null}
            <CardMetadata fields={card.metadata} values={values} />
          </>
        );

      case "metric":
        return (
          <>
            <div className="flex items-start justify-between gap-2">
              <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted-2">
                {card.valueLabel ?? fieldText(card.value ?? { column: "value" })}
              </span>
              <CardBadges fields={card.badges} values={values} />
            </div>
            <div className="text-[28px] font-semibold tracking-tight text-fg">
              {valueOf(values, card.value) || "—"}
            </div>
            <div className="mt-auto">
              <p className="truncate text-[11.5px] text-muted">{footnote || title}</p>
              {subtitle ? (
                <p className="truncate text-[10.5px] text-muted-2">{subtitle}</p>
              ) : null}
            </div>
          </>
        );

      case "keyValue":
        return (
          <>
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <h3 className="truncate font-mono text-[13px] font-semibold text-fg">
                  {title}
                </h3>
                {subtitle ? (
                  <p className="mt-0.5 truncate text-[11px] text-muted-2">{subtitle}</p>
                ) : null}
              </div>
              <CardBadges fields={card.badges} values={values} />
            </div>
            <dl className="grid grid-cols-2 gap-x-4 gap-y-1.5 border-t border-line/70 pt-2">
              {card.fields.slice(0, 6).map((field) => {
                const value = valueOf(values, field);
                if (!value || value === "—") return null;
                return (
                  <div key={field.column} className="min-w-0">
                    <dt className="truncate text-[9.5px] uppercase tracking-[0.08em] text-muted-2">
                      {fieldText(field)}
                    </dt>
                    <dd className="truncate font-mono text-[11px] text-fg">{value}</dd>
                  </div>
                );
              })}
            </dl>
          </>
        );

      case "article":
      default:
        return (
          <>
            <div className="flex flex-wrap items-center gap-1">
              <CardBadges fields={card.badges} values={values} />
            </div>
            <div className="min-w-0">
              <h3 className="line-clamp-2 text-[14px] font-semibold leading-snug text-fg">
                {title}
              </h3>
              {subtitle ? (
                <p className="mt-1 truncate text-[11.5px] text-muted">{subtitle}</p>
              ) : null}
            </div>
            {description ? (
              <p className="line-clamp-3 text-[12px] leading-relaxed text-muted">
                {description}
              </p>
            ) : null}
            <div className="mt-auto border-t border-line/70 pt-2">
              <CardMetadata fields={card.metadata} values={values} />
            </div>
          </>
        );
    }
  })();

  return (
    <button type="button" className={shell} onClick={onClick}>
      {content}
    </button>
  );
}

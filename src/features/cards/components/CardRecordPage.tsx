import { useEffect, useMemo, useState } from "react";
import {
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  Copy,
  ExternalLink,
  Table2,
} from "lucide-react";
import type { ColumnInfo, Row } from "@/types";
import type { CardTemplate, FieldRef } from "@/features/cards/model";
import {
  cardInitial,
  fieldText,
  formatCardValue,
  isLinkField,
  linkHref,
} from "@/features/cards/renderers";
import { recordObject } from "@/features/cards/templates";
import { cellToValue } from "@/lib/cellValues";
import { copyText } from "@/lib/clipboard";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";

type CardRecordPageProps = {
  template: CardTemplate;
  columns: ColumnInfo[];
  row: Row;
  index: number;
  total: number;
  onBack: () => void;
  onPrevious: () => void;
  onNext: () => void;
  canPrevious: boolean;
  canNext: boolean;
  onOpenData: () => void;
  embedded?: boolean;
};

function FieldValue({
  field,
  value,
}: {
  field: FieldRef;
  value: ReturnType<typeof recordObject>[string];
}) {
  const text = formatCardValue(value, field);
  if (isLinkField(field) && text !== "—") {
    return (
      <a
        href={linkHref(field, text)}
        target="_blank"
        rel="noreferrer"
        className="inline-flex max-w-full items-center gap-1 break-all text-accent hover:underline"
      >
        <span className="break-all">{text}</span>
        <ExternalLink size={12} className="flex-none" />
      </a>
    );
  }
  return <span className="break-words">{text}</span>;
}

export function CardRecordPage({
  template,
  columns,
  row,
  index,
  total,
  onBack,
  onPrevious,
  onNext,
  canPrevious,
  canNext,
  onOpenData,
  embedded = false,
}: CardRecordPageProps) {
  const values = useMemo(() => recordObject(columns, row), [columns, row]);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (embedded) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onBack();
      if (event.key === "ArrowLeft" && canPrevious) onPrevious();
      if (event.key === "ArrowRight" && canNext) onNext();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [canNext, canPrevious, embedded, onBack, onNext, onPrevious]);

  const titleField = template.card.title;
  const subtitleField = template.card.subtitle;
  const descriptionField = template.card.description;
  const title = titleField
    ? formatCardValue(values[titleField.column], titleField)
    : "Record";
  const subtitle = subtitleField
    ? formatCardValue(values[subtitleField.column], subtitleField)
    : "";
  const description = descriptionField
    ? formatCardValue(values[descriptionField.column], descriptionField)
    : "";

  const copyJson = async () => {
    const record: Record<string, unknown> = {};
    columns.forEach((column) => {
      record[column.name] = cellToValue(values[column.name]);
    });
    await copyText(JSON.stringify(record, null, 2));
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  const groups = template.detail.groups
    .map((group) => ({
      ...group,
      fields: group.fields.filter(
        (field) => !template.detail.hidden.includes(field.column),
      ),
    }))
    .filter((group) => group.fields.length > 0);

  return (
    <main
      className={
        embedded
          ? "flex h-full flex-col overflow-auto bg-ink"
          : "fixed inset-0 z-40 flex flex-col overflow-auto bg-ink"
      }
    >
      {!embedded ? (
      <div className="sticky top-0 z-20 flex flex-none items-center justify-between gap-3 border-b border-line bg-panel/90 px-4 py-2 backdrop-blur-xl">
        <div className="flex min-w-0 items-center gap-2">
          <Button variant="ghost" size="sm" icon={ArrowLeft} onClick={onBack}>
            Cards
          </Button>
          <span className="text-muted-2">/</span>
          <span className="truncate font-mono text-[11.5px] text-muted">
            {template.table}
          </span>
          <span className="text-muted-2">/</span>
          <span className="truncate text-[11.5px] text-fg">{title}</span>
        </div>
        <div className="flex flex-none items-center gap-1.5">
          <IconButton
            icon={ChevronLeft}
            label="Previous record"
            size="sm"
            onClick={onPrevious}
            disabled={!canPrevious}
          />
          <span className="min-w-[70px] text-center text-[10.5px] tabular-nums text-muted-2">
            {index + 1} / {total}
          </span>
          <IconButton
            icon={ChevronRight}
            label="Next record"
            size="sm"
            onClick={onNext}
            disabled={!canNext}
          />
          <Button
            variant="ghost"
            size="sm"
            icon={Table2}
            onClick={onOpenData}
            title="Open this table in the data grid"
          >
            Data
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={Copy}
            onClick={() => void copyJson()}
          >
            {copied ? "Copied" : "Copy JSON"}
          </Button>
        </div>
      </div>
      ) : null}

      <div className="mx-auto w-full max-w-[1080px] px-6 py-10 pb-20">
        <header className="border-b border-line pb-8">
          <div className="flex flex-wrap items-center gap-2">
            {template.card.badges.slice(0, 5).map((field) => {
              const value = formatCardValue(values[field.column], field);
              if (!value || value === "—") return null;
              return <Badge key={field.column}>{value}</Badge>;
            })}
            <Badge tone="accent">{template.name}</Badge>
          </div>

          <div className="mt-5 flex items-start gap-4">
            {template.layout === "profile" ? (
              <div className="grid h-16 w-16 flex-none place-items-center rounded-2xl border border-accent/25 bg-accent/10 text-xl font-bold text-accent">
                {cardInitial(title)}
              </div>
            ) : (
              <div className="mt-1 h-10 w-1 flex-none rounded-full bg-accent" />
            )}
            <div className="min-w-0">
              <h1 className="break-words text-[34px] font-semibold leading-tight tracking-tight text-fg">
                {title}
              </h1>
              {subtitle ? (
                <p className="mt-2 text-[15px] text-muted">{subtitle}</p>
              ) : null}
            </div>
          </div>

          {description ? (
            <p className="mt-6 max-w-[820px] text-[15px] leading-7 text-muted">
              {description}
            </p>
          ) : null}

          {template.card.metadata.length > 0 ? (
            <div className="mt-7 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              {template.card.metadata.map((field) => {
                const value = formatCardValue(values[field.column], field);
                if (!value || value === "—") return null;
                return (
                  <div
                    key={field.column}
                    className="rounded-lg border border-line bg-panel/60 px-3.5 py-2.5"
                  >
                    <div className="text-[9.5px] font-bold uppercase tracking-[0.12em] text-muted-2">
                      {fieldText(field)}
                    </div>
                    <div className="mt-0.5 truncate font-mono text-[12.5px] text-fg">
                      {value}
                    </div>
                  </div>
                );
              })}
            </div>
          ) : null}
        </header>

        <div className="mt-8 space-y-8">
          {groups.map((group) => (
            <section key={group.id}>
              <h2 className="mb-3 text-[11px] font-bold uppercase tracking-[0.14em] text-muted-2">
                {group.title}
              </h2>
              <dl className="grid overflow-hidden rounded-xl border border-line bg-panel/50 sm:grid-cols-2">
                {group.fields.map((field) => (
                  <div
                    key={`${group.id}-${field.column}`}
                    className="grid grid-cols-[130px_minmax(0,1fr)] gap-3 border-b border-line/60 px-4 py-3 text-[12.5px] last:border-b-0 sm:border-r sm:[&:nth-child(2n)]:border-r-0"
                  >
                    <dt className="text-muted-2">{fieldText(field)}</dt>
                    <dd className="min-w-0 text-fg">
                      <FieldValue field={field} value={values[field.column]} />
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
        </div>

        <footer className="mt-12 flex items-center justify-between gap-3 border-t border-line pt-5">
          <div className="text-[10.5px] text-muted-2">
            Record {index + 1} of {total} · {template.table}
          </div>
          {!embedded ? (
            <div className="flex items-center gap-2">
              <Button
                variant="secondary"
                size="sm"
                icon={ChevronLeft}
                onClick={onPrevious}
                disabled={!canPrevious}
              >
                Previous
              </Button>
              <Button
                variant="secondary"
                size="sm"
                iconRight={ChevronRight}
                onClick={onNext}
                disabled={!canNext}
              >
                Next
              </Button>
            </div>
          ) : null}
        </footer>
      </div>
    </main>
  );
}

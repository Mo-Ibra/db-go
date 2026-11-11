import { useEffect, useMemo, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Copy,
  ExternalLink,
  Table2,
  X,
} from "lucide-react";
import type { ColumnInfo, Row } from "@/types";
import type { CardTemplate, FieldRef } from "@/features/cards/model";
import {
  fieldText,
  formatCardValue,
  isLinkField,
  linkHref,
} from "@/features/cards/renderers";
import { recordObject } from "@/features/cards/templates";
import { cellToValue } from "@/lib/cellValues";
import { copyText } from "@/lib/clipboard";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Badge } from "@/components/ui/Badge";

type CardDetailDrawerProps = {
  template: CardTemplate;
  columns: ColumnInfo[];
  row: Row;
  index: number;
  onClose: () => void;
  onPrevious: () => void;
  onNext: () => void;
  canPrevious: boolean;
  canNext: boolean;
  onOpenData: () => void;
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
        className="inline-flex items-center gap-1 break-all text-accent hover:underline"
      >
        {text}
        <ExternalLink size={11} />
      </a>
    );
  }
  return <span className="break-words">{text}</span>;
}

export function CardDetailDrawer({
  template,
  columns,
  row,
  index,
  onClose,
  onPrevious,
  onNext,
  canPrevious,
  canNext,
  onOpenData,
}: CardDetailDrawerProps) {
  const values = useMemo(() => recordObject(columns, row), [columns, row]);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const titleField = template.card.title;
  const title = titleField ? formatCardValue(values[titleField.column], titleField) : "Record";

  const copyJson = async () => {
    const record: Record<string, unknown> = {};
    columns.forEach((column) => {
      record[column.name] = cellToValue(values[column.name]);
    });
    await copyText(JSON.stringify(record, null, 2));
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/55 backdrop-blur-sm">
      <button
        type="button"
        aria-label="Close detail"
        className="absolute inset-0 cursor-default"
        onClick={onClose}
      />
      <aside
        role="dialog"
        aria-modal="true"
        aria-label={`Record ${index + 1}`}
        className="relative z-10 flex h-full w-full max-w-[560px] flex-col border-l border-line bg-panel shadow-2xl"
      >
        <header className="flex flex-none items-start justify-between gap-3 border-b border-line px-5 py-4">
          <div className="min-w-0">
            <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted-2">
              {template.name}
            </span>
            <h2 className="mt-0.5 break-words text-lg font-semibold tracking-tight">
              {title}
            </h2>
            <div className="mt-2 flex flex-wrap gap-1">
              {template.card.badges.slice(0, 4).map((field) => {
                const value = formatCardValue(values[field.column], field);
                if (!value || value === "—") return null;
                return <Badge key={field.column}>{value}</Badge>;
              })}
            </div>
          </div>
          <IconButton icon={X} label="Close detail" onClick={onClose} />
        </header>

        <div className="min-h-0 flex-1 overflow-auto px-5 py-4">
          <div className="space-y-5">
            {template.detail.groups.map((group) => (
              <section key={group.id}>
                <h3 className="mb-2 text-[10px] font-bold uppercase tracking-[0.14em] text-muted-2">
                  {group.title}
                </h3>
                <dl className="divide-y divide-line/60 rounded-lg border border-line bg-panel-2/50">
                  {group.fields
                    .filter((field) => !template.detail.hidden.includes(field.column))
                    .map((field) => (
                    <div
                      key={`${group.id}-${field.column}`}
                      className="grid grid-cols-[130px_minmax(0,1fr)] gap-3 px-3 py-2 text-[12px]"
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
        </div>

        <footer className="flex flex-none items-center justify-between gap-2 border-t border-line px-4 py-3">
          <div className="flex items-center gap-1.5">
            <IconButton
              icon={ChevronLeft}
              label="Previous record"
              size="sm"
              onClick={onPrevious}
              disabled={!canPrevious}
            />
            <span className="text-[10.5px] tabular-nums text-muted-2">
              Record {index + 1}
            </span>
            <IconButton
              icon={ChevronRight}
              label="Next record"
              size="sm"
              onClick={onNext}
              disabled={!canNext}
            />
          </div>
          <div className="flex items-center gap-1.5">
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
        </footer>
      </aside>
    </div>
  );
}

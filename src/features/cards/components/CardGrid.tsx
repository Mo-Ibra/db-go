import type { ColumnInfo, Row } from "@/types";
import type { CardTemplate } from "@/features/cards/model";
import { CardTile } from "./CardTile";

type CardGridProps = {
  template: CardTemplate;
  columns: ColumnInfo[];
  rows: Row[];
  loading: boolean;
  error: string | null;
  onSelect: (row: Row, index: number) => void;
  onRetry: () => void;
};

export function CardGrid({
  template,
  columns,
  rows,
  loading,
  error,
  onSelect,
  onRetry,
}: CardGridProps) {
  if (error) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-3 p-10 text-center">
        <p className="font-mono text-[12px] text-[#ff7b72]">{error}</p>
        <button type="button" className="btn btn-neutral" onClick={onRetry}>
          Try again
        </button>
      </div>
    );
  }

  if (loading && rows.length === 0) {
    return (
      <div className="grid flex-1 place-items-center text-xs text-muted">
        Loading cards...
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2 p-10 text-center">
        <div className="grid h-[42px] w-[42px] place-items-center rounded-xl border border-line bg-panel-3 text-muted">
          ▦
        </div>
        <h3 className="text-base tracking-tight">No records</h3>
        <p className="text-[12px] text-muted">
          Try a different search or template.
        </p>
      </div>
    );
  }

  return (
    <div className="min-h-0 flex-1 overflow-auto p-4">
      <div
        className="grid gap-3"
        style={{
          gridTemplateColumns: `repeat(${template.columns}, minmax(0, 1fr))`,
        }}
      >
        {rows.map((row, index) => (
          <CardTile
            key={row.rowid ?? `row-${index}`}
            template={template}
            columns={columns}
            row={row}
            onClick={() => onSelect(row, index)}
          />
        ))}
      </div>
    </div>
  );
}

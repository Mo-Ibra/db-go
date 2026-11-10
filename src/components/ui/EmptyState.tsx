import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  compact = false,
}: {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: ReactNode;
  compact?: boolean;
}) {
  return (
    <div
      className={
        compact
          ? "flex flex-col items-center justify-center gap-2 p-6 text-center"
          : "flex flex-1 flex-col items-center justify-center gap-2.5 p-10 text-center"
      }
    >
      <div className="grid h-[42px] w-[42px] place-items-center rounded-[11px] border border-line bg-panel-3 text-muted">
        <Icon size={19} strokeWidth={1.8} />
      </div>
      <h2 className="mt-1 text-base tracking-tight">{title}</h2>
      {description ? (
        <p className="max-w-[420px] text-[13px] leading-relaxed text-muted">
          {description}
        </p>
      ) : null}
      {action ? <div className="mt-1">{action}</div> : null}
    </div>
  );
}

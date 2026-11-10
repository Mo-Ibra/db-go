import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

export function SectionHeader({
  icon: Icon,
  title,
  subtitle,
  actions,
}: {
  icon?: LucideIcon;
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3">
      <div className="flex min-w-0 items-center gap-2">
        {Icon ? <Icon size={15} className="text-muted-2" strokeWidth={2} /> : null}
        <span className="truncate text-[10.5px] font-bold uppercase tracking-[0.14em] text-muted">
          {title}
        </span>
        {subtitle}
      </div>
      {actions ? <div className="flex flex-none items-center gap-2">{actions}</div> : null}
    </div>
  );
}

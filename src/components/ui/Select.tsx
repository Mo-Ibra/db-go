import type { SelectHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export function Select({
  className,
  ...props
}: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      className={cn(
        "h-7 rounded-md border border-line bg-panel-2 px-1.5 text-xs text-fg outline-none focus:border-accent disabled:opacity-40",
        className,
      )}
      {...props}
    />
  );
}

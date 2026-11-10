import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

type BadgeTone = "neutral" | "subtle" | "accent" | "danger";

const tones: Record<BadgeTone, string> = {
  neutral:
    "border-line bg-panel-3 text-muted",
  subtle:
    "border-line/70 bg-transparent text-muted",
  accent:
    "border-accent/30 bg-accent/10 text-[#b8fff4]",
  danger:
    "border-danger/30 bg-danger/10 text-[#fca5a5]",
};

export function Badge({
  children,
  tone = "subtle",
  className,
}: {
  children: ReactNode;
  tone?: BadgeTone;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "rounded-full border px-1.5 py-0.5 text-[10.5px] tabular-nums",
        tones[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}

import type { ButtonHTMLAttributes } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/cn";

type IconButtonVariant = "ghost" | "secondary" | "danger";
type IconButtonSize = "sm" | "md";

const variants: Record<IconButtonVariant, string> = {
  ghost:
    "border-transparent bg-transparent text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg",
  secondary:
    "border-line-strong bg-panel-3/70 text-fg hover:border-[#3a4a5a] hover:bg-panel-3",
  danger:
    "border-danger/35 bg-danger/10 text-[#f85149] hover:border-danger/60 hover:bg-danger/15",
};

const sizes: Record<IconButtonSize, string> = {
  sm: "h-6 w-6",
  md: "h-[30px] w-[30px]",
};

export type IconButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  icon: LucideIcon;
  label: string;
  variant?: IconButtonVariant;
  size?: IconButtonSize;
};

export function IconButton({
  icon: Icon,
  label,
  variant = "ghost",
  size = "md",
  className,
  type = "button",
  ...props
}: IconButtonProps) {
  return (
    <button
      type={type}
      aria-label={label}
      title={label}
      className={cn(
        "inline-flex items-center justify-center rounded-md border shadow-sm transition-all active:translate-y-px focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 disabled:cursor-default disabled:opacity-40 disabled:shadow-none",
        variants[variant],
        sizes[size],
        className,
      )}
      {...props}
    >
      <Icon size={size === "sm" ? 13 : 15} strokeWidth={2} />
    </button>
  );
}

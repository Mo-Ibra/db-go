import type { ButtonHTMLAttributes, ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/cn";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md" | "lg";

const variants: Record<ButtonVariant, string> = {
  primary:
    "border-[#2ea043] bg-[#238636] font-bold text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.08)] hover:border-[#3fb950] hover:bg-[#2ea043]",
  secondary:
    "border-line-strong bg-panel-3/70 text-fg hover:border-[#3a4a5a] hover:bg-panel-3",
  ghost:
    "border-transparent bg-transparent text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg",
  danger:
    "border-danger/35 bg-danger/10 text-[#f85149] hover:border-danger/60 hover:bg-danger/15",
};

const sizes: Record<ButtonSize, string> = {
  sm: "h-7 px-2.5 text-[11.5px]",
  md: "h-[30px] px-3 text-[12.5px]",
  lg: "h-[38px] px-5 text-[13px]",
};

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: LucideIcon;
  iconRight?: LucideIcon;
  children?: ReactNode;
};

export function Button({
  variant = "secondary",
  size = "md",
  icon: Icon,
  iconRight: IconRight,
  className,
  children,
  type = "button",
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(
        "inline-flex items-center justify-center gap-1.5 rounded-[7px] border font-medium shadow-sm transition-all active:translate-y-px focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 disabled:cursor-default disabled:opacity-40 disabled:shadow-none",
        variants[variant],
        sizes[size],
        className,
      )}
      {...props}
    >
      {Icon ? <Icon size={size === "sm" ? 13 : 15} strokeWidth={2} /> : null}
      {children}
      {IconRight ? <IconRight size={size === "sm" ? 13 : 15} strokeWidth={2} /> : null}
    </button>
  );
}

import type { ButtonHTMLAttributes, ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/cn";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md" | "lg";

const variants: Record<ButtonVariant, string> = {
  primary:
    "border-[#14b8a6] bg-linear-to-b from-accent to-[#14b8a6] font-bold text-[#03110f] hover:brightness-110",
  secondary:
    "border-line-strong bg-panel-2 text-fg hover:border-[#3a4a5a] hover:bg-panel-3",
  ghost:
    "border-transparent bg-transparent text-fg hover:border-line-strong hover:bg-panel-2",
  danger:
    "border-danger/35 bg-danger/10 text-[#fca5a5] hover:border-danger/60 hover:bg-danger/15",
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
        "inline-flex items-center justify-center gap-1.5 rounded-[7px] border font-medium transition-colors disabled:cursor-default disabled:opacity-40",
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

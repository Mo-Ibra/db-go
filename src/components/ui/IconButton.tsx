import type { ButtonHTMLAttributes } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/cn";

type IconButtonVariant = "ghost" | "secondary" | "danger";
type IconButtonSize = "sm" | "md";

const variants: Record<IconButtonVariant, string> = {
  ghost:
    "border-transparent bg-transparent text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg",
  secondary:
    "border-line-strong bg-panel-2 text-fg hover:border-[#3a4a5a] hover:bg-panel-3",
  danger:
    "border-danger/35 bg-danger/10 text-[#fca5a5] hover:border-danger/60 hover:bg-danger/15",
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
        "inline-flex items-center justify-center rounded-md border transition-colors disabled:cursor-default disabled:opacity-40",
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

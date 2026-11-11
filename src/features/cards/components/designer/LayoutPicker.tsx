import { AlignLeft, Image, List, Rows3, Type } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { CardLayout } from "@/features/cards/model";
import { cn } from "@/lib/cn";
import { DesignerSection } from "./shared";

const LAYOUTS: {
  value: CardLayout;
  label: string;
  description: string;
  icon: LucideIcon;
}[] = [
  {
    value: "article",
    label: "Article",
    description: "Title, summary, labels",
    icon: AlignLeft,
  },
  {
    value: "profile",
    label: "Profile",
    description: "Avatar, identity, contact",
    icon: Image,
  },
  {
    value: "compact",
    label: "Compact",
    description: "Dense one-line records",
    icon: Rows3,
  },
  {
    value: "metric",
    label: "Metric",
    description: "One big number",
    icon: Type,
  },
  {
    value: "keyValue",
    label: "Key / Value",
    description: "Label and value grid",
    icon: List,
  },
];

export function LayoutPicker({
  layout,
  onChange,
}: {
  layout: CardLayout;
  onChange: (layout: CardLayout) => void;
}) {
  return (
    <DesignerSection
      title="Choose a card style"
      description="Pick the layout that best matches how this data should be read."
    >
      <div className="grid grid-cols-2 gap-2">
        {LAYOUTS.map((item) => {
          const Icon = item.icon;
          const active = layout === item.value;
          return (
            <button
              key={item.value}
              type="button"
              onClick={() => onChange(item.value)}
              className={cn(
                "flex items-start gap-2 rounded-lg border p-2.5 text-left transition-colors",
                active
                  ? "border-accent/40 bg-accent/10"
                  : "border-line bg-ink/40 hover:border-line-strong hover:bg-panel-2",
              )}
            >
              <Icon size={15} className={active ? "text-accent" : "text-muted-2"} />
              <span>
                <span className="block text-[11.5px] font-semibold text-fg">
                  {item.label}
                </span>
                <span className="mt-0.5 block text-[10px] leading-relaxed text-muted-2">
                  {item.description}
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </DesignerSection>
  );
}

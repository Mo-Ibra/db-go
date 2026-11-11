import { useEffect, useState } from "react";
import {
  Columns3,
  Copy,
  Eye,
  LayoutGrid,
  List,
  Plus,
  Trash2,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { CardTemplate } from "@/features/cards/model";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { cn } from "@/lib/cn";

export type DesignerTab = "card" | "details" | "layout";

const TABS: { id: DesignerTab; label: string; icon: LucideIcon }[] = [
  { id: "card", label: "Card", icon: LayoutGrid },
  { id: "details", label: "Detail page", icon: List },
  { id: "layout", label: "Layout", icon: Columns3 },
];

export function CardDesignerHeader({
  template,
  templateCount,
  tab,
  onTabChange,
  onRename,
  onCreate,
  onDuplicate,
  onDelete,
  onPreviewRecord,
  canPreviewRecord,
}: {
  template: CardTemplate;
  templateCount: number;
  tab: DesignerTab;
  onTabChange: (tab: DesignerTab) => void;
  onRename: (name: string) => void;
  onCreate: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onPreviewRecord: () => void;
  canPreviewRecord: boolean;
}) {
  const [name, setName] = useState(template.name);

  useEffect(() => {
    setName(template.name);
  }, [template.id, template.name]);

  return (
    <div className="border-b border-line p-4">
      <div className="flex items-center justify-between gap-2">
        <div className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted-2">
          Card designer
        </div>
        <span className="text-[10px] text-muted-2">Saves automatically</span>
      </div>
      <input
        className="field-input mt-2 w-full text-[13px] font-semibold"
        value={name}
        onChange={(event) => setName(event.target.value)}
        onBlur={() => {
          const trimmed = name.trim();
          if (trimmed && trimmed !== template.name) onRename(trimmed);
          if (!trimmed) setName(template.name);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
        aria-label="Template name"
      />
      <div className="mt-2 flex items-center gap-1.5">
        <Button variant="secondary" size="sm" icon={Plus} onClick={onCreate}>
          New
        </Button>
        <Button variant="ghost" size="sm" icon={Copy} onClick={onDuplicate}>
          Duplicate
        </Button>
        <Button
          variant="ghost"
          size="sm"
          icon={Eye}
          onClick={onPreviewRecord}
          disabled={!canPreviewRecord}
        >
          Preview page
        </Button>
        <IconButton
          icon={Trash2}
          label="Delete template"
          variant="danger"
          size="sm"
          onClick={onDelete}
          disabled={templateCount <= 1}
        />
      </div>
      <div className="mt-3 grid grid-cols-3 gap-1 rounded-lg border border-line bg-ink p-1">
        {TABS.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              type="button"
              className={cn(
                "flex h-8 items-center justify-center gap-1.5 rounded-md text-[11px] font-semibold transition-colors",
                tab === item.id
                  ? "bg-panel-3 text-fg shadow-sm"
                  : "text-muted hover:bg-panel-2 hover:text-fg",
              )}
              onClick={() => onTabChange(item.id)}
            >
              <Icon size={13} />
              {item.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}

import type { ColumnInfo } from "@/types";
import type { CardLayout, CardTemplate } from "@/features/cards/model";
import { CardContentEditor } from "./designer/CardContentEditor";
import {
  CardDesignerHeader,
  type DesignerTab,
} from "./designer/CardDesignerHeader";
import { DetailGroupsEditor } from "./designer/DetailGroupsEditor";
import { LayoutPicker } from "./designer/LayoutPicker";
import { LayoutSettings } from "./designer/LayoutSettings";

export type { DesignerTab };

type CardDesignerProps = {
  template: CardTemplate;
  templates: CardTemplate[];
  columns: ColumnInfo[];
  onUpdate: (template: CardTemplate) => void;
  onChangeLayout: (layout: CardLayout) => void;
  onRename: (name: string) => void;
  onCreate: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onPreviewRecord: () => void;
  canPreviewRecord: boolean;
  tab: DesignerTab;
  onTabChange: (tab: DesignerTab) => void;
};

export function CardDesigner({
  template,
  templates,
  columns,
  onUpdate,
  onChangeLayout,
  onRename,
  onCreate,
  onDuplicate,
  onDelete,
  onPreviewRecord,
  canPreviewRecord,
  tab,
  onTabChange,
}: CardDesignerProps) {
  const updateCard = (patch: Partial<CardTemplate["card"]>) =>
    onUpdate({ ...template, card: { ...template.card, ...patch } });

  const updateDetailGroups = (groups: CardTemplate["detail"]["groups"]) =>
    onUpdate({ ...template, detail: { ...template.detail, groups } });

  return (
    <aside className="flex w-[390px] min-w-[340px] flex-none flex-col border-r border-line bg-panel/40">
      <CardDesignerHeader
        template={template}
        templateCount={templates.length}
        tab={tab}
        onTabChange={onTabChange}
        onRename={onRename}
        onCreate={onCreate}
        onDuplicate={onDuplicate}
        onDelete={onDelete}
        onPreviewRecord={onPreviewRecord}
        canPreviewRecord={canPreviewRecord}
      />

      <div className="min-h-0 flex-1 overflow-auto">
        {tab === "card" ? (
          <>
            <LayoutPicker layout={template.layout} onChange={onChangeLayout} />
            <CardContentEditor
              template={template}
              columns={columns}
              updateCard={updateCard}
            />
          </>
        ) : null}

        {tab === "details" ? (
          <DetailGroupsEditor
            template={template}
            columns={columns}
            updateDetailGroups={updateDetailGroups}
          />
        ) : null}

        {tab === "layout" ? (
          <LayoutSettings
            template={template}
            columns={columns}
            onUpdate={onUpdate}
          />
        ) : null}
      </div>
    </aside>
  );
}

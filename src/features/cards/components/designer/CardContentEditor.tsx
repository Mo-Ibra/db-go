import { AlignLeft, Heading, Image, Tag, Type } from "lucide-react";
import type { ColumnInfo } from "@/types";
import type { CardTemplate } from "@/features/cards/model";
import { DesignerSection, FieldChips, SlotRow } from "./shared";

export function CardContentEditor({
  template,
  columns,
  updateCard,
}: {
  template: CardTemplate;
  columns: ColumnInfo[];
  updateCard: (patch: Partial<CardTemplate["card"]>) => void;
}) {
  return (
    <>
      <DesignerSection
        title="Card content"
        description="Only these fields appear on the card. Everything else stays on the detail page."
      >
        <SlotRow
          icon={Heading}
          label="Title"
          help="The main heading people scan first."
          columns={columns}
          field={template.card.title}
          onChange={(field) => updateCard({ title: field })}
          kind="text"
        />
        <SlotRow
          icon={Type}
          label="Subtitle"
          help="Short context, such as city, category, or role."
          columns={columns}
          field={template.card.subtitle}
          onChange={(field) => updateCard({ subtitle: field })}
          kind="text"
        />
        {template.layout === "metric" ? (
          <>
            <SlotRow
              icon={Type}
              label="Metric value"
              help="The big number shown in the center of the card."
              columns={columns}
              field={template.card.value}
              onChange={(field) => updateCard({ value: field })}
              kind="numeric"
            />
            <label className="flex flex-col gap-1">
              <span className="flex items-center gap-1.5 text-[12px] font-medium text-fg">
                <Tag size={13} className="text-muted-2" />
                Metric label
              </span>
              <span className="text-[10.5px] text-muted-2">
                Describes the number, for example “Revenue” or “Employees”.
              </span>
              <input
                className="field-input mt-1"
                value={template.card.valueLabel ?? ""}
                onChange={(event) =>
                  updateCard({ valueLabel: event.target.value })
                }
                placeholder="Revenue"
              />
            </label>
            <SlotRow
              icon={AlignLeft}
              label="Footnote"
              help="Small context under the number."
              columns={columns}
              field={template.card.footnote}
              onChange={(field) => updateCard({ footnote: field })}
              kind="text"
            />
          </>
        ) : (
          <SlotRow
            icon={AlignLeft}
            label="Description"
            help="A short summary. Full text appears on the detail page."
            columns={columns}
            field={template.card.description}
            onChange={(field) => updateCard({ description: field })}
            kind="text"
          />
        )}
        {template.layout === "profile" ? (
          <SlotRow
            icon={Image}
            label="Image"
            help="Avatar or logo slot. Image rendering arrives in a later phase."
            columns={columns}
            field={template.card.image}
            onChange={(field) => updateCard({ image: field })}
            kind="blob"
          />
        ) : null}
      </DesignerSection>

      <DesignerSection
        title="Labels and facts"
        description="Badges are short labels. Metadata appears as key facts in the card footer."
      >
        {template.layout === "keyValue" ? (
          <FieldChips
            label="Key / value fields"
            help="The label/value pairs shown inside the card."
            columns={columns}
            fields={template.card.fields}
            onChange={(fields) => updateCard({ fields })}
            max={8}
            kind="text"
          />
        ) : null}
        <FieldChips
          label="Badges"
          help="Small pills such as status, sector, type, or category."
          columns={columns}
          fields={template.card.badges}
          onChange={(fields) => updateCard({ badges: fields })}
          max={4}
          kind="text"
        />
        <FieldChips
          label="Metadata"
          help="Short facts in the card footer, such as employees or country."
          columns={columns}
          fields={template.card.metadata}
          onChange={(fields) => updateCard({ metadata: fields })}
          max={4}
          kind="text"
        />
      </DesignerSection>
    </>
  );
}

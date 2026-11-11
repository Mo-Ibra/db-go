import { useCallback, useEffect, useState } from "react";
import { Table2 } from "lucide-react";
import {
  useWorkspaceSelector,
  useWorkspaceTableSlice,
} from "@/app/providers/WorkspaceContext";
import { useTableColumns } from "@/features/cards/hooks/useTableColumns";
import { useCardTemplates } from "@/features/cards/hooks/useCardTemplates";
import { useCardPage } from "@/features/cards/hooks/useCardPage";
import { CardToolbar } from "@/features/cards/components/CardToolbar";
import { CardGrid } from "@/features/cards/components/CardGrid";
import { CardRecordPage } from "@/features/cards/components/CardRecordPage";
import { CardDesigner } from "@/features/cards/components/CardDesigner";
import type { DesignerTab } from "@/features/cards/components/CardDesigner";
import { CardTile } from "@/features/cards/components/CardTile";
import { EmptyState } from "@/components/ui/EmptyState";
import type { SortSpec } from "@/types";

export function CardsScreen() {
  const db = useWorkspaceSelector((workspace) => workspace.db);
  const setView = useWorkspaceSelector((workspace) => workspace.setView);
  const { selected, loadTable } = useWorkspaceTableSlice();
  const { columns, loading: columnsLoading, error: columnsError } = useTableColumns(selected);
  const {
    templates,
    activeTemplate,
    activeTemplateId,
    selectTemplate,
    updateTemplate,
    renameTemplate,
    changeLayout,
    createTemplate,
    duplicateTemplate,
    deleteTemplate,
  } = useCardTemplates({
    dbPath: db?.path ?? null,
    table: selected,
    columns,
  });

  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(12);
  const [searchInput, setSearchInput] = useState("");
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<SortSpec | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [designing, setDesigning] = useState(false);
  const [designerTab, setDesignerTab] = useState<DesignerTab>("card");

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setSearch(searchInput);
      setPage(0);
    }, 220);
    return () => window.clearTimeout(timer);
  }, [searchInput]);

  useEffect(() => {
    setPage(0);
    setSelectedIndex(null);
    setSort(activeTemplate?.sort ?? null);
    if (activeTemplate) setPageSize(activeTemplate.pageSize);
  }, [selected, activeTemplateId, activeTemplate?.sort, activeTemplate?.pageSize]);

  const cardPage = useCardPage({
    table: selected,
    columns,
    template: activeTemplate,
    page,
    pageSize,
    search,
    sort,
  });

  useEffect(() => {
    if (selectedIndex !== null && selectedIndex >= cardPage.rows.length) {
      setSelectedIndex(null);
    }
  }, [cardPage.rows.length, selectedIndex]);

  const selectedRow = selectedIndex !== null ? cardPage.rows[selectedIndex] : null;
  const handlePreviewSelect = useCallback((_: unknown, index: number) => {
    setSelectedIndex(index);
  }, []);

  if (!selected) {
    return (
      <main className="flex min-h-0 min-w-0 flex-1 flex-col">
        <EmptyState
          icon={Table2}
          title="Select a table"
          description="Choose a table from the sidebar to explore it as cards."
        />
      </main>
    );
  }

  if (columnsLoading && columns.length === 0) {
    return (
      <main className="grid min-h-0 min-w-0 flex-1 place-items-center text-xs text-muted">
        Loading table fields...
      </main>
    );
  }

  if (columnsError) {
    return (
      <main className="grid min-h-0 min-w-0 flex-1 place-items-center p-6 text-center">
        <div>
          <p className="font-mono text-[12px] text-[#ff7b72]">{columnsError}</p>
        </div>
      </main>
    );
  }

  if (!activeTemplate) {
    return (
      <main className="grid min-h-0 min-w-0 flex-1 place-items-center text-xs text-muted">
        Preparing card templates...
      </main>
    );
  }

  if (!designing && selectedRow && selectedIndex !== null) {
    return (
      <CardRecordPage
        template={activeTemplate}
        columns={columns}
        row={selectedRow}
        index={selectedIndex}
        total={cardPage.rows.length}
        onBack={() => setSelectedIndex(null)}
        onPrevious={() => setSelectedIndex((index) => Math.max(0, (index ?? 0) - 1))}
        onNext={() =>
          setSelectedIndex((index) =>
            Math.min(cardPage.rows.length - 1, (index ?? 0) + 1),
          )
        }
        canPrevious={selectedIndex > 0}
        canNext={selectedIndex < cardPage.rows.length - 1}
        onOpenData={() => {
          setSelectedIndex(null);
          setView("data");
          void loadTable(selected, null);
        }}
      />
    );
  }

  return (
    <main className="flex min-h-0 min-w-0 flex-1 flex-col">
      <CardToolbar
        templates={templates}
        activeTemplateId={activeTemplateId}
        onTemplateChange={selectTemplate}
        columns={columns}
        search={searchInput}
        onSearchChange={setSearchInput}
        sort={sort}
        onSortChange={(next) => {
          setSort(next);
          setPage(0);
        }}
        page={page}
        onPageChange={setPage}
        pageSize={pageSize}
        onPageSizeChange={(size) => {
          setPageSize(size);
          setPage(0);
        }}
        count={cardPage.count}
        loading={cardPage.loading}
        onRefresh={() => void cardPage.reload()}
        designing={designing}
        onToggleDesign={() => setDesigning((current) => !current)}
      />

      {designing ? (
        <div className="flex min-h-0 flex-1">
          <CardDesigner
            template={activeTemplate}
            templates={templates}
            columns={columns}
            onUpdate={updateTemplate}
            onChangeLayout={(layout) => changeLayout(activeTemplate.id, layout)}
            onRename={(name) => renameTemplate(activeTemplate.id, name)}
            onCreate={() => createTemplate(activeTemplate.layout)}
            onDuplicate={() => duplicateTemplate(activeTemplate.id)}
            onDelete={() => deleteTemplate(activeTemplate.id)}
            onPreviewRecord={() => {
              if (cardPage.rows.length > 0) {
                setDesigning(false);
                setSelectedIndex(0);
              }
            }}
            canPreviewRecord={cardPage.rows.length > 0}
            tab={designerTab}
            onTabChange={setDesignerTab}
          />
          <div className="flex min-h-0 flex-1 flex-col bg-ink">
            <div className="flex-none border-b border-line px-6 py-4">
              <h2 className="text-[13px] font-semibold tracking-tight">
                {designerTab === "details" ? "Detail page preview" : "Live card preview"}
              </h2>
              <p className="mt-0.5 text-[11.5px] text-muted">
                {designerTab === "details"
                  ? "This is exactly how the record page appears when a card is opened."
                  : "Changes are saved automatically for this table."}
              </p>
            </div>
            {designerTab === "details" ? (
              cardPage.rows[0] ? (
                <div className="min-h-0 flex-1 overflow-hidden">
                  <CardRecordPage
                    template={activeTemplate}
                    columns={columns}
                    row={cardPage.rows[0]}
                    index={0}
                    total={cardPage.rows.length}
                    onBack={() => {}}
                    onPrevious={() => {}}
                    onNext={() => {}}
                    canPrevious={false}
                    canNext={false}
                    onOpenData={() => {}}
                    embedded
                  />
                </div>
              ) : (
                <div className="grid flex-1 place-items-center text-xs text-muted">
                  No records to preview.
                </div>
              )
            ) : (
              <div className="min-h-0 flex-1 overflow-auto p-6">
                {cardPage.rows.length === 0 ? (
                  <div className="grid h-[240px] place-items-center rounded-xl border border-dashed border-line text-xs text-muted">
                    No records to preview.
                  </div>
                ) : (
                  <div
                    className="grid max-w-[1200px] gap-3"
                    style={{
                      gridTemplateColumns: `repeat(${activeTemplate.columns}, minmax(180px, 1fr))`,
                    }}
                  >
                    {cardPage.rows
                      .slice(0, Math.max(3, activeTemplate.columns * 2))
                      .map((row, index) => (
                      <CardTile
                        key={row.rowid ?? `preview-${index}`}
                        template={activeTemplate}
                        columns={columns}
                        row={row}
                        index={index}
                        onSelect={handlePreviewSelect}
                      />
                      ))}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      ) : (
        <CardGrid
          template={activeTemplate}
          columns={columns}
          rows={cardPage.rows}
          loading={cardPage.loading}
          error={cardPage.error}
          onSelect={(_, index) => setSelectedIndex(index)}
          onRetry={() => void cardPage.reload()}
        />
      )}

    </main>
  );
}

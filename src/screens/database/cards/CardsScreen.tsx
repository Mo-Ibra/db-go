import { useEffect, useState } from "react";
import { Table2 } from "lucide-react";
import { useWorkspace } from "@/app/providers/WorkspaceContext";
import { useTableColumns } from "@/features/cards/hooks/useTableColumns";
import { useCardTemplates } from "@/features/cards/hooks/useCardTemplates";
import { useCardPage } from "@/features/cards/hooks/useCardPage";
import { CardToolbar } from "@/features/cards/components/CardToolbar";
import { CardGrid } from "@/features/cards/components/CardGrid";
import { CardDetailDrawer } from "@/features/cards/components/CardDetailDrawer";
import { CardDesigner } from "@/features/cards/components/CardDesigner";
import { CardTile } from "@/features/cards/components/CardTile";
import { EmptyState } from "@/components/ui/EmptyState";
import type { SortSpec } from "@/types";

export function CardsScreen() {
  const { db, selected, setView, loadTable } = useWorkspace();
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
          />
          <div className="min-h-0 flex-1 overflow-auto bg-ink p-6">
            <div className="mb-4">
              <h2 className="text-[13px] font-semibold tracking-tight">Live preview</h2>
              <p className="mt-0.5 text-[11.5px] text-muted">
                Changes are saved automatically for this table.
              </p>
            </div>
            {cardPage.rows.length === 0 ? (
              <div className="grid h-[240px] place-items-center rounded-xl border border-dashed border-line text-xs text-muted">
                No records to preview.
              </div>
            ) : (
              <div className="grid max-w-[1200px] grid-cols-1 gap-3 xl:grid-cols-3">
                {cardPage.rows.slice(0, 3).map((row, index) => (
                  <CardTile
                    key={row.rowid ?? `preview-${index}`}
                    template={activeTemplate}
                    columns={columns}
                    row={row}
                    onClick={() => setSelectedIndex(index)}
                  />
                ))}
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

      {!designing && selectedRow && selectedIndex !== null ? (
        <CardDetailDrawer
          template={activeTemplate}
          columns={columns}
          row={selectedRow}
          index={selectedIndex}
          onClose={() => setSelectedIndex(null)}
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
      ) : null}
    </main>
  );
}

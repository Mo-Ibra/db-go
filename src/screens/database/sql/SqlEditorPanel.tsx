import { lazy, Suspense } from "react";
import type { ExportFormat } from "@/types";
import { QueryPlanPanel } from "@/features/query/components/QueryPlanPanel";
import { Download, ListTree, Play } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Select } from "@/components/ui/Select";
import { QUERY_LIMITS } from "@/components/ui/constants";
import { useWorkspace } from "@/app/providers/WorkspaceContext";

const SqlEditor = lazy(() =>
  import("@/features/query/components/SqlEditor").then((module) => ({ default: module.SqlEditor })),
);

export function SqlEditorPanel() {
  const {
    view,
    schema,
    exportFormat,
    setExportFormat,
    exporting,
    sqlText,
    queryResult,
    queryRunning,
    queryPlan,
    setQueryPlan,
    explainRunning,
    queryLimit,
    setQueryLimit,
    queryHistory,
    runQuery,
    explainQuery,
    changeSqlText,
    exportCurrent,
  } = useWorkspace();

  return (
    <>
      <div className="flex-none border-b border-line bg-panel">
        <div className="flex h-[42px] items-center gap-2 px-3">
          <Button
            variant="primary"
            size="sm"
            icon={Play}
            onClick={() => void runQuery(sqlText)}
            disabled={queryRunning || explainRunning || !sqlText.trim()}
          >
            {queryRunning ? "Running..." : "Run"}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            icon={ListTree}
            onClick={() => void explainQuery(sqlText)}
            disabled={queryRunning || explainRunning || !sqlText.trim()}
            title="Show SQLite's query plan without running the statement"
          >
            {explainRunning ? "Explaining..." : "Explain"}
          </Button>
          <span className="whitespace-nowrap font-mono text-[10px] text-muted-2">
            Ctrl / Cmd + Enter
          </span>
          <label className="flex items-center gap-1.5 text-[10.5px] font-semibold uppercase tracking-[0.08em] text-muted">
            Max rows
            <Select
              value={queryLimit}
              onChange={(event) => setQueryLimit(Number(event.target.value))}
              aria-label="Max rows"
            >
              {QUERY_LIMITS.map((limit) => (
                <option key={limit} value={limit}>
                  {limit}
                </option>
              ))}
            </Select>
          </label>
          <select
            className="h-[26px] max-w-[260px] rounded-md border border-line bg-panel-2 px-1.5 text-[11px] text-fg outline-none disabled:opacity-45"
            value=""
            onChange={(event) => {
              const value = event.target.value;
              if (value) changeSqlText(value);
            }}
            disabled={queryHistory.length === 0}
            title="Query history"
          >
            <option value="">History...</option>
            {queryHistory.map((item, index) => (
              <option key={`${index}-${item.slice(0, 24)}`} value={item}>
                {item.replace(/\s+/g, " ").slice(0, 90)}
              </option>
            ))}
          </select>
          <span className="mx-1 h-4 w-px bg-line" />
          <Select
            value={exportFormat}
            onChange={(event) =>
              setExportFormat(event.target.value as ExportFormat)
            }
            aria-label="Export format"
          >
            <option value="csv">CSV</option>
            <option value="json">JSON</option>
          </Select>
          <Button
            variant="ghost"
            size="sm"
            icon={Download}
            onClick={() => void exportCurrent()}
            disabled={exporting || !sqlText.trim()}
            data-testid="export-query"
            title="Export this query result"
          >
            {exporting ? "Exporting..." : "Export"}
          </Button>
          {queryResult && (
            <span className="ml-auto whitespace-nowrap font-mono text-[11px] text-muted">
              {queryResult.rows.length.toLocaleString()} rows
              {queryResult.truncated ? " · truncated" : ""}
            </span>
          )}
        </div>
        <div className="h-[170px] overflow-hidden border-t border-line">
          <Suspense
            fallback={
              <div className="grid h-full place-items-center text-xs text-muted">
                Loading editor...
              </div>
            }
          >
            <SqlEditor
              value={sqlText}
              schema={schema}
              onChange={changeSqlText}
              onRun={(value) => void runQuery(value)}
            />
          </Suspense>
        </div>
      </div>

      {view === "sql" && queryPlan && (
        <QueryPlanPanel
          plan={queryPlan}
          onClose={() => setQueryPlan(null)}
        />
      )}
    </>
  );
}

import { cn } from "@/lib/cn";
import type { QueryPlan, QueryPlanStep } from "@/types";

type QueryPlanPanelProps = {
  plan: QueryPlan;
  onClose: () => void;
};

function planDepth(step: QueryPlanStep, byId: Map<number, QueryPlanStep>): number {
  let depth = 0;
  let parent = step.parent;
  const seen = new Set<number>();
  while (parent !== 0 && byId.has(parent) && !seen.has(parent) && depth < 20) {
    seen.add(parent);
    depth += 1;
    parent = byId.get(parent)?.parent ?? 0;
  }
  return depth;
}

export function QueryPlanPanel({ plan, onClose }: QueryPlanPanelProps) {
  const byId = new Map(plan.steps.map((step) => [step.id, step]));

  return (
    <div
      data-testid="query-plan"
      className="flex-none border-b border-line bg-panel/60"
    >
      <div className="flex h-9 items-center justify-between gap-3 border-b border-line/70 px-3">
        <div className="flex items-center gap-2.5">
          <span className="text-[10px] font-bold uppercase tracking-[0.14em] text-muted">
            Query plan
          </span>
          <span className="rounded-full border border-line bg-panel-3 px-1.5 py-0.5 text-[10px] text-muted">
            {plan.steps.length} step{plan.steps.length === 1 ? "" : "s"}
          </span>
          <span
            className={cn(
              "rounded-full border px-1.5 py-0.5 text-[10px] font-semibold",
              plan.full_scans > 0
                ? "border-danger/30 bg-danger/10 text-[#f85149]"
                : "border-accent/30 bg-accent/10 text-accent",
            )}
          >
            {plan.full_scans > 0
              ? `⚠ ${plan.full_scans} full scan${plan.full_scans === 1 ? "" : "s"}`
              : "no full scans"}
          </span>
          <span className="rounded-full border border-line bg-panel-3 px-1.5 py-0.5 text-[10px] text-muted">
            {plan.indexes_used} index lookup{plan.indexes_used === 1 ? "" : "s"}
          </span>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close query plan"
          className="grid h-6 w-6 place-items-center rounded-md text-lg leading-none text-muted hover:bg-panel-2 hover:text-fg"
        >
          ×
        </button>
      </div>

      <div className="max-h-[170px] overflow-auto">
        {plan.steps.length === 0 ? (
          <div className="px-3 py-3 text-[11px] text-muted-2">No plan steps returned.</div>
        ) : (
          <table className="w-full border-collapse text-left">
            <thead className="sticky top-0 bg-panel-2 text-[9.5px] uppercase tracking-[0.08em] text-muted-2">
              <tr>
                <th className="w-[44px] px-3 py-1.5 font-semibold">Step</th>
                <th className="w-[52px] px-2 py-1.5 font-semibold">Parent</th>
                <th className="px-2 py-1.5 font-semibold">Detail</th>
              </tr>
            </thead>
            <tbody>
              {plan.steps.map((step) => (
                <tr
                  key={`${step.id}-${step.parent}`}
                  className={cn(
                    "border-t border-line/50",
                    step.full_scan && "bg-danger/[0.06]",
                    step.uses_index && !step.full_scan && "bg-accent/[0.04]",
                  )}
                >
                  <td className="px-3 py-1.5 font-mono text-[10.5px] text-muted-2">
                    {step.id}
                  </td>
                  <td className="px-2 py-1.5 font-mono text-[10.5px] text-muted-2">
                    {step.parent}
                  </td>
                  <td
                    className={cn(
                      "px-2 py-1.5 font-mono text-[11px]",
                      step.full_scan
                        ? "text-[#f85149]"
                        : step.uses_index
                          ? "text-[#7ee787]"
                          : "text-fg",
                    )}
                    style={{ paddingLeft: 8 + planDepth(step, byId) * 16 }}
                  >
                    {step.full_scan ? "⚠ " : step.uses_index ? "↗ " : ""}
                    {step.detail}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

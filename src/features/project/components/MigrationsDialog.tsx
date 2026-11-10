import { cn } from "@/lib/cn";
import type { MigrationInfo } from "@/types";

type MigrationsDialogProps = {
  open: boolean;
  migrations: MigrationInfo[];
  loading: boolean;
  applying: boolean;
  error: string | null;
  onClose: () => void;
  onRefresh: () => void;
  onApply: () => void;
};


export function MigrationsDialog({
  open,
  migrations,
  loading,
  applying,
  error,
  onClose,
  onRefresh,
  onApply,
}: MigrationsDialogProps) {
  if (!open) return null;

  const pending = migrations.filter((migration) => !migration.applied);
  const applied = migrations.filter((migration) => migration.applied);

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-auto bg-black/60 p-6 backdrop-blur-sm"
      onClick={(event) => {
        if (event.target === event.currentTarget && !applying) onClose();
      }}
    >
      <div
        className="w-full max-w-[780px] rounded-xl border border-line bg-panel shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="Migrations"
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight">Migrations</h2>
            <p className="mt-0.5 text-[11px] text-muted">
              Files in <span className="font-mono">migrations/</span> are applied in filename
              order. Each file gets a backup and an audit session.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={applying}
            aria-label="Close migrations"
            className="inline-flex h-6 w-6 items-center justify-center rounded-md border border-transparent text-lg leading-none text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg disabled:opacity-40"
          >
            ×
          </button>
        </div>

        <div className="max-h-[68vh] overflow-auto p-5">
          <div className="mb-3 flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-[11.5px] text-muted">
              <span className="rounded-full border border-accent/30 bg-accent/10 px-2 py-0.5 font-semibold text-accent">
                {pending.length} pending
              </span>
              <span className="rounded-full border border-line bg-panel-3 px-2 py-0.5">
                {applied.length} applied
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button className="btn btn-neutral" onClick={onRefresh} disabled={loading || applying}>
                {loading ? "Loading..." : "Refresh"}
              </button>
              <button
                className="btn btn-primary"
                onClick={onApply}
                disabled={applying || loading || pending.length === 0}
              >
                {applying ? "Applying..." : `Apply ${pending.length} pending`}
              </button>
            </div>
          </div>

          {error && (
            <div className="mb-3 rounded-md border border-danger/25 bg-danger/10 px-3 py-2 font-mono text-[11px] text-[#fecaca]">
              {error}
            </div>
          )}

          <div className="overflow-hidden rounded-lg border border-line">
            {migrations.length === 0 ? (
              <div className="px-3 py-6 text-center text-[11.5px] text-muted-2">
                {loading ? "Loading migrations..." : "No migrations yet."}
              </div>
            ) : (
              migrations.map((migration) => (
                <div
                  key={`${migration.name}-${migration.path ?? "db"}`}
                  className={cn(
                    "flex items-center justify-between gap-3 border-b border-line/60 px-3 py-2 last:border-b-0",
                    !migration.applied && "bg-accent/[0.04]",
                  )}
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="truncate font-mono text-[11.5px] text-fg">
                        {migration.name}
                      </span>
                      <span
                        className={cn(
                          "rounded-full border px-1.5 py-0.5 text-[9.5px] font-semibold uppercase tracking-[0.08em]",
                          migration.applied
                            ? "border-line bg-panel-3 text-muted"
                            : "border-accent/30 bg-accent/10 text-accent",
                        )}
                      >
                        {migration.applied ? "applied" : "pending"}
                      </span>
                    </div>
                    <div className="mt-0.5 truncate font-mono text-[10px] text-muted-2">
                      {migration.path ?? "recorded in database only"}
                      {migration.applied_at ? ` · ${migration.applied_at}` : ""}
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <button className="btn btn-neutral" type="button" onClick={onClose} disabled={applying}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

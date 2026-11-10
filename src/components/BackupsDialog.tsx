import { cn } from "../lib/cn";
import type { BackupInfo } from "../types";

type BackupsDialogProps = {
  open: boolean;
  backups: BackupInfo[];
  loading: boolean;
  error: string | null;
  restoring: string | null;
  creating: boolean;
  onClose: () => void;
  onRefresh: () => void;
  onCreateBackup: () => void;
  onRestore: (backup: BackupInfo) => void;
};

const BTN =
  "inline-flex h-8 items-center justify-center rounded-md border px-3 text-[12px] font-medium transition-colors disabled:cursor-default disabled:opacity-40";
const BTN_NEUTRAL = `${BTN} border-line-strong bg-panel-2 text-fg hover:border-[#3a4a5a] hover:bg-panel-3`;
const BTN_PRIMARY = `${BTN} border-[#14b8a6] bg-linear-to-b from-accent to-[#14b8a6] font-bold text-[#03110f] hover:brightness-110`;

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatWhen(ms: number): string {
  if (!ms) return "unknown";
  return new Date(ms).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function BackupsDialog({
  open,
  backups,
  loading,
  error,
  restoring,
  creating,
  onClose,
  onRefresh,
  onCreateBackup,
  onRestore,
}: BackupsDialogProps) {
  if (!open) return null;

  const busy = restoring !== null || creating;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-auto bg-black/60 p-6 backdrop-blur-sm"
      onClick={(event) => {
        if (event.target === event.currentTarget && !busy) onClose();
      }}
    >
      <div
        className="w-full max-w-[820px] rounded-xl border border-line bg-panel shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="Backups"
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight">Backups</h2>
            <p className="mt-0.5 text-[11px] text-muted">
              Backups are manual. Take one when you want a snapshot; restoring
              replaces the current database.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            aria-label="Close backups"
            className="inline-flex h-6 w-6 items-center justify-center rounded-md border border-transparent text-lg leading-none text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg disabled:opacity-40"
          >
            ×
          </button>
        </div>

        <div className="max-h-[68vh] overflow-auto p-5">
          <div className="mb-3 flex items-center justify-between gap-2">
            <span className="rounded-full border border-line bg-panel-3 px-2 py-0.5 text-[11px] text-muted">
              {backups.length} snapshot{backups.length === 1 ? "" : "s"}
            </span>
            <div className="flex items-center gap-2">
              <button
                className={BTN_PRIMARY}
                onClick={onCreateBackup}
                disabled={loading || busy}
                data-testid="create-backup"
              >
                {creating ? "Creating..." : "Take backup"}
              </button>
              <button
                className={BTN_NEUTRAL}
                onClick={onRefresh}
                disabled={loading || busy}
                data-testid="backups-refresh"
              >
                {loading ? "Loading..." : "Refresh"}
              </button>
            </div>
          </div>

          {error && (
            <div className="mb-3 rounded-md border border-danger/25 bg-danger/10 px-3 py-2 font-mono text-[11px] text-[#fecaca]">
              {error}
            </div>
          )}

          <div className="overflow-hidden rounded-lg border border-line">
            {backups.length === 0 ? (
              <div className="px-3 py-6 text-center text-[11.5px] text-muted-2">
                {loading ? "Loading backups..." : "No backups yet. Click Take backup to create one."}
              </div>
            ) : (
              backups.map((backup) => (
                <div
                  key={backup.path}
                  data-backup={backup.name}
                  className="flex items-center justify-between gap-3 border-b border-line/60 px-3 py-2 last:border-b-0"
                >
                  <div className="min-w-0">
                    <div className="truncate font-mono text-[11.5px] text-fg">
                      {backup.name}
                    </div>
                    <div className="mt-0.5 flex flex-wrap items-center gap-2 text-[10px] text-muted-2">
                      <span>{formatBytes(backup.size_bytes)}</span>
                      <span>·</span>
                      <span>{formatWhen(backup.modified_ms)}</span>
                      <span className="truncate">· {backup.path}</span>
                    </div>
                  </div>
                  <button
                    type="button"
                    className={cn(
                      BTN_PRIMARY,
                      "flex-none",
                      busy && "opacity-60",
                    )}
                    onClick={() => onRestore(backup)}
                    disabled={busy}
                    data-testid="restore-backup"
                  >
                    {restoring === backup.path ? "Restoring..." : "Restore"}
                  </button>
                </div>
              ))
            )}
          </div>

          <p className="mt-3 text-[10.5px] leading-relaxed text-muted-2">
            Backups are only created when you click <span className="font-mono">Take backup</span>
            {" "}or run <span className="font-mono">dbgo backup .</span>. Restoring replaces the
            current database; take a backup first if you need to undo it.
          </p>
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <button
            className={BTN_NEUTRAL}
            type="button"
            onClick={onClose}
            disabled={busy}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

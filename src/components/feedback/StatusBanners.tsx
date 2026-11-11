import { Save, Undo2, X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import {
  useWorkspacePendingSlice,
  useWorkspaceSessionSlice,
  useWorkspaceTableSlice,
} from "@/app/providers/WorkspaceContext";

export function StatusBanners() {
  const { db, error, setError, notice, setNotice } = useWorkspaceSessionSlice();
  const { selected } = useWorkspaceTableSlice();
  const {
    pendingInserts,
    pendingUpdates,
    pendingDeletes,
    saving,
    hasPending,
    discardChanges,
    saveChanges,
  } = useWorkspacePendingSlice();

  const pendingCount = pendingInserts.length + pendingUpdates.length + pendingDeletes.length;

  return (
    <>
      {error ? (
        <div className="flex flex-none items-center justify-between gap-3 border-b border-danger/25 bg-danger/10 px-3.5 py-2 font-mono text-[11.5px] text-[#f85149]">
          <span>{error}</span>
          <IconButton icon={X} label="Dismiss error" size="sm" onClick={() => setError(null)} />
        </div>
      ) : null}

      {notice ? (
        <div className="flex flex-none items-center justify-between gap-3 border-b border-accent/25 bg-accent/10 px-3.5 py-2 font-mono text-[11.5px] text-[#7ee787]">
          <span>{notice}</span>
          <IconButton icon={X} label="Dismiss notice" size="sm" onClick={() => setNotice(null)} />
        </div>
      ) : null}

      {db && hasPending ? (
        <div
          data-testid="pending-bar"
          className="flex flex-none items-center justify-between gap-3 border-b border-accent/30 bg-accent/[0.08] px-3.5 py-2"
        >
          <div className="flex min-w-0 items-center gap-2.5">
            <span className="h-1.5 w-1.5 flex-none rounded-full bg-accent shadow-[0_0_8px_rgba(45,212,191,0.7)]" />
            <span className="whitespace-nowrap font-mono text-[11.5px] font-semibold text-[#7ee787]">
              {pendingCount} unsaved change{pendingCount === 1 ? "" : "s"}
              {selected ? ` on ${selected}` : ""}
            </span>
            <span className="hidden whitespace-nowrap text-[10.5px] text-muted sm:inline">
              {pendingInserts.length} new · {pendingUpdates.length} edited ·{" "}
              {pendingDeletes.length} deleted
            </span>
          </div>
          <div className="flex flex-none items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              icon={Undo2}
              onClick={discardChanges}
              disabled={saving}
              data-testid="discard-changes"
            >
              Discard
            </Button>
            <Button
              variant="primary"
              size="sm"
              icon={Save}
              onClick={() => void saveChanges()}
              disabled={saving}
              data-testid="save-changes"
            >
              {saving ? "Saving..." : "Save changes"}
            </Button>
          </div>
        </div>
      ) : null}
    </>
  );
}

import {
  Archive,
  CloudUpload,
  FolderOpen,
  History,
  Play,
  Plus,
  RefreshCw,
  Upload,
} from "lucide-react";
import logo from "@/assets/dbgo-logo.png";
import { useWorkspace } from "@/app/providers/WorkspaceContext";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";

export function AppHeader() {
  const {
    db,
    busy,
    selected,
    view,
    refresh,
    openMigrations,
    openBackups,
    requestImport,
    requestD1Export,
    createNewDatabase,
    openDb,
    saving,
    restoringBackup,
    creatingBackup,
    queryRunning,
  } = useWorkspace();

  const refreshLabel =
    view === "sql"
      ? "Run current query"
      : view === "diagram"
        ? "Refresh schema diagram"
        : view === "structure"
          ? "Refresh table structure"
          : "Refresh current page";

  return (
    <header className="flex h-[52px] flex-none items-center justify-between border-b border-line bg-ink/80 px-4 backdrop-blur-xl">
      <div className="flex items-center gap-2.5">
        <img
          src={logo}
          alt="DBGo"
          className="h-10 w-10 object-contain"
        />
        <div className="font-bold tracking-tight">DBGo</div>
        <span className="ml-0.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-2">
          local SQL workbench
        </span>
      </div>

      <div className="flex items-center gap-1.5">
        {db && (
          <>
            <IconButton
              icon={view === "sql" ? Play : RefreshCw}
              label={refreshLabel}
              onClick={refresh}
              disabled={!selected || busy || queryRunning}
            />
            <IconButton
              icon={History}
              label="Migrations"
              onClick={openMigrations}
              disabled={busy || queryRunning || saving}
            />
            <IconButton
              icon={Archive}
              label="Backups"
              onClick={openBackups}
              disabled={busy || queryRunning || saving || restoringBackup !== null}
            />
            <IconButton
              icon={Upload}
              label="Import CSV, JSON, or JSONL"
              onClick={requestImport}
              disabled={busy || restoringBackup !== null || creatingBackup}
            />
            <IconButton
              icon={CloudUpload}
              label="Export Cloudflare D1 bundle"
              onClick={requestD1Export}
              disabled={busy || restoringBackup !== null || creatingBackup}
            />
          </>
        )}
        <Button
          variant="ghost"
          size="sm"
          icon={Plus}
          onClick={() => void createNewDatabase()}
        >
          New
        </Button>
        <Button
          variant="primary"
          size="sm"
          icon={FolderOpen}
          onClick={() => void openDb()}
        >
          Open
        </Button>
      </div>
    </header>
  );
}

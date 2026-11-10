import { Database, FolderOpen, HardDrive, Plus, X } from "lucide-react";
import type { useDatabaseSession } from "@/features/connection/hooks/useDatabaseSession";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { Badge } from "@/components/ui/Badge";
import { formatRecentOpened } from "@/components/ui/constants";

type DatabaseSession = ReturnType<typeof useDatabaseSession>;

export function WelcomeScreen({ session }: { session: DatabaseSession }) {
  const {
    recentDatabases,
    missingRecents,
    clearRecentDatabases,
    openRecentDatabase,
    removeRecentDatabase,
    createNewDatabase,
    openDb,
    loading,
  } = session;

  return (
    <div className="flex h-screen flex-col bg-ink">
      <header className="flex h-[52px] flex-none items-center gap-2.5 border-b border-line px-4">
        <div className="grid h-7 w-7 place-items-center rounded-lg bg-linear-to-br from-accent to-accent-2 text-[11px] font-extrabold tracking-tight text-[#04110f]">
          DB
        </div>
        <div className="font-bold tracking-tight">DBGo</div>
        <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-2">
          local SQL workbench
        </span>
      </header>

      <main className="flex min-h-0 flex-1 items-start justify-center overflow-auto px-6 py-14">
        <div className="w-full max-w-[620px]">
          <div className="mb-8 flex flex-col items-center text-center">
            <div className="grid h-[58px] w-[58px] place-items-center rounded-[16px] bg-linear-to-br from-accent to-accent-2 text-[#04110f] shadow-[0_14px_40px_rgba(45,212,191,0.25)]">
              <Database size={28} strokeWidth={1.8} />
            </div>
            <h1 className="mt-4 text-[24px] tracking-tight">Your local SQL workbench</h1>
            <p className="mt-1 max-w-[440px] text-[13px] leading-relaxed text-muted">
              Open an existing SQLite database or create a project to start exploring
              tables, writing queries, and visualizing relationships.
            </p>
            <div className="mt-5 flex items-center gap-2">
              <Button
                variant="primary"
                size="lg"
                icon={Plus}
                onClick={() => void createNewDatabase()}
                disabled={loading}
              >
                Create database
              </Button>
              <Button
                variant="secondary"
                size="lg"
                icon={FolderOpen}
                onClick={() => void openDb()}
                disabled={loading}
              >
                Open database
              </Button>
            </div>
            <div className="mt-2 font-mono text-[10.5px] text-muted-2">Ctrl / Cmd + O</div>
          </div>

          <section className="rounded-xl border border-line bg-panel/60">
            <div className="flex items-center justify-between border-b border-line px-4 py-3">
              <div className="flex items-center gap-2">
                <HardDrive size={14} className="text-muted-2" />
                <span className="text-[10.5px] font-bold uppercase tracking-[0.14em] text-muted">
                  Recent databases
                </span>
                <Badge tone="neutral">{recentDatabases.length}</Badge>
              </div>
              {recentDatabases.length > 0 ? (
                <Button variant="ghost" size="sm" onClick={clearRecentDatabases}>
                  Clear
                </Button>
              ) : null}
            </div>

            {recentDatabases.length === 0 ? (
              <div className="px-4 py-8 text-center text-[12px] text-muted-2">
                Databases you open will appear here.
              </div>
            ) : (
              <div>
                {recentDatabases.map((entry) => {
                  const missing = missingRecents.has(entry.path);
                  return (
                    <div
                      key={entry.path}
                      className="flex items-center gap-3 border-b border-line/60 px-4 py-2.5 last:border-b-0"
                    >
                      <button
                        type="button"
                        data-recent-path={entry.path}
                        onClick={() => void openRecentDatabase(entry)}
                        disabled={missing}
                        className="flex min-w-0 flex-1 items-center gap-3 text-left disabled:cursor-default disabled:opacity-45"
                        title={entry.path}
                      >
                        <Database size={15} className="flex-none text-muted-2" />
                        <span className="min-w-0 flex-1">
                          <span className="flex min-w-0 items-center gap-2">
                            <span className="truncate font-mono text-[12px] text-fg">
                              {entry.name}
                            </span>
                            {missing ? <Badge tone="danger">missing</Badge> : null}
                          </span>
                          <span className="mt-0.5 block truncate font-mono text-[10px] text-muted-2">
                            {entry.path}
                          </span>
                        </span>
                        <span className="flex-none text-[10px] text-muted-2">
                          {formatRecentOpened(entry.openedAt)}
                        </span>
                      </button>
                      <IconButton
                        icon={X}
                        label={`Remove ${entry.name} from recent databases`}
                        size="sm"
                        onClick={() => removeRecentDatabase(entry.path)}
                      />
                    </div>
                  );
                })}
              </div>
            )}
          </section>
        </div>
      </main>
    </div>
  );
}

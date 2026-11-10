import { useEffect, useState } from "react";
import { open as openFolder } from "@tauri-apps/plugin-dialog";
import { api } from "@/lib/api";
import { cn } from "@/lib/cn";
import type { D1ExportResult } from "@/types";

type D1ExportDialogProps = {
  open: boolean;
  defaultName: string;
  onClose: () => void;
  onExported: (result: D1ExportResult) => void;
};


export function D1ExportDialog({
  open,
  defaultName,
  onClose,
  onExported,
}: D1ExportDialogProps) {
  const [outputDir, setOutputDir] = useState("");
  const [name, setName] = useState(defaultName);
  const [force, setForce] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setOutputDir("");
    setName(defaultName);
    setForce(false);
    setExporting(false);
    setError(null);
  }, [open, defaultName]);

  if (!open) return null;

  const chooseFolder = async () => {
    const path = await openFolder({
      directory: true,
      multiple: false,
      title: "Choose a folder for the D1 export",
    });
    if (!path || Array.isArray(path)) return;
    setOutputDir(path);
    setError(null);
  };

  const runExport = async () => {
    if (!outputDir) {
      setError("Choose an output folder.");
      return;
    }
    setExporting(true);
    setError(null);
    try {
      const result = await api.d1Export(
        outputDir,
        name.trim() || null,
        force,
      );
      onExported(result);
      onClose();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setExporting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-auto bg-black/60 p-6 backdrop-blur-sm"
      onClick={(event) => {
        if (event.target === event.currentTarget && !exporting) onClose();
      }}
    >
      <div
        data-testid="d1-export-dialog"
        className="w-full max-w-[680px] rounded-xl border border-line bg-panel shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="D1 export"
      >
        <div className="flex items-center justify-between border-b border-line px-5 py-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight">D1 export</h2>
            <p className="mt-0.5 text-[11px] text-muted">
              Generate schema, data, Wrangler config, and a README. DBGo does not
              deploy it.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={exporting}
            aria-label="Close D1 export"
            className="inline-flex h-6 w-6 items-center justify-center rounded-md border border-transparent text-lg leading-none text-muted hover:border-line-strong hover:bg-panel-2 hover:text-fg disabled:opacity-40"
          >
            ×
          </button>
        </div>

        <div className="p-5">
          <div className="card">
            <div className="flex items-center justify-between gap-2">
              <div className="min-w-0">
                <div className="field-label">Output folder</div>
                <div className="mt-1 truncate font-mono text-[11.5px] text-fg">
                  {outputDir || "No folder selected"}
                </div>
              </div>
              <button
                type="button"
                className="btn btn-neutral"
                onClick={() => void chooseFolder()}
                disabled={exporting}
                data-testid="d1-choose-folder"
              >
                Choose folder
              </button>
            </div>
          </div>

          <div className="mt-3 grid gap-3 sm:grid-cols-2">
            <label className={cn("card", "flex flex-col gap-1.5")}>
              <span className="field-label">D1 database name</span>
              <input
                className="field-input"
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder={defaultName}
                aria-label="D1 database name"
              />
              <span className="text-[10px] text-muted-2">
                Used in wrangler.toml and the deploy instructions.
              </span>
            </label>
            <div className={cn("card", "flex flex-col justify-between gap-2")}>
              <div>
                <div className="field-label">Bundle</div>
                <div className="mt-1 text-[11px] leading-relaxed text-muted">
                  schema.sql · data-*.sql · wrangler.toml · manifest.json · README.md
                </div>
              </div>
              <label className="flex items-center gap-1.5 text-[11px] text-muted">
                <input
                  type="checkbox"
                  className="h-3.5 w-3.5 accent-accent"
                  checked={force}
                  onChange={(event) => setForce(event.target.checked)}
                />
                Overwrite generated files in a non-empty folder
              </label>
            </div>
          </div>

          {error && (
            <div className="mt-3 rounded-md border border-danger/25 bg-danger/10 px-3 py-2 font-mono text-[11px] text-[#fecaca]">
              {error}
            </div>
          )}

          <p className="mt-3 text-[10.5px] leading-relaxed text-muted-2">
            Internal <span className="font-mono">_dbgo_*</span> tables are excluded.
            Tables are exported in foreign-key order and data is split into batches.
            Deploy manually with Wrangler using the generated README.
          </p>
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <button
            className="btn btn-neutral"
            type="button"
            onClick={onClose}
            disabled={exporting}
          >
            Cancel
          </button>
          <button
            className="btn btn-primary"
            type="button"
            onClick={() => void runExport()}
            disabled={exporting}
            data-testid="d1-run"
          >
            {exporting ? "Exporting..." : "Export"}
          </button>
        </div>
      </div>
    </div>
  );
}

import { SqlEditorPanel } from "@/screens/database/sql/SqlEditorPanel";
import { ResultView } from "@/screens/database/results/ResultView";

export function SqlScreen() {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <SqlEditorPanel />
      <ResultView />
    </div>
  );
}

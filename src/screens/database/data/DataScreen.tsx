import { DataFilterBar } from "@/screens/database/data/DataFilterBar";
import { ViewToolbar } from "@/screens/database/data/ViewToolbar";
import { ResultView } from "@/screens/database/results/ResultView";

export function DataScreen() {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <ViewToolbar />
      <DataFilterBar />
      <ResultView />
    </div>
  );
}

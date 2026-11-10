import { useCallback, useRef, useState } from "react";
import { api } from "@/lib/api";
import type { TableDetails, TableStats } from "@/types";

type TableDetailsOptions = {
  setError: (error: string | null) => void;
};

export function useTableDetails({ setError }: TableDetailsOptions) {
  const [tableDetails, setTableDetails] = useState<TableDetails | null>(null);
  const [tableStats, setTableStats] = useState<TableStats | null>(null);
  const [detailsLoading, setDetailsLoading] = useState(false);
  const detailsRequestId = useRef(0);

  const loadDetails = useCallback(
    async (table: string) => {
      const id = ++detailsRequestId.current;
      setDetailsLoading(true);
      setError(null);
      try {
        const [details, stats] = await Promise.all([
          api.tableDetails(table),
          api.tableStats(table),
        ]);
        if (id !== detailsRequestId.current) return;
        setTableDetails(details);
        setTableStats(stats);
      } catch (cause) {
        if (id === detailsRequestId.current) {
          setError(String(cause));
          setTableDetails(null);
          setTableStats(null);
        }
      } finally {
        if (id === detailsRequestId.current) setDetailsLoading(false);
      }
    },
    [setError],
  );

  const resetTableDetails = useCallback(() => {
    detailsRequestId.current += 1;
    setTableDetails(null);
    setTableStats(null);
    setDetailsLoading(false);
  }, []);

  return {
    tableDetails,
    setTableDetails,
    tableStats,
    setTableStats,
    detailsLoading,
    loadDetails,
    resetTableDetails,
  };
}

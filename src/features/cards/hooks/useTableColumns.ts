import { useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";
import type { ColumnInfo } from "@/types";

export function useTableColumns(table: string | null) {
  const [columns, setColumns] = useState<ColumnInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestId = useRef(0);

  useEffect(() => {
    if (!table) {
      setColumns([]);
      setLoading(false);
      setError(null);
      return;
    }

    const id = ++requestId.current;
    setLoading(true);
    setError(null);
    api
      .describeTable(table)
      .then((next) => {
        if (id !== requestId.current) return;
        setColumns(next);
      })
      .catch((cause) => {
        if (id !== requestId.current) return;
        setError(String(cause));
        setColumns([]);
      })
      .finally(() => {
        if (id === requestId.current) setLoading(false);
      });

    return () => {
      requestId.current += 1;
    };
  }, [table]);

  return { columns, loading, error };
}

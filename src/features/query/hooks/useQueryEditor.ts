import { useCallback, useState } from "react";
import { api } from "@/lib/api";
import type { QueryPlan, QueryResult } from "@/types";

const QUERY_HISTORY_KEY = "dbgo:query-history";

type QueryEditorOptions = {
  setView: (view: "sql") => void;
  setSelectedRow: (row: null) => void;
  setError: (error: string | null) => void;
};

export function useQueryEditor({
  setView,
  setSelectedRow,
  setError,
}: QueryEditorOptions) {
  const [sqlText, setSqlText] = useState("");
  const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
  const [queryRunning, setQueryRunning] = useState(false);
  const [queryPlan, setQueryPlan] = useState<QueryPlan | null>(null);
  const [explainRunning, setExplainRunning] = useState(false);
  const [queryLimit, setQueryLimit] = useState(1000);
  const [queryHistory, setQueryHistory] = useState<string[]>(() => {
    try {
      const raw = localStorage.getItem(QUERY_HISTORY_KEY);
      if (!raw) return [];
      const parsed: unknown = JSON.parse(raw);
      return Array.isArray(parsed)
        ? parsed.filter((item): item is string => typeof item === "string")
        : [];
    } catch {
      return [];
    }
  });

  const runQuery = useCallback(
    async (text: string) => {
      const statement = text.trim();
      if (!statement) return;

      setSqlText(statement);
      setView("sql");
      setSelectedRow(null);
      setQueryPlan(null);
      setQueryRunning(true);
      setError(null);

      try {
        const result = await api.runQuery(statement, queryLimit);
        setQueryResult(result);
        setQueryHistory((current) => {
          const next = [statement, ...current.filter((item) => item !== statement)].slice(0, 20);
          try {
            localStorage.setItem(QUERY_HISTORY_KEY, JSON.stringify(next));
          } catch {
            // Ignore storage failures; history is a convenience.
          }
          return next;
        });
      } catch (cause) {
        setQueryResult(null);
        setError(String(cause));
      } finally {
        setQueryRunning(false);
      }
    },
    [queryLimit, setError, setSelectedRow, setView],
  );

  const explainQuery = useCallback(
    async (text: string) => {
      const statement = text.trim();
      if (!statement) return;

      setSqlText(statement);
      setView("sql");
      setSelectedRow(null);
      setExplainRunning(true);
      setError(null);
      try {
        setQueryPlan(await api.explainQuery(statement));
      } catch (cause) {
        setQueryPlan(null);
        setError(String(cause));
      } finally {
        setExplainRunning(false);
      }
    },
    [setError, setSelectedRow, setView],
  );

  const changeSqlText = useCallback((value: string) => {
    setSqlText(value);
    setQueryPlan(null);
  }, []);

  const resetQueryEditor = useCallback(() => {
    setSqlText("");
    setQueryResult(null);
    setQueryRunning(false);
    setQueryPlan(null);
    setExplainRunning(false);
  }, []);

  return {
    sqlText,
    setSqlText,
    queryResult,
    setQueryResult,
    queryRunning,
    queryPlan,
    setQueryPlan,
    explainRunning,
    queryLimit,
    setQueryLimit,
    queryHistory,
    runQuery,
    explainQuery,
    changeSqlText,
    resetQueryEditor,
  };
}

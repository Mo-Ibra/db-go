import type { FilterOp } from "@/types";

export const PAGE_SIZES = [100, 200, 500, 1000];
export const QUERY_LIMITS = [100, 500, 1000, 5000];

export const FILTER_OPS: { value: FilterOp; label: string }[] = [
  { value: "contains", label: "contains" },
  { value: "eq", label: "=" },
  { value: "neq", label: "≠" },
  { value: "gt", label: ">" },
  { value: "gte", label: "≥" },
  { value: "lt", label: "<" },
  { value: "lte", label: "≤" },
  { value: "is_null", label: "is null" },
  { value: "not_null", label: "is not null" },
];

export function formatRecentOpened(ms: number): string {
  const date = new Date(ms);
  const now = new Date();
  if (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  ) {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
}

export function isNullOp(op: FilterOp): boolean {
  return op === "is_null" || op === "not_null";
}

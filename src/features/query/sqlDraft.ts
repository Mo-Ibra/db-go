import { useSyncExternalStore } from "react";

/**
 * The SQL draft is intentionally kept outside of React state.
 *
 * Typing in the editor must not re-render the whole workspace, so the draft
 * lives in a tiny external store. Only components that subscribe through
 * `useSqlDraft` are notified.
 */
let draft = "";
const listeners = new Set<() => void>();

export function getSqlDraft(): string {
  return draft;
}

export function setSqlDraft(value: string): void {
  if (value === draft) return;
  draft = value;
  for (const listener of listeners) listener();
}

export function subscribeSqlDraft(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useSqlDraft(): string {
  return useSyncExternalStore(subscribeSqlDraft, getSqlDraft, getSqlDraft);
}

import type { FilterSpec, Page, SortSpec } from "@/types";

export function isPagedMode(
  sort: SortSpec | null,
  filter: FilterSpec | null,
): boolean {
  return sort !== null || filter !== null;
}

export function currentPageIndex(
  pagedMode: boolean,
  pageIndex: number,
  historyIndex: number,
): number {
  return pagedMode ? pageIndex : historyIndex;
}

export function rowBounds(
  page: Page | null,
  currentPage: number,
  pageSize: number,
): { start: number; end: number } {
  if (!page || page.rows.length === 0) return { start: 0, end: 0 };
  return {
    start: currentPage * pageSize + 1,
    end: currentPage * pageSize + page.rows.length,
  };
}

export function nextPageDisabled(loading: boolean, page: Page | null): boolean {
  return loading || !page?.has_more;
}

export function previousPageDisabled(
  loading: boolean,
  pagedMode: boolean,
  pageIndex: number,
  historyIndex: number,
): boolean {
  return loading || (pagedMode ? pageIndex <= 0 : historyIndex <= 0);
}

import { describe, expect, it } from "vitest";
import type { Page } from "@/types";
import {
  currentPageIndex,
  isPagedMode,
  nextPageDisabled,
  previousPageDisabled,
  rowBounds,
} from "@/lib/table/navigation";

const page: Page = {
  columns: [],
  rows: [
    { rowid: 1, cells: [] },
    { rowid: 2, cells: [] },
    { rowid: 3, cells: [] },
  ],
  next_cursor: 3,
  prev_cursor: 1,
  has_more: true,
};

describe("table navigation", () => {
  it("treats sorting or filtering as paged mode", () => {
    expect(isPagedMode(null, null)).toBe(false);
    expect(isPagedMode({ column: "name", direction: "asc" }, null)).toBe(true);
    expect(isPagedMode(null, { column: "id", op: "eq", value: "1" })).toBe(true);
  });

  it("selects the current page index for the active mode", () => {
    expect(currentPageIndex(true, 3, 9)).toBe(3);
    expect(currentPageIndex(false, 3, 9)).toBe(9);
  });

  it("calculates visible row bounds", () => {
    expect(rowBounds(page, 0, 200)).toEqual({ start: 1, end: 3 });
    expect(rowBounds(page, 1, 200)).toEqual({ start: 201, end: 203 });
    expect(rowBounds(null, 0, 200)).toEqual({ start: 0, end: 0 });
  });

  it("disables next when loading or at the end", () => {
    expect(nextPageDisabled(false, page)).toBe(false);
    expect(nextPageDisabled(true, page)).toBe(true);
    expect(nextPageDisabled(false, { ...page, has_more: false })).toBe(true);
  });

  it("disables previous on the first page", () => {
    expect(previousPageDisabled(false, true, 0, 0)).toBe(true);
    expect(previousPageDisabled(false, true, 1, 0)).toBe(false);
    expect(previousPageDisabled(false, false, 0, 0)).toBe(true);
    expect(previousPageDisabled(false, false, 0, 1)).toBe(false);
  });
});

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Page } from "@/types";

const { page } = vi.hoisted(() => ({
  page: {
    columns: [],
    rows: [{ rowid: 1, cells: [{ kind: "text", value: "Acme" }] }],
    next_cursor: 1,
    prev_cursor: 1,
    has_more: false,
  } satisfies Page,
}));

vi.mock("@/lib/api", () => ({
  api: {
    describeTable: vi.fn().mockResolvedValue([
      { name: "name", type_name: "TEXT", not_null: false, primary_key: false },
    ]),
    foreignKeys: vi.fn().mockResolvedValue([]),
    loadPage: vi.fn().mockResolvedValue(page),
    loadTablePage: vi.fn().mockResolvedValue(page),
    countRows: vi.fn().mockResolvedValue({ count: 1, capped: false }),
  },
}));

import { useTableData } from "@/features/tables/hooks/useTableData";

describe("useTableData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads a table and its columns", async () => {
    const { result } = renderHook(() =>
      useTableData({
        guardPending: () => false,
        resetTableUi: () => {},
        resetTableDetails: () => {},
        setView: () => {},
        setError: () => {},
      }),
    );

    await act(async () => {
      result.current.selectTable("companies");
    });

    await waitFor(() => expect(result.current.selected).toBe("companies"));
    expect(result.current.columns).toHaveLength(1);
    expect(result.current.page?.rows).toHaveLength(1);
    expect(result.current.count?.count).toBe(1);
    expect(result.current.loading).toBe(false);
  });

  it("refuses to switch tables while pending edits are guarded", async () => {
    const { result } = renderHook(() =>
      useTableData({
        guardPending: () => true,
        resetTableUi: () => {},
        resetTableDetails: () => {},
        setView: () => {},
        setError: () => {},
      }),
    );

    await act(async () => {
      result.current.selectTable("companies");
    });

    expect(result.current.selected).toBeNull();
  });
});

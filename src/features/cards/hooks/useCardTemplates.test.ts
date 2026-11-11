import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { ColumnInfo } from "@/types";
import { useCardTemplates } from "./useCardTemplates";

const columns: ColumnInfo[] = [
  { name: "id", type_name: "INTEGER", not_null: true, primary_key: true },
  { name: "name", type_name: "TEXT", not_null: true, primary_key: false },
  { name: "description", type_name: "TEXT", not_null: false, primary_key: false },
  { name: "revenue_usd", type_name: "REAL", not_null: false, primary_key: false },
];

describe("useCardTemplates", () => {
  beforeEach(() => localStorage.clear());

  it("bootstraps five templates and supports create, rename, duplicate, delete", async () => {
    const { result } = renderHook(() =>
      useCardTemplates({
        dbPath: "/tmp/example.db",
        table: "companies",
        columns,
      }),
    );

    await waitFor(() => expect(result.current.templates).toHaveLength(5));
    expect(result.current.activeTemplate?.layout).toBe("article");

    act(() => result.current.createTemplate("metric"));
    await waitFor(() => expect(result.current.templates).toHaveLength(6));
    expect(result.current.activeTemplate?.layout).toBe("metric");

    const active = result.current.activeTemplate!;
    act(() => result.current.renameTemplate(active.id, "My metric"));
    await waitFor(() =>
      expect(result.current.activeTemplate?.name).toBe("My metric"),
    );

    act(() => result.current.duplicateTemplate(active.id));
    await waitFor(() => expect(result.current.templates).toHaveLength(7));

    act(() => result.current.deleteTemplate(result.current.activeTemplate!.id));
    await waitFor(() => expect(result.current.templates).toHaveLength(6));
  });
});

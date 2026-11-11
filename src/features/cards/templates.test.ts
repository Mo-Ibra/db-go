import { beforeEach, describe, expect, it } from "vitest";
import type { ColumnInfo } from "@/types";
import { buildTemplates, guessLayout } from "./templates";
import { formatCardValue } from "./renderers";

const columns: ColumnInfo[] = [
  { name: "id", type_name: "INTEGER", not_null: true, primary_key: true },
  { name: "name", type_name: "TEXT", not_null: true, primary_key: false },
  { name: "description", type_name: "TEXT", not_null: false, primary_key: false },
  { name: "city", type_name: "TEXT", not_null: false, primary_key: false },
  { name: "sector", type_name: "TEXT", not_null: false, primary_key: false },
  { name: "revenue_usd", type_name: "REAL", not_null: false, primary_key: false },
  { name: "website", type_name: "TEXT", not_null: false, primary_key: false },
  { name: "active", type_name: "INTEGER", not_null: true, primary_key: false },
  { name: "created_at", type_name: "TEXT", not_null: true, primary_key: false },
  { name: "updated_at", type_name: "TEXT", not_null: true, primary_key: false },
];

describe("card templates", () => {
  beforeEach(() => localStorage.clear());

  it("builds all five layouts and chooses article for long text", () => {
    const store = buildTemplates("companies", columns);
    expect(store.templates).toHaveLength(5);
    expect(store.templates.map((template) => template.layout)).toEqual([
      "article",
      "profile",
      "compact",
      "metric",
      "keyValue",
    ]);
    expect(store.defaultTemplateId).toBe("companies-article");
    expect(store.templates[0].card.title?.column).toBe("name");
    expect(store.templates[0].card.description?.column).toBe("description");
  });

  it("guesses profile when an image blob exists", () => {
    const withLogo: ColumnInfo[] = [
      ...columns,
      { name: "logo", type_name: "BLOB", not_null: false, primary_key: false },
    ];
    expect(guessLayout(withLogo)).toBe("profile");
  });

  it("formats currency, booleans, and dates", () => {
    expect(
      formatCardValue({ kind: "real", value: 1234567 }, {
        column: "revenue_usd",
        format: "currency",
      }),
    ).toContain("$1,234,567");
    expect(
      formatCardValue({ kind: "integer", value: 1 }, {
        column: "active",
        format: "boolean",
      }),
    ).toBe("Yes");
    expect(
      formatCardValue({ kind: "text", value: "2026-10-29T07:35:12Z" }, {
        column: "updated_at",
        format: "date",
      }),
    ).toMatch(/2026/);
  });
});

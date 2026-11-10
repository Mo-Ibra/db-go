import { describe, expect, it } from "vitest";
import type { ColumnInfo, Row } from "@/types";
import {
  applyValuesToRow,
  mergePendingRows,
  mergePendingUpdate,
  pendingInsertToRow,
  rowToValues,
} from "@/lib/table/pendingEdits";

const columns: ColumnInfo[] = [
  { name: "id", type_name: "INTEGER", not_null: true, primary_key: true },
  { name: "name", type_name: "TEXT", not_null: false, primary_key: false },
  { name: "logo", type_name: "BLOB", not_null: false, primary_key: false },
];

function row(id: number, name: string): Row {
  return {
    rowid: id,
    cells: [
      { kind: "integer", value: id },
      { kind: "text", value: name },
      { kind: "null" },
    ],
  };
}

describe("pending edit helpers", () => {
  it("extracts editable values and skips blobs", () => {
    expect(rowToValues(columns, row(1, "Acme"))).toEqual({
      id: 1,
      name: "Acme",
    });
  });

  it("applies pending values without changing primary keys or blobs", () => {
    const updated = applyValuesToRow(columns, row(1, "Acme"), {
      id: 99,
      name: "Globex",
      logo: "ignored",
    });
    expect(updated.cells[0]).toEqual({ kind: "integer", value: 1 });
    expect(updated.cells[1]).toEqual({ kind: "text", value: "Globex" });
    expect(updated.cells[2]).toEqual({ kind: "null" });
  });

  it("builds a display row for a pending insert", () => {
    const inserted = pendingInsertToRow(columns, {
      key: "pending-1",
      values: { name: "Initech" },
    });
    expect(inserted.rowid).toBeNull();
    expect(inserted.pendingKey).toBe("pending-1");
    expect(inserted.cells[1]).toEqual({ kind: "text", value: "Initech" });
  });

  it("merges updates, deletes, and inserts into page rows", () => {
    const rows = mergePendingRows(
      [row(1, "Acme"), row(2, "Globex")],
      columns,
      [{ rowid: 1, values: { name: "Acme 2" }, before: { name: "Acme" } }],
      [2],
      [{ key: "pending-1", values: { name: "Initech" } }],
    );
    expect(rows).toHaveLength(2);
    expect(rows[0].cells[1]).toEqual({ kind: "text", value: "Acme 2" });
    expect(rows[1].pendingKey).toBe("pending-1");
  });

  it("drops updates that return to their original value", () => {
    const updates = mergePendingUpdate(
      [{ rowid: 1, values: { name: "Acme 2" }, before: { name: "Acme" } }],
      1,
      { name: "Acme" },
      { name: "Acme" },
    );
    expect(updates).toEqual([]);
  });

  it("creates a new update when one does not exist", () => {
    const updates = mergePendingUpdate([], 1, { name: "Globex" }, { name: "Acme" });
    expect(updates).toEqual([
      { rowid: 1, values: { name: "Globex" }, before: { name: "Acme" } },
    ]);
  });
});

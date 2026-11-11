import type { CellValue } from "@/types";
import type { FieldRef } from "./model";
import { fieldLabel } from "./templates";

const numberFormatter = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 2,
});

const compactNumberFormatter = new Intl.NumberFormat(undefined, {
  notation: "compact",
  maximumFractionDigits: 1,
});

const currencyFormatter = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 0,
});

function asNumber(cell: CellValue): number | null {
  if (cell.kind === "integer" || cell.kind === "real") return cell.value;
  if (cell.kind === "text") {
    const parsed = Number(cell.value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function asText(cell: CellValue): string {
  switch (cell.kind) {
    case "null":
      return "—";
    case "blob":
      return `<blob ${cell.size} bytes>`;
    case "integer":
    case "real":
      return String(cell.value);
    case "text":
      return cell.value;
  }
}

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export function formatCardValue(cell: CellValue | undefined, field: FieldRef): string {
  if (!cell) return "—";
  if (cell.kind === "null") return "—";

  switch (field.format) {
    case "currency": {
      const number = asNumber(cell);
      return number === null ? asText(cell) : currencyFormatter.format(number);
    }
    case "number": {
      const number = asNumber(cell);
      if (number === null) return asText(cell);
      return Math.abs(number) >= 10_000
        ? compactNumberFormatter.format(number)
        : numberFormatter.format(number);
    }
    case "date":
      return cell.kind === "text" ? formatDate(cell.value) : asText(cell);
    case "boolean": {
      if (cell.kind === "integer" || cell.kind === "real") {
        return cell.value === 0 ? "No" : "Yes";
      }
      if (cell.kind === "text") {
        const lower = cell.value.toLowerCase();
        if (["1", "true", "yes", "active", "published"].includes(lower)) return "Yes";
        if (["0", "false", "no", "inactive", "draft"].includes(lower)) return "No";
      }
      return asText(cell);
    }
    default:
      return asText(cell);
  }
}

export function fieldText(field: FieldRef): string {
  return field.label ?? fieldLabel(field.column);
}

export function cardInitial(title: string): string {
  const clean = title.trim();
  return clean ? clean.charAt(0).toUpperCase() : "?";
}

export function isLinkField(field: FieldRef): boolean {
  return field.format === "url" || field.format === "email";
}

export function linkHref(field: FieldRef, value: string): string {
  if (field.format === "email") return `mailto:${value}`;
  if (/^https?:\/\//i.test(value)) return value;
  return `https://${value}`;
}

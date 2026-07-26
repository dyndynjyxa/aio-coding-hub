import { parseYyyyMmDd } from "./localDate";

function clampNumber(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

export function dayKeyFromLocalDate(d: Date) {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function buildRecentDayKeys(days: number) {
  const n = clampNumber(Math.floor(days), 1, 60);
  const out: string[] = [];
  for (let delta = n - 1; delta >= 0; delta -= 1) {
    const d = new Date();
    d.setDate(d.getDate() - delta);
    out.push(dayKeyFromLocalDate(d));
  }
  return out;
}

export function toMmDd(dayKey: string) {
  return dayKey.slice(5).replace("-", "/");
}

export function buildDayKeysInRangeInclusive(startDay: string, endDay: string): string[] {
  const start = parseYyyyMmDd(startDay);
  const end = parseYyyyMmDd(endDay);
  if (!start || !end) return [];

  const startDate = new Date(start.year, start.month - 1, start.day, 0, 0, 0, 0);
  const endDate = new Date(end.year, end.month - 1, end.day, 0, 0, 0, 0);
  if (!Number.isFinite(startDate.getTime()) || !Number.isFinite(endDate.getTime())) return [];

  const out: string[] = [];
  const d = new Date(startDate);
  while (d.getTime() <= endDate.getTime()) {
    out.push(dayKeyFromLocalDate(d));
    d.setDate(d.getDate() + 1);
  }
  return out;
}

export function buildMonthToTodayDayKeys(): string[] {
  const now = new Date();
  const start = new Date(now.getFullYear(), now.getMonth(), 1, 0, 0, 0, 0);
  if (!Number.isFinite(start.getTime()) || !Number.isFinite(now.getTime())) return [];

  const out: string[] = [];
  const d = new Date(start);
  while (d.getTime() <= now.getTime()) {
    out.push(dayKeyFromLocalDate(d));
    d.setDate(d.getDate() + 1);
  }
  return out;
}

export function buildMonthKeysFromRows(rows: ReadonlyArray<{ day: string }>): string[] {
  const set = new Set<string>();
  for (const row of rows) {
    if (!row.day) continue;
    if (/^\d{4}-\d{2}$/.test(row.day)) set.add(row.day);
  }
  return Array.from(set).sort();
}

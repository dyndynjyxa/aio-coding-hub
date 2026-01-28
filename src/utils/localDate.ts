export type LocalDateParts = {
  year: number;
  month: number;
  day: number;
};

export function parseYyyyMmDd(date: string): LocalDateParts | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date);
  if (!m) return null;
  const year = Number(m[1]);
  const month = Number(m[2]);
  const day = Number(m[3]);
  if (!Number.isFinite(year) || !Number.isFinite(month) || !Number.isFinite(day)) return null;
  if (month < 1 || month > 12) return null;
  if (day < 1 || day > 31) return null;
  return { year, month, day };
}

export function unixSecondsAtLocalStartOfDay(date: string): number | null {
  const parts = parseYyyyMmDd(date);
  if (!parts) return null;
  const tsMs = new Date(parts.year, parts.month - 1, parts.day, 0, 0, 0, 0).getTime();
  if (!Number.isFinite(tsMs)) return null;
  return Math.floor(tsMs / 1000);
}

export function unixSecondsAtLocalStartOfNextDay(date: string): number | null {
  const parts = parseYyyyMmDd(date);
  if (!parts) return null;
  const tsMs = new Date(parts.year, parts.month - 1, parts.day + 1, 0, 0, 0, 0).getTime();
  if (!Number.isFinite(tsMs)) return null;
  return Math.floor(tsMs / 1000);
}

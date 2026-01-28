export const PERIOD_ITEMS = [
  { key: "daily", label: "今天" },
  { key: "weekly", label: "近 7 天" },
  { key: "monthly", label: "本月" },
  { key: "allTime", label: "全部" },
  { key: "custom", label: "自定义" },
] as const;

export type PeriodKey = (typeof PERIOD_ITEMS)[number]["key"];

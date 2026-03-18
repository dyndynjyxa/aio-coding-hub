import type { HomeUsagePeriod } from "../services/settings";

export const DEFAULT_HOME_USAGE_PERIOD: HomeUsagePeriod = "last15";

export function resolveHomeUsageWindowDays(period: HomeUsagePeriod, now = new Date()) {
  switch (period) {
    case "last7":
      return 7;
    case "last15":
      return 15;
    case "last30":
      return 30;
    case "month":
      return Math.max(1, now.getDate());
  }
}

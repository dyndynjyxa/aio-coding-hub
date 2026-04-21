import type { RequestLogDetail } from "../../services/gateway/requestLogs";
import type { ProviderChainAttemptLog } from "../ProviderChainView";
import { Card } from "../../ui/Card";
import { cn } from "../../utils/cn";
import { cliBadgeTone, cliShortLabel } from "../../constants/clis";
import { ProviderChainView } from "../ProviderChainView";

export type RequestLogDetailChainTabProps = {
  selectedLog: RequestLogDetail;
  attemptLogs: ProviderChainAttemptLog[];
  attemptLogsLoading: boolean;
  isInProgress: boolean;
  finalProviderText: string | null;
};

export function RequestLogDetailChainTab({
  selectedLog,
  attemptLogs,
  attemptLogsLoading,
  isInProgress,
  finalProviderText,
}: RequestLogDetailChainTabProps) {
  return (
    <div className="space-y-3">
      <Card padding="sm">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">决策链</div>
          <div className="flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
            <span
              className={cn(
                "rounded-full px-2 py-0.5 font-medium",
                cliBadgeTone(selectedLog.cli_key)
              )}
            >
              {cliShortLabel(selectedLog.cli_key)}
            </span>
            <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5">
              {isInProgress ? "当前供应商" : "最终供应商"}：{finalProviderText || "未知"}
            </span>
          </div>
        </div>
        <ProviderChainView
          attemptLogs={attemptLogs}
          attemptLogsLoading={attemptLogsLoading}
          attemptsJson={selectedLog.attempts_json}
        />
      </Card>
    </div>
  );
}

import type { RequestLogDetail } from "../../services/gateway/requestLogs";
import { DisclosureSection } from "./DisclosureSection";

export type RequestLogDetailRawTabProps = {
  selectedLog: RequestLogDetail;
};

export function RequestLogDetailRawTab({ selectedLog }: RequestLogDetailRawTabProps) {
  const errorDetailsJson = tryPrettyPrint(selectedLog.error_details_json);
  const attemptsJson = tryPrettyPrint(selectedLog.attempts_json);
  const usageJson = tryPrettyPrint(selectedLog.usage_json);

  const hasAny = errorDetailsJson != null || attemptsJson != null || usageJson != null;

  if (!hasAny) {
    return (
      <div className="text-sm text-slate-500 dark:text-slate-400">
        无原始数据。
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {errorDetailsJson != null ? (
        <DisclosureSection label="error_details_json" defaultOpen>
          <pre className="max-h-80 overflow-auto whitespace-pre-wrap break-all text-xs font-mono text-slate-700 dark:text-slate-300">
            {errorDetailsJson}
          </pre>
        </DisclosureSection>
      ) : null}

      {attemptsJson != null ? (
        <DisclosureSection label="attempts_json" defaultOpen={errorDetailsJson == null}>
          <pre className="max-h-80 overflow-auto whitespace-pre-wrap break-all text-xs font-mono text-slate-700 dark:text-slate-300">
            {attemptsJson}
          </pre>
        </DisclosureSection>
      ) : null}

      {usageJson != null ? (
        <DisclosureSection label="usage_json">
          <pre className="max-h-80 overflow-auto whitespace-pre-wrap break-all text-xs font-mono text-slate-700 dark:text-slate-300">
            {usageJson}
          </pre>
        </DisclosureSection>
      ) : null}
    </div>
  );
}

function tryPrettyPrint(json: string | null | undefined): string | null {
  if (!json) return null;
  try {
    const parsed = JSON.parse(json);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return json;
  }
}

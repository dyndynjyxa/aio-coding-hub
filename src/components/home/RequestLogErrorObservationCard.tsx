import { AlertTriangle, Lightbulb } from "lucide-react";
import { Card } from "../../ui/Card";
import { getGatewayErrorShortLabel } from "../../constants/gatewayErrorCodes";
import type { RequestLogErrorObservation } from "./requestLogErrorDetails";
import { DisclosureSection } from "./DisclosureSection";

export type RequestLogErrorObservationCardProps = {
  observation: RequestLogErrorObservation | null;
};

export function RequestLogErrorObservationCard({
  observation,
}: RequestLogErrorObservationCardProps) {
  if (!observation) return null;

  const shortLabel = observation.displayErrorCode
    ? getGatewayErrorShortLabel(observation.displayErrorCode)
    : null;
  const desc = observation.gwDescription?.desc ?? null;
  const suggestion = observation.gwDescription?.suggestion ?? null;

  const detailFields = buildDetailFields(observation);
  const hasDetails = detailFields.length > 0 || observation.upstreamBodyPreview != null || observation.rawDetailsText != null;

  return (
    <Card padding="sm">
      <div className="space-y-2.5">
        {/* Header: error code badge + description */}
        <div className="flex flex-wrap items-start gap-2">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-rose-500 dark:text-rose-400" />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              {observation.displayErrorCode ? (
                <span className="rounded-full bg-rose-50 px-2 py-0.5 text-xs font-semibold text-rose-700 dark:bg-rose-900/30 dark:text-rose-300">
                  {shortLabel !== observation.displayErrorCode
                    ? `${shortLabel} (${observation.displayErrorCode})`
                    : observation.displayErrorCode}
                </span>
              ) : null}
              {desc ? (
                <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                  {desc}
                </span>
              ) : null}
            </div>

            {/* Reason text (if no desc available, show reason as primary text) */}
            {!desc && observation.reason ? (
              <p className="mt-1 text-sm text-slate-700 dark:text-slate-300">
                {observation.reason}
              </p>
            ) : null}
          </div>
        </div>

        {/* Suggestion */}
        {suggestion ? (
          <div className="flex items-start gap-2 rounded-lg bg-amber-50/60 px-3 py-2 dark:bg-amber-900/15">
            <Lightbulb className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400" />
            <p className="text-xs text-amber-800 dark:text-amber-300">{suggestion}</p>
          </div>
        ) : null}

        {/* Expandable detail fields */}
        {hasDetails ? (
          <DisclosureSection label="详细信息">
            <div className="space-y-2">
              {detailFields.map(({ label, value }) => (
                <DetailRow key={label} label={label} value={value} />
              ))}

              {observation.upstreamBodyPreview ? (
                <div>
                  <div className="mb-1 text-xs font-medium text-slate-500 dark:text-slate-400">
                    上游响应预览
                  </div>
                  <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-all rounded-md bg-slate-100 px-2.5 py-2 text-xs font-mono text-slate-700 dark:bg-slate-800 dark:text-slate-300">
                    {observation.upstreamBodyPreview.length > 500
                      ? `${observation.upstreamBodyPreview.slice(0, 500)}…`
                      : observation.upstreamBodyPreview}
                  </pre>
                </div>
              ) : null}

              {observation.rawDetailsText && !observation.upstreamBodyPreview ? (
                <div>
                  <div className="mb-1 text-xs font-medium text-slate-500 dark:text-slate-400">
                    原始错误信息
                  </div>
                  <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-all rounded-md bg-slate-100 px-2.5 py-2 text-xs font-mono text-slate-700 dark:bg-slate-800 dark:text-slate-300">
                    {observation.rawDetailsText.length > 500
                      ? `${observation.rawDetailsText.slice(0, 500)}…`
                      : observation.rawDetailsText}
                  </pre>
                </div>
              ) : null}
            </div>
          </DisclosureSection>
        ) : null}
      </div>
    </Card>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline gap-2 text-xs">
      <span className="shrink-0 text-slate-500 dark:text-slate-400">{label}:</span>
      <span className="font-mono text-slate-700 dark:text-slate-300">{value}</span>
    </div>
  );
}

type DetailField = { label: string; value: string };

function buildDetailFields(obs: RequestLogErrorObservation): DetailField[] {
  const fields: DetailField[] = [];
  if (obs.errorCategory) fields.push({ label: "错误分类", value: obs.errorCategory });
  if (obs.upstreamStatus != null) fields.push({ label: "上游状态码", value: String(obs.upstreamStatus) });
  if (obs.decision) fields.push({ label: "决策", value: obs.decision });
  if (obs.selectionMethod) fields.push({ label: "选择方式", value: obs.selectionMethod });
  if (obs.reasonCode) fields.push({ label: "原因码", value: obs.reasonCode });
  if (obs.matchedRule) fields.push({ label: "匹配规则", value: obs.matchedRule });
  if (obs.reason && obs.gwDescription?.desc) {
    // Only show reason in detail if desc already shown as primary text
    fields.push({ label: "原因", value: obs.reason });
  }
  if (obs.circuitStateBefore || obs.circuitStateAfter) {
    const parts: string[] = [];
    if (obs.circuitStateBefore) parts.push(`${obs.circuitStateBefore}`);
    if (obs.circuitStateAfter && obs.circuitStateAfter !== obs.circuitStateBefore) {
      parts.push(`→ ${obs.circuitStateAfter}`);
    }
    if (obs.circuitFailureCount != null && obs.circuitFailureThreshold != null) {
      parts.push(`(${obs.circuitFailureCount}/${obs.circuitFailureThreshold})`);
    }
    fields.push({ label: "熔断器", value: parts.join(" ") });
  }
  if (obs.providerId != null) {
    const providerText = obs.providerName ? `${obs.providerName} (id=${obs.providerId})` : `id=${obs.providerId}`;
    fields.push({ label: "供应商", value: providerText });
  }
  return fields;
}

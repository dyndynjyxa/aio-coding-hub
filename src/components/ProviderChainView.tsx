import { useMemo, useState } from "react";
import { Spinner } from "../ui/Spinner";
import { cn } from "../utils/cn";
import { Globe, AlertTriangle, Zap, ChevronDown, ArrowRight } from "lucide-react";
import { getGatewayErrorShortLabel } from "../constants/gatewayErrorCodes";
import { DisclosureSection } from "./home/DisclosureSection";

export type ProviderChainAttemptLog = {
  attempt_index: number;
  provider_id: number;
  provider_name: string;
  base_url: string;
  outcome: string;
  status: number | null;
  attempt_started_ms?: number | null;
  attempt_duration_ms?: number | null;
};

type ProviderChainAttemptJson = {
  provider_id: number;
  provider_name: string;
  base_url: string;
  outcome: string;
  status: number | null;
  provider_index?: number | null;
  retry_index?: number | null;
  session_reuse?: boolean | null;
  error_category?: string | null;
  error_code?: string | null;
  decision?: string | null;
  reason?: string | null;
  selection_method?: string | null;
  reason_code?: string | null;
  attempt_started_ms?: number | null;
  attempt_duration_ms?: number | null;
  circuit_state_before?: string | null;
  circuit_state_after?: string | null;
  circuit_failure_count?: number | null;
  circuit_failure_threshold?: number | null;
};

type ProviderChainAttempt = {
  attempt_index: number;
  provider_id: number;
  provider_name: string;
  base_url: string;
  outcome: string;
  status: number | null;
  attempt_started_ms: number | null;
  attempt_duration_ms: number | null;
  provider_index: number | null;
  retry_index: number | null;
  session_reuse: boolean | null;
  error_category: string | null;
  error_code: string | null;
  decision: string | null;
  reason: string | null;
  selection_method: string | null;
  reason_code: string | null;
  circuit_state_before: string | null;
  circuit_state_after: string | null;
  circuit_failure_count: number | null;
  circuit_failure_threshold: number | null;
};

export function ProviderChainView({
  attemptLogs,
  attemptLogsLoading,
  attemptsJson,
}: {
  attemptLogs: ProviderChainAttemptLog[];
  attemptLogsLoading?: boolean;
  attemptsJson: string | null | undefined;
}) {
  const parsedAttemptsJson = useMemo(() => {
    if (!attemptsJson)
      return { ok: false as const, attempts: null as ProviderChainAttemptJson[] | null };
    try {
      const parsed = JSON.parse(attemptsJson);
      if (!Array.isArray(parsed)) {
        return { ok: false as const, attempts: null };
      }
      return { ok: true as const, attempts: parsed as ProviderChainAttemptJson[] };
    } catch {
      return { ok: false as const, attempts: null };
    }
  }, [attemptsJson]);

  const attempts = useMemo((): ProviderChainAttempt[] | null => {
    const logs = attemptLogs ?? [];
    const jsonAttempts = parsedAttemptsJson.ok ? parsedAttemptsJson.attempts : null;

    if (logs.length === 0 && !jsonAttempts) return null;

    if (logs.length === 0 && jsonAttempts) {
      return jsonAttempts.map((a, index) => ({
        attempt_index: index + 1,
        provider_id: a.provider_id,
        provider_name: a.provider_name,
        base_url: a.base_url,
        outcome: a.outcome,
        status: a.status ?? null,
        attempt_started_ms: a.attempt_started_ms ?? null,
        attempt_duration_ms: a.attempt_duration_ms ?? null,
        provider_index: a.provider_index ?? null,
        retry_index: a.retry_index ?? null,
        session_reuse: a.session_reuse ?? null,
        error_category: a.error_category ?? null,
        error_code: a.error_code ?? null,
        decision: a.decision ?? null,
        reason: a.reason ?? null,
        selection_method: a.selection_method ?? null,
        reason_code: a.reason_code ?? null,
        circuit_state_before: a.circuit_state_before ?? null,
        circuit_state_after: a.circuit_state_after ?? null,
        circuit_failure_count: a.circuit_failure_count ?? null,
        circuit_failure_threshold: a.circuit_failure_threshold ?? null,
      }));
    }

    const byAttemptIndex: Record<number, ProviderChainAttemptJson | undefined> = {};
    if (jsonAttempts) {
      for (let i = 0; i < jsonAttempts.length; i += 1) {
        byAttemptIndex[i + 1] = jsonAttempts[i];
      }
    }

    const normalized = logs
      .slice()
      .sort((a, b) => a.attempt_index - b.attempt_index)
      .map((log) => {
        const json = byAttemptIndex[log.attempt_index];
        return {
          attempt_index: log.attempt_index,
          provider_id: log.provider_id ?? json?.provider_id ?? 0,
          provider_name: log.provider_name || json?.provider_name || "未知",
          base_url: log.base_url || json?.base_url || "",
          outcome: log.outcome || json?.outcome || "",
          status: log.status ?? json?.status ?? null,
          attempt_started_ms: log.attempt_started_ms ?? json?.attempt_started_ms ?? null,
          attempt_duration_ms: log.attempt_duration_ms ?? json?.attempt_duration_ms ?? null,
          provider_index: json?.provider_index ?? null,
          retry_index: json?.retry_index ?? null,
          session_reuse: json?.session_reuse ?? null,
          error_category: json?.error_category ?? null,
          error_code: json?.error_code ?? null,
          decision: json?.decision ?? null,
          reason: json?.reason ?? null,
          selection_method: json?.selection_method ?? null,
          reason_code: json?.reason_code ?? null,
          circuit_state_before: json?.circuit_state_before ?? null,
          circuit_state_after: json?.circuit_state_after ?? null,
          circuit_failure_count: json?.circuit_failure_count ?? null,
          circuit_failure_threshold: json?.circuit_failure_threshold ?? null,
        };
      });

    return normalized;
  }, [attemptLogs, parsedAttemptsJson]);

  const dataSourceLabel = useMemo(() => {
    if (attemptLogsLoading) return "加载中…";
    if (attemptLogs.length > 0) {
      return parsedAttemptsJson.ok
        ? "数据源：request_logs.attempts_json（结构化）"
        : "数据源：attempts 兼容接口";
    }
    if (parsedAttemptsJson.ok) return "数据源：request_logs.attempts_json";
    return "数据源：尝试 JSON（原始）";
  }, [attemptLogs.length, attemptLogsLoading, parsedAttemptsJson.ok]);

  if (attemptLogsLoading) {
    return (
      <div className="mt-2 flex items-center gap-2 text-sm text-slate-600 dark:text-slate-400">
        <Spinner size="sm" />
        加载中…
      </div>
    );
  }

  if (!attempts) {
    return <div className="mt-2 text-sm text-slate-600 dark:text-slate-400">无故障切换尝试。</div>;
  }

  if (attempts.length === 0) {
    return <div className="mt-2 text-sm text-slate-600 dark:text-slate-400">无故障切换尝试。</div>;
  }

  const startAttempt = attempts[0] ?? null;
  const finalAttempt = attempts.length > 0 ? attempts[attempts.length - 1] : null;
  const startProviderLabel = startAttempt
    ? startAttempt.provider_name && startAttempt.provider_name !== "未知"
      ? startAttempt.provider_name
      : `未知（id=${startAttempt.provider_id}）`
    : "—";
  const finalProviderLabel = finalAttempt
    ? finalAttempt.provider_name && finalAttempt.provider_name !== "未知"
      ? finalAttempt.provider_name
      : `未知（id=${finalAttempt.provider_id}）`
    : "—";
  const finalSuccess = finalAttempt ? finalAttempt.outcome === "success" : false;

  return (
    <div className="mt-4 space-y-4">
      <div className="flex flex-wrap items-center gap-2 text-xs text-slate-600 dark:text-slate-400">
        <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2.5 py-1">
          起始供应商：
          <span className="font-medium text-slate-800 dark:text-slate-200">
            {startProviderLabel}
          </span>
        </span>
        <span className="text-slate-400 dark:text-slate-500">→</span>
        <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2.5 py-1">
          最终供应商：
          <span className="font-medium text-slate-800 dark:text-slate-200">
            {finalProviderLabel}
          </span>
        </span>
        <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2.5 py-1">
          共尝试 {attempts.length} 次
        </span>
        {finalAttempt ? (
          <span
            className={cn(
              "rounded-full px-2.5 py-1 font-medium",
              finalSuccess
                ? "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400"
                : "bg-rose-50 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400"
            )}
          >
            {finalSuccess ? "最终成功" : "最终失败"}
          </span>
        ) : null}
        <span className="text-slate-400 dark:text-slate-500">{dataSourceLabel}</span>
        {attemptLogs.length === 0 && parsedAttemptsJson.ok ? (
          <span className="rounded-full bg-slate-100 dark:bg-slate-700 px-2.5 py-1 font-medium text-slate-700 dark:text-slate-300">
            当前显示的是摘要链路，未拿到逐次尝试日志
          </span>
        ) : null}
        {attemptsJson && !parsedAttemptsJson.ok ? (
          <span className="rounded-full bg-amber-50 dark:bg-amber-900/30 px-2.5 py-1 font-medium text-amber-700 dark:text-amber-400">
            尝试 JSON 解析失败
          </span>
        ) : null}
      </div>

      <div className="relative pl-8">
        <div className="absolute left-[15px] top-2 bottom-2 w-px bg-slate-200 dark:bg-slate-700" />
        <div className="space-y-4">
          {attempts.map((attempt) => (
            <AttemptCard
              key={`${attempt.attempt_index}-${attempt.provider_id}-${attempt.base_url}`}
              attempt={attempt}
              isFinal={Boolean(
                finalAttempt && attempt.attempt_index === finalAttempt.attempt_index
              )}
              hasMultipleAttempts={attempts.length > 1}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function AttemptCard({
  attempt,
  isFinal,
  hasMultipleAttempts,
}: {
  attempt: ProviderChainAttempt;
  isFinal: boolean;
  hasMultipleAttempts: boolean;
}) {
  const [expanded, setExpanded] = useState(true);
  const success = attempt.outcome === "success";
  const skipped = attempt.outcome === "skipped";

  const hasCircuitBreaker =
    attempt.circuit_state_after != null || attempt.circuit_state_before != null;

  return (
    <div className="relative">
      <div
        className={cn(
          "absolute -left-8 top-4 flex h-8 w-8 items-center justify-center rounded-full border-2 bg-white text-sm font-semibold shadow-sm dark:bg-slate-900",
          success
            ? "border-emerald-300 text-emerald-600 dark:border-emerald-700 dark:text-emerald-400"
            : skipped
              ? "border-slate-300 text-slate-500 dark:border-slate-600 dark:text-slate-300"
              : "border-rose-300 text-rose-600 dark:border-rose-700 dark:text-rose-400"
        )}
      >
        {attempt.attempt_index}
      </div>

      <div
        className={cn(
          "rounded-2xl border bg-white shadow-sm dark:bg-slate-800/90 overflow-hidden",
          isFinal
            ? success
              ? "border-emerald-200 bg-emerald-50/50 dark:border-emerald-700 dark:bg-emerald-900/20"
              : skipped
                ? "border-slate-200 bg-slate-50/50 dark:border-slate-600 dark:bg-slate-700/20"
                : "border-rose-200 bg-rose-50/50 dark:border-rose-700 dark:bg-rose-900/20"
            : "border-slate-200 dark:border-slate-700"
        )}
      >
        {/* Header */}
        <button
          type="button"
          className="w-full text-left px-4 py-3 flex items-center justify-between gap-2 hover:bg-slate-50/50 dark:hover:bg-slate-700/30 transition-colors"
          onClick={() => setExpanded((prev) => !prev)}
        >
          <div className="flex flex-wrap items-center gap-2 min-w-0">
            <span className="text-base font-semibold text-slate-900 dark:text-slate-100">
              {success
                ? `请求成功`
                : skipped
                  ? `跳过`
                  : hasMultipleAttempts
                    ? `重试 #${attempt.attempt_index}`
                    : `请求失败`}
            </span>
            {attempt.attempt_duration_ms != null ? (
              <span className="text-xs text-slate-500 dark:text-slate-400">
                +{attempt.attempt_duration_ms}ms
              </span>
            ) : null}
            {attempt.status != null ? (
              <span
                className={cn(
                  "text-xs font-medium",
                  attempt.status >= 400
                    ? "text-rose-600 dark:text-rose-400"
                    : "text-slate-500 dark:text-slate-400"
                )}
              >
                HTTP {attempt.status}
              </span>
            ) : null}
          </div>
          <ChevronDown
            className={cn(
              "h-4 w-4 text-slate-400 shrink-0 transition-transform",
              expanded && "rotate-180"
            )}
          />
        </button>

        {/* Detail body */}
        {expanded && (
          <div className="px-4 pb-4 space-y-3 border-t border-slate-100 dark:border-slate-700/50 pt-3">
            <div className="text-sm text-slate-500 dark:text-slate-400">
              Provider ID: <span className="font-semibold text-slate-800 dark:text-slate-200">{attempt.provider_id}</span>
            </div>

            {/* Decision tags */}
            <DecisionTags attempt={attempt} />

            {attempt.base_url ? (
              <div className="flex items-start gap-2 text-sm">
                <Globe className="h-4 w-4 text-slate-400 dark:text-slate-500 shrink-0 mt-0.5" />
                <div>
                  <span className="text-slate-500 dark:text-slate-400">端点</span>
                  <div className="font-mono text-slate-800 dark:text-slate-200 break-all">
                    {attempt.base_url}
                  </div>
                </div>
              </div>
            ) : null}

            {hasCircuitBreaker ? (
              <div className="flex items-center gap-2 text-sm">
                <Zap className="h-4 w-4 text-slate-400 dark:text-slate-500 shrink-0" />
                <span className="text-slate-500 dark:text-slate-400">熔断器:</span>
                <CircuitBadge attempt={attempt} />
              </div>
            ) : null}

            {!success && !skipped && attempt.reason ? (
              <div className="rounded-lg border border-rose-200/60 bg-rose-50/50 px-3 py-3 dark:border-rose-500/20 dark:bg-rose-950/20">
                <div className="flex items-center gap-1.5 mb-2">
                  <AlertTriangle className="h-4 w-4 text-rose-500 dark:text-rose-400 shrink-0" />
                  <span className="text-sm font-semibold text-rose-600 dark:text-rose-400">
                    错误
                  </span>
                  {attempt.reason_code && attempt.reason_code !== attempt.reason ? (
                    <span className="rounded-full bg-rose-100 px-2 py-0.5 text-xs font-medium text-rose-600 dark:bg-rose-900/30 dark:text-rose-300">
                      {attempt.reason_code}
                    </span>
                  ) : null}
                </div>
                <pre className="whitespace-pre-wrap break-all text-xs font-mono text-rose-800 dark:text-rose-200 leading-relaxed">
                  {attempt.reason}
                </pre>
              </div>
            ) : null}

            {/* Expandable structured error details */}
            {!success && !skipped && hasStructuredDetails(attempt) ? (
              <DisclosureSection label="结构化错误详情">
                <div className="space-y-1.5 text-xs">
                  {attempt.error_code ? (
                    <div className="flex items-baseline gap-2">
                      <span className="shrink-0 text-slate-500 dark:text-slate-400">错误码:</span>
                      <span className="font-mono text-slate-700 dark:text-slate-300">
                        {getGatewayErrorShortLabel(attempt.error_code)} ({attempt.error_code})
                      </span>
                    </div>
                  ) : null}
                  {attempt.error_category ? (
                    <div className="flex items-baseline gap-2">
                      <span className="shrink-0 text-slate-500 dark:text-slate-400">错误分类:</span>
                      <span className="font-mono text-slate-700 dark:text-slate-300">{attempt.error_category}</span>
                    </div>
                  ) : null}
                  {attempt.decision ? (
                    <div className="flex items-baseline gap-2">
                      <span className="shrink-0 text-slate-500 dark:text-slate-400">决策:</span>
                      <span className="font-mono text-slate-700 dark:text-slate-300">{attempt.decision}</span>
                    </div>
                  ) : null}
                  {attempt.selection_method ? (
                    <div className="flex items-baseline gap-2">
                      <span className="shrink-0 text-slate-500 dark:text-slate-400">选择方式:</span>
                      <span className="font-mono text-slate-700 dark:text-slate-300">{attempt.selection_method}</span>
                    </div>
                  ) : null}
                  {hasCircuitBreaker ? (
                    <div className="flex items-baseline gap-2">
                      <span className="shrink-0 text-slate-500 dark:text-slate-400">熔断器变化:</span>
                      <span className="font-mono text-slate-700 dark:text-slate-300">
                        {attempt.circuit_state_before ?? "—"}
                        {attempt.circuit_state_after && attempt.circuit_state_after !== attempt.circuit_state_before ? (
                          <> <ArrowRight className="inline h-3 w-3" /> {attempt.circuit_state_after}</>
                        ) : null}
                        {attempt.circuit_failure_count != null && attempt.circuit_failure_threshold != null
                          ? ` (${attempt.circuit_failure_count}/${attempt.circuit_failure_threshold})`
                          : null}
                      </span>
                    </div>
                  ) : null}
                </div>
              </DisclosureSection>
            ) : null}
          </div>
        )}
      </div>
    </div>
  );
}

// --- Helper components for AttemptCard ---

const DECISION_BADGE_TONES: Record<string, string> = {
  switch: "bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-300",
  retry: "bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
  abort: "bg-rose-50 text-rose-700 dark:bg-rose-900/30 dark:text-rose-300",
};

function DecisionTags({ attempt }: { attempt: ProviderChainAttempt }) {
  const tags: Array<{ label: string; value: string; tone: string }> = [];

  if (attempt.selection_method) {
    tags.push({
      label: "选择",
      value: attempt.selection_method,
      tone: "bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
    });
  }
  if (attempt.decision) {
    tags.push({
      label: "决策",
      value: attempt.decision,
      tone: DECISION_BADGE_TONES[attempt.decision] ?? "bg-slate-100 text-slate-700 dark:bg-slate-700 dark:text-slate-300",
    });
  }
  if (attempt.error_code) {
    tags.push({
      label: "错误码",
      value: getGatewayErrorShortLabel(attempt.error_code),
      tone: "bg-rose-50 text-rose-700 dark:bg-rose-900/30 dark:text-rose-300",
    });
  }
  if (attempt.error_category) {
    tags.push({
      label: "分类",
      value: attempt.error_category,
      tone: "bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-300",
    });
  }

  if (tags.length === 0) return null;

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {tags.map((tag) => (
        <span
          key={tag.label}
          className={cn("rounded-full px-2 py-0.5 text-xs font-medium", tag.tone)}
          title={tag.label}
        >
          {tag.value}
        </span>
      ))}
    </div>
  );
}

function CircuitBadge({ attempt }: { attempt: ProviderChainAttempt }) {
  const state = attempt.circuit_state_after ?? attempt.circuit_state_before;
  return (
    <>
      <span
        className={cn(
          "rounded-md px-2 py-0.5 text-xs font-bold text-white",
          state === "open"
            ? "bg-rose-500"
            : state === "half_open"
              ? "bg-amber-500"
              : "bg-emerald-500"
        )}
      >
        {state}
      </span>
      {attempt.circuit_failure_count != null && attempt.circuit_failure_threshold != null ? (
        <span className="text-sm text-slate-600 dark:text-slate-300">
          {attempt.circuit_failure_count}/{attempt.circuit_failure_threshold} 次失败
        </span>
      ) : null}
    </>
  );
}

function hasStructuredDetails(attempt: ProviderChainAttempt): boolean {
  return (
    attempt.error_code != null ||
    attempt.error_category != null ||
    attempt.decision != null ||
    attempt.selection_method != null ||
    attempt.circuit_state_before != null ||
    attempt.circuit_state_after != null
  );
}

import { toast } from "sonner";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Tooltip } from "../ui/Tooltip";
import { copyText } from "../services/clipboard";
import type { ClaudeModelValidationResult } from "../services/claude/claudeModelValidation";
import type { ClaudeValidationTemplateKey } from "../services/claude/claudeValidationTemplates";
import {
  detectReverseProxyKeywords,
  evaluateClaudeValidation,
  getClaudeValidationOutputExpectation,
} from "../services/claude/claudeValidationTemplates";
import { cn } from "../utils/cn";
import type { LucideIcon } from "lucide-react";
import {
  CheckCircle2,
  ChevronDown,
  XCircle,
  Clock,
  Zap,
  FileJson,
  Copy,
  Server,
  Box,
  Braces,
  Activity,
  ShieldCheck,
  ShieldAlert,
  BrainCircuit,
  Terminal,
} from "lucide-react";

type Props = {
  templateKey: ClaudeValidationTemplateKey;
  result: ClaudeModelValidationResult | null;
  mode?: "full" | "compact";
};

function get<T>(obj: unknown, key: string): T | null {
  if (!obj || typeof obj !== "object") return null;
  const v = (obj as Record<string, unknown>)[key];
  return v as T | null;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizeHeaderValues(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) return value.filter((v): v is string => typeof v === "string");
  return [];
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function truncateText(value: string, max = 220) {
  if (!value) return "";
  if (value.length <= max) return value;
  return `${value.slice(0, max)}…`;
}

function collectWordBoundaryHits(text: string, keywords: string[]) {
  if (!text || keywords.length === 0) return [];
  const hits: string[] = [];
  for (const keyword of keywords) {
    if (!keyword) continue;
    try {
      const re = new RegExp(`\\b${escapeRegExp(keyword)}\\b`, "i");
      if (re.test(text)) hits.push(keyword);
    } catch {
      // ignore
    }
  }
  return [...new Set(hits)];
}

type KeywordEvidenceLine = {
  lineNumber: number;
  lineText: string;
  matchedKeywords: string[];
};

function TextEvidenceSection({
  title,
  lines,
  keyPrefix,
}: {
  title: string;
  lines: KeywordEvidenceLine[];
  keyPrefix: string;
}) {
  if (lines.length === 0) return null;

  return (
    <div className="space-y-1.5">
      <div className="text-[11px] font-semibold text-amber-900 dark:text-amber-400">{title}</div>
      <div className="space-y-1">
        {lines.map((line) => (
          <div
            key={`${keyPrefix}_${line.lineNumber}`}
            className="rounded-md border border-amber-200 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 px-2 py-1"
          >
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <span className="font-mono text-[11px] text-amber-950 dark:text-amber-300">
                L{line.lineNumber}: {line.lineText || "—"}
              </span>
              {line.matchedKeywords.length > 0 ? (
                <span className="font-mono text-[10px] text-amber-700 dark:text-amber-400">
                  hit: {line.matchedKeywords.join(", ")}
                </span>
              ) : null}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function collectKeywordEvidenceLines(
  text: string,
  keywords: string[],
  opts?: { maxLines?: number; maxLineLength?: number }
): KeywordEvidenceLine[] {
  const maxLines = typeof opts?.maxLines === "number" ? Math.max(1, Math.floor(opts.maxLines)) : 16;
  const maxLineLength =
    typeof opts?.maxLineLength === "number" ? Math.max(40, Math.floor(opts.maxLineLength)) : 220;

  if (!text || !text.trim() || keywords.length === 0) return [];

  const compiled = keywords
    .filter((k) => Boolean(k))
    .map((keyword) => {
      try {
        return { keyword, re: new RegExp(`\\b${escapeRegExp(keyword)}\\b`, "i") };
      } catch {
        return null;
      }
    })
    .filter((v): v is { keyword: string; re: RegExp } => v != null);

  if (compiled.length === 0) return [];

  const lines = text.split(/\r?\n/);
  const out: KeywordEvidenceLine[] = [];

  for (let i = 0; i < lines.length; i += 1) {
    const lineText = lines[i] ?? "";
    const matchedKeywords = compiled
      .filter(({ re }) => re.test(lineText))
      .map(({ keyword }) => keyword);

    if (matchedKeywords.length === 0) continue;

    out.push({
      lineNumber: i + 1,
      lineText: truncateText(lineText, maxLineLength),
      matchedKeywords: [...new Set(matchedKeywords)].sort((a, b) => a.localeCompare(b)),
    });

    if (out.length >= maxLines) break;
  }

  return out;
}

// --- Components ---

function MetricCard({
  label,
  value,
  icon: Icon,
  subValue,
}: {
  label: string;
  value: React.ReactNode;
  icon?: LucideIcon;
  subValue?: string;
}) {
  return (
    <div className="group flex flex-col gap-1.5 rounded-xl border border-slate-200/60 dark:border-slate-700/60 bg-white/50 dark:bg-slate-800/50 p-3.5 shadow-sm transition-all hover:bg-white dark:hover:bg-slate-800 hover:shadow-md">
      <div className="flex items-center gap-2 text-[11px] font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">
        {Icon && (
          <Icon className="h-4 w-4 text-slate-400 group-hover:text-indigo-500 transition-colors" />
        )}
        {label}
      </div>
      <div className="flex items-baseline gap-1.5">
        <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">{value}</span>
        {subValue && <span className="text-xs text-slate-400 dark:text-slate-500">{subValue}</span>}
      </div>
    </div>
  );
}

function SectionHeader({ title, icon: Icon }: { title: string; icon: LucideIcon }) {
  return (
    <div className="flex items-center gap-2 border-b border-slate-100 dark:border-slate-700 pb-2 mb-3">
      <div className="rounded p-1 bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-400">
        <Icon className="h-3.5 w-3.5" />
      </div>
      <span className="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
        {title}
      </span>
    </div>
  );
}

function CheckRow({
  label,
  ok,
  value,
  required = true,
  helpText,
}: {
  label: string;
  ok?: boolean;
  value?: React.ReactNode;
  required?: boolean;
  helpText?: string | null;
}) {
  const help = typeof helpText === "string" ? helpText.trim() : "";
  return (
    <div className="flex items-center justify-between py-1.5 text-sm">
      <div className="flex items-center gap-2">
        {ok != null ? (
          ok ? (
            <CheckCircle2 className={cn("h-4 w-4 shrink-0", "text-emerald-500")} />
          ) : (
            <XCircle
              className={cn("h-4 w-4 shrink-0", required ? "text-rose-500" : "text-slate-400")}
            />
          )
        ) : (
          <div className="h-4 w-4 shrink-0 rounded-full border border-slate-300 dark:border-slate-600 bg-slate-100 dark:bg-slate-700" />
        )}
        <span
          className={cn(
            "text-slate-700 dark:text-slate-300",
            !required && "text-slate-400 dark:text-slate-500"
          )}
        >
          {label}
        </span>
        {help ? (
          <Tooltip
            content={help}
            placement="top"
            contentClassName="whitespace-pre-line max-w-[420px]"
          >
            <span
              className="inline-flex h-4 w-4 items-center justify-center rounded-full border border-slate-300 dark:border-slate-600 bg-slate-100 dark:bg-slate-700 text-[10px] font-bold leading-none text-slate-600 dark:text-slate-400 cursor-help"
              aria-label={`${label} 说明`}
              title="查看说明"
            >
              ?
            </span>
          </Tooltip>
        ) : null}
      </div>
      {value && (
        <span className="font-mono text-xs text-slate-600 dark:text-slate-400">{value}</span>
      )}
    </div>
  );
}

function formatClaudeValidationFailure(result: ClaudeModelValidationResult) {
  const status =
    typeof result.status === "number" && Number.isFinite(result.status) ? result.status : null;
  const raw = typeof result.error === "string" ? result.error.trim() : "";

  const statusLabel = status != null ? `HTTP ${status}` : "请求失败";

  if (status === 401 || status === 403) {
    return { summary: `${statusLabel} · 鉴权失败`, detail: "请检查 API Key 权限", raw };
  }
  if (status === 429) {
    return { summary: `${statusLabel} · 触发限流`, detail: "请稍后重试", raw };
  }
  if (status != null && status >= 500) {
    return { summary: `${statusLabel} · 服务端错误`, detail: "上游服务不可用", raw };
  }
  if (raw.includes("EMPTY_RESPONSE_BODY")) {
    return { summary: "空响应", detail: "上游未返回数据", raw };
  }
  if (raw.startsWith("SEC_INVALID_INPUT")) {
    return { summary: "配置无效", detail: "请检查高级请求配置", raw };
  }

  return { summary: status != null ? `${statusLabel}` : "未知错误", detail: raw, raw };
}

export function ClaudeModelValidationResultPanel({ templateKey, result, mode = "full" }: Props) {
  if (!result) {
    return (
      <div className="flex h-40 flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-slate-200 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-800/50 text-slate-500 dark:text-slate-400">
        <Server className="h-8 w-8 text-slate-300 dark:text-slate-600" />
        <span className="text-sm">暂无验证结果</span>
      </div>
    );
  }

  // --- Failure View ---
  if (!result.ok) {
    const failure = formatClaudeValidationFailure(result);
    return (
      <Card className="overflow-hidden !p-0">
        <div className="border-b border-rose-100 dark:border-rose-800 bg-rose-50 dark:bg-rose-900/30 px-4 py-3">
          <div className="flex items-center gap-2 text-rose-800 dark:text-rose-400">
            <XCircle className="h-5 w-5" />
            <span className="font-semibold">验证失败</span>
          </div>
        </div>
        <div className="p-4 space-y-4">
          <div className="space-y-1">
            <div className="text-lg font-medium text-slate-900 dark:text-slate-100">
              {failure.summary}
            </div>
            <div className="text-sm text-slate-500 dark:text-slate-400">{failure.detail}</div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <MetricCard label="状态码" value={result.status ?? "—"} icon={Activity} />
            <MetricCard
              label="延迟"
              value={result.duration_ms ? `${result.duration_ms}ms` : "—"}
              icon={Clock}
            />
          </div>

          {failure.raw && (
            <div className="rounded-lg bg-slate-950 p-3">
              <pre className="whitespace-pre-wrap font-mono text-[11px] leading-relaxed text-rose-300">
                {failure.raw}
              </pre>
            </div>
          )}
        </div>
      </Card>
    );
  }

  // --- Success View ---
  const evaluation = evaluateClaudeValidation(templateKey, result);
  const reverseProxy = detectReverseProxyKeywords(result);
  const signals = result.signals as unknown;
  const usage = result.usage as unknown;
  const grade = evaluation.grade;
  const overallPass = evaluation.overallPass;

  const mentionsBedrock = get<boolean>(signals, "mentions_amazon_bedrock");
  const outputChars = result.output_text_chars ?? 0;
  const outputPreview = result.output_text_preview ?? "";
  const roundtripStep2OutputPreview = get<string>(signals, "roundtrip_step2_output_preview");
  const outputPreviewForDisplay =
    typeof roundtripStep2OutputPreview === "string" && roundtripStep2OutputPreview.trim()
      ? roundtripStep2OutputPreview
      : outputPreview;

  const requestedModel = result.requested_model?.trim() || null;
  const respondedModel = result.responded_model?.trim() || null;
  const modelConsistency =
    requestedModel && respondedModel ? requestedModel === respondedModel : null;

  const cache5m = get<number>(usage, "cache_creation_5m_input_tokens");

  const inputTokens = get<number>(usage, "input_tokens");
  const outputTokens = get<number>(usage, "output_tokens");
  const cacheCreate = cache5m ?? get<number>(usage, "cache_creation_input_tokens");
  const cacheDetailPass = cacheCreate != null;
  const cacheRead = get<number>(usage, "cache_read_input_tokens");
  const cacheReadStep2 = get<number>(signals, "roundtrip_step2_cache_read_input_tokens");

  const {
    requireModelConsistency,
    requireSseStopReasonMaxTokens,
    requireThinkingOutput,
    requireSignature,
    requireResponseId,
    requireServiceTier,
    requireOutputConfig,
    requireToolSupport,
    requireMultiTurn,
    requireCacheDetail,
  } = evaluation.template.evaluation;

  const {
    outputChars: outputCheck,
    outputTokens: outputTokensCheck,
    cacheDetail: cacheDetailCheck,
    sseStopReasonMaxTokens: sseStopReasonMaxTokensCheck,
    modelConsistency: modelConsistencyCheck,
    thinkingOutput: thinkingCheck,
    signature: signatureCheck,
    signatureRoundtrip: signatureRoundtripCheck,
    crossProviderSignatureRoundtrip: crossProviderSignatureRoundtripCheck,
    thinkingPreserved: thinkingPreservedCheck,
    signatureTamper: signatureTamperCheck,
    responseId: responseIdCheck,
    serviceTier: serviceTierCheck,
    outputConfig: outputConfigCheck,
    toolSupport: toolSupportCheck,
    multiTurn: multiTurnCheck,
    cacheReadHit: cacheReadHitCheck,
    reverseProxy: reverseProxyCheck,
  } = evaluation.checks;

  const outputExpectation = getClaudeValidationOutputExpectation(evaluation.template);

  const shouldShowSseStopReasonRow =
    evaluation.template.key === "official_max_tokens_5" || requireSseStopReasonMaxTokens;

  const showCapabilitiesSection =
    requireOutputConfig ||
    requireToolSupport ||
    requireMultiTurn ||
    requireCacheDetail ||
    Boolean(cacheReadHitCheck);

  const showOutputExpectSection =
    Boolean(outputTokensCheck) ||
    (shouldShowSseStopReasonRow && Boolean(sseStopReasonMaxTokensCheck)) ||
    Boolean(outputCheck && outputExpectation);

  const sseStopReasonValue = (() => {
    if (!shouldShowSseStopReasonRow) return null;

    const responseParseMode = get<string>(signals, "response_parse_mode");
    const parsedAsSse = responseParseMode === "sse" || responseParseMode === "sse_fallback";
    const sseMessageDeltaSeen =
      get<boolean>(result.checks as unknown, "sse_message_delta_seen") === true;

    const raw = get<string>(result.checks as unknown, "sse_message_delta_stop_reason");
    const stopReason = typeof raw === "string" && raw.trim() ? raw.trim() : null;
    if (!parsedAsSse) return `parse_mode=${responseParseMode ?? "—"}`;
    if (!sseMessageDeltaSeen) return "缺少 message_delta";
    return stopReason ?? "缺少 stop_reason";
  })();

  const reverseProxyEvidence = (() => {
    const maxLinesPerSource = 16;
    const maxLineLength = 220;

    const headerNames = reverseProxy.sources.responseHeaders.headerNames;
    const headerKeywords = reverseProxy.sources.responseHeaders.hits;
    const responseHeaders = result.response_headers;

    const headers = headerNames.map((headerName) => {
      const values = isPlainObject(responseHeaders)
        ? normalizeHeaderValues((responseHeaders as Record<string, unknown>)[headerName])
        : [];
      const headerValue = values.length > 0 ? values.join(", ") : "—";
      const matchedKeywords = collectWordBoundaryHits(
        `${headerName}\n${values.join("\n")}`,
        headerKeywords
      ).sort((a, b) => a.localeCompare(b));

      return { headerName, headerValue: truncateText(headerValue, maxLineLength), matchedKeywords };
    });

    const output = collectKeywordEvidenceLines(
      outputPreview,
      reverseProxy.sources.outputPreview.hits,
      {
        maxLines: maxLinesPerSource,
        maxLineLength,
      }
    );
    const sse = collectKeywordEvidenceLines(
      result.raw_excerpt ?? "",
      reverseProxy.sources.rawExcerpt.hits,
      {
        maxLines: maxLinesPerSource,
        maxLineLength,
      }
    );

    return { headers, output, sse };
  })();

  const reverseProxyEvidenceCounts = {
    headers: reverseProxyEvidence.headers.length,
    output: reverseProxyEvidence.output.length,
    sse: reverseProxyEvidence.sse.length,
  };
  const reverseProxyEvidenceEmpty =
    reverseProxyEvidenceCounts.headers +
      reverseProxyEvidenceCounts.output +
      reverseProxyEvidenceCounts.sse ===
    0;

  const showEvidenceGradePill = Boolean(grade && !(overallPass === true && grade.label === "通过"));
  const showCacheMetrics = requireCacheDetail || requireOutputConfig || Boolean(cacheReadHitCheck);

  return (
    <div className="space-y-6">
      {reverseProxy.anyHit ? (
        mode === "full" ? (
          <Card className="overflow-hidden border border-amber-200 dark:border-amber-700 bg-amber-50/60 dark:bg-amber-900/20">
            <div className="flex items-start gap-3 px-4 py-3">
              <div className="mt-0.5 rounded-lg bg-amber-100 dark:bg-amber-900/50 p-1.5 text-amber-700 dark:text-amber-400 ring-1 ring-inset ring-amber-200 dark:ring-amber-700">
                <ShieldAlert className="h-4 w-4" />
              </div>
              <div className="min-w-0">
                <div className="text-sm font-semibold text-amber-900 dark:text-amber-400">
                  疑似逆向/反代痕迹（判定不通过）
                </div>
                <div className="mt-1 text-xs text-amber-800 dark:text-amber-400/80">
                  命中关键字：{" "}
                  <span className="font-mono">{reverseProxy.hits.join(", ") || "—"}</span>
                </div>
                <div className="mt-1 flex flex-wrap gap-1 text-xs text-amber-800 dark:text-amber-400/80">
                  {reverseProxy.sources.responseHeaders.hits.length > 0 ? (
                    <span className="rounded bg-amber-100 px-2 py-0.5 font-mono">
                      headers: {reverseProxy.sources.responseHeaders.hits.join(", ")}
                    </span>
                  ) : null}
                  {reverseProxy.sources.outputPreview.hits.length > 0 ? (
                    <span className="rounded bg-amber-100 px-2 py-0.5 font-mono">
                      output: {reverseProxy.sources.outputPreview.hits.join(", ")}
                    </span>
                  ) : null}
                  {reverseProxy.sources.rawExcerpt.hits.length > 0 ? (
                    <span className="rounded bg-amber-100 px-2 py-0.5 font-mono">
                      sse: {reverseProxy.sources.rawExcerpt.hits.join(", ")}
                    </span>
                  ) : null}
                </div>
                {reverseProxy.sources.responseHeaders.headerNames.length > 0 ? (
                  <div className="mt-1 text-xs text-amber-800">
                    命中响应头：{" "}
                    <span className="font-mono">
                      {reverseProxy.sources.responseHeaders.headerNames.join(", ")}
                    </span>
                  </div>
                ) : null}

                <details className="group mt-2 rounded-lg border border-amber-200 dark:border-amber-700 bg-white/60 dark:bg-slate-800/60 shadow-sm open:ring-2 open:ring-amber-500/20 transition-all">
                  <summary className="flex cursor-pointer items-center justify-between px-3 py-2 select-none">
                    <div className="min-w-0">
                      <div className="text-xs font-medium text-amber-900 truncate">
                        查看证据（仅展示命中项）
                        <span className="ml-2 font-mono text-[11px] text-amber-700">
                          headers:{reverseProxyEvidenceCounts.headers} · output:
                          {reverseProxyEvidenceCounts.output} · sse:{reverseProxyEvidenceCounts.sse}
                        </span>
                      </div>
                      <div className="mt-0.5 text-[11px] text-amber-800/80">
                        证据来源：headers（响应头）/ output（输出预览）/ sse（流式原文）
                      </div>
                    </div>
                    <ChevronDown className="h-4 w-4 shrink-0 text-amber-700 transition-transform group-open:rotate-180" />
                  </summary>

                  <div className="border-t border-amber-100 dark:border-amber-800 px-3 py-2 space-y-3">
                    {reverseProxyEvidence.headers.length > 0 ? (
                      <div className="space-y-1.5">
                        <div className="text-[11px] font-semibold text-amber-900">
                          headers（响应头）
                        </div>
                        <div className="space-y-1">
                          {reverseProxyEvidence.headers.map((h) => (
                            <div
                              key={h.headerName}
                              className="rounded-md border border-amber-200 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30 px-2 py-1"
                            >
                              <div className="flex flex-wrap items-baseline justify-between gap-2">
                                <span className="font-mono text-[11px] text-amber-950 dark:text-amber-300">
                                  {h.headerName}
                                </span>
                                {h.matchedKeywords.length > 0 ? (
                                  <span className="font-mono text-[10px] text-amber-700 dark:text-amber-400">
                                    hit: {h.matchedKeywords.join(", ")}
                                  </span>
                                ) : null}
                              </div>
                              <div className="mt-0.5 font-mono text-[10px] text-amber-900/80">
                                {h.headerValue}
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : null}

                    <TextEvidenceSection
                      title="output（输出预览）"
                      keyPrefix="output"
                      lines={reverseProxyEvidence.output}
                    />
                    <TextEvidenceSection
                      title="sse（流式原文）"
                      keyPrefix="sse"
                      lines={reverseProxyEvidence.sse}
                    />

                    {reverseProxyEvidenceEmpty ? (
                      <div className="text-xs text-amber-800">
                        已命中关键字，但未能定位具体证据（可能是文本为空或响应头结构异常）。
                      </div>
                    ) : null}
                  </div>
                </details>
              </div>
            </div>
          </Card>
        ) : (
          <div className="rounded-xl border border-amber-200 dark:border-amber-700 bg-amber-50/60 dark:bg-amber-900/20 px-4 py-3">
            <div className="flex items-center gap-2 text-xs font-semibold text-amber-900 dark:text-amber-300">
              <ShieldAlert className="h-4 w-4" />
              疑似逆向/反代痕迹（高风险）
            </div>
            <div className="mt-1 text-[11px] text-amber-800 dark:text-amber-200">
              命中关键字：<span className="font-mono">{reverseProxy.hits.join(", ") || "—"}</span>
            </div>
          </div>
        )
      ) : null}

      {/* 1. Header & Stats */}
      <Card className="overflow-hidden !p-0">
        <div
          className={cn(
            "flex items-center justify-between border-b px-4 py-3",
            overallPass === true
              ? "border-emerald-100 dark:border-emerald-800 bg-emerald-50/50 dark:bg-emerald-900/30"
              : overallPass === false
                ? "border-rose-100 dark:border-rose-800 bg-rose-50/50 dark:bg-rose-900/30"
                : "border-slate-100 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-800/50"
          )}
        >
          <div
            className={cn(
              "flex flex-wrap items-center gap-2",
              overallPass === true
                ? "text-emerald-800 dark:text-emerald-400"
                : overallPass === false
                  ? "text-rose-800 dark:text-rose-400"
                  : "text-slate-700 dark:text-slate-300"
            )}
          >
            {overallPass === false ? (
              <XCircle className="h-5 w-5" />
            ) : (
              <CheckCircle2 className="h-5 w-5" />
            )}
            <span className="font-semibold">模板检查</span>
            <span
              className={cn(
                "inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-semibold ring-1 ring-inset",
                overallPass === true
                  ? "bg-emerald-100 text-emerald-800 ring-emerald-200 dark:bg-emerald-900/30 dark:text-emerald-400 dark:ring-emerald-700"
                  : overallPass === false
                    ? "bg-rose-100 text-rose-800 ring-rose-200 dark:bg-rose-900/30 dark:text-rose-400 dark:ring-rose-700"
                    : "bg-slate-100 text-slate-700 ring-slate-200 dark:bg-slate-800/50 dark:text-slate-300 dark:ring-slate-600"
              )}
              title={
                overallPass === true
                  ? "模板检查通过"
                  : overallPass === false
                    ? "模板检查未通过"
                    : ""
              }
            >
              {overallPass == null ? "未知" : overallPass ? "通过" : "未通过"}
            </span>
            {showEvidenceGradePill ? (
              <span
                className={cn(
                  "ml-1 inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-semibold ring-1 ring-inset",
                  grade?.level === "A"
                    ? "bg-emerald-100 text-emerald-800 ring-emerald-200 dark:bg-emerald-900/30 dark:text-emerald-400 dark:ring-emerald-700"
                    : grade?.level === "B"
                      ? "bg-indigo-100 text-indigo-800 ring-indigo-200 dark:bg-indigo-900/30 dark:text-indigo-400 dark:ring-indigo-700"
                      : grade?.level === "C"
                        ? "bg-amber-100 text-amber-900 ring-amber-200 dark:bg-amber-900/30 dark:text-amber-400 dark:ring-amber-700"
                        : "bg-rose-100 text-rose-800 ring-rose-200 dark:bg-rose-900/30 dark:text-rose-400 dark:ring-rose-700"
                )}
                title={grade?.title ?? ""}
              >
                证据 {grade?.level} · {grade?.label}
              </span>
            ) : null}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {mentionsBedrock && (
              <span className="inline-flex items-center gap-1 rounded-full bg-slate-100 dark:bg-slate-700 px-2.5 py-0.5 text-xs font-medium text-slate-600 dark:text-slate-400">
                <Server className="h-3 w-3" />
                Bedrock
              </span>
            )}
            <span className="font-mono text-xs text-slate-600 dark:text-slate-400">
              {result.requested_model ? `#${result.requested_model}` : "#—"}
            </span>
          </div>
        </div>

        {overallPass === true && grade && grade.level !== "A" ? (
          <div className="border-b border-slate-100 dark:border-slate-700 px-4 py-2 text-[11px] text-slate-600 dark:text-slate-400">
            证据说明：{grade.title}（不影响“通过”）
          </div>
        ) : null}

        {mode === "full" ? (
          <div
            className={cn(
              "grid grid-cols-2 gap-px bg-slate-100 dark:bg-slate-700",
              showCacheMetrics ? "sm:grid-cols-4" : "sm:grid-cols-3"
            )}
          >
            <div className="bg-white dark:bg-slate-800 p-4">
              <div className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400 mb-1">
                <Activity className="h-3.5 w-3.5" />
                HTTP
              </div>
              <div className="text-lg font-semibold text-slate-900 dark:text-slate-100">
                {result.status}
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 p-4">
              <div className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400 mb-1">
                <Clock className="h-3.5 w-3.5" />
                延迟
              </div>
              <div className="text-lg font-semibold text-slate-900 dark:text-slate-100">
                {result.duration_ms}ms
              </div>
            </div>
            <div className="bg-white dark:bg-slate-800 p-4">
              <div className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400 mb-1">
                <Zap className="h-3.5 w-3.5" />
                消耗
              </div>
              <div className="text-lg font-semibold text-slate-900 dark:text-slate-100">
                {inputTokens}{" "}
                <span className="text-xs text-slate-400 dark:text-slate-500">输入</span> ·{" "}
                {outputTokens}{" "}
                <span className="text-xs text-slate-400 dark:text-slate-500">输出</span>
              </div>
            </div>
            {showCacheMetrics ? (
              <div className="bg-white dark:bg-slate-800 p-4">
                <div className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400 mb-1">
                  <Box className="h-3.5 w-3.5" />
                  缓存
                </div>
                <div className="text-lg font-semibold text-slate-900 dark:text-slate-100">
                  {typeof cacheRead === "number" && Number.isFinite(cacheRead) ? cacheRead : "—"}{" "}
                  <span className="text-xs text-slate-400 dark:text-slate-500">读取</span> ·{" "}
                  {typeof cacheCreate === "number" && Number.isFinite(cacheCreate)
                    ? cacheCreate
                    : "—"}{" "}
                  <span className="text-xs text-slate-400 dark:text-slate-500">写入</span>
                </div>
                {typeof cacheReadStep2 === "number" && Number.isFinite(cacheReadStep2) ? (
                  <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
                    step2 read-hit:{" "}
                    <span className="font-mono text-slate-700 dark:text-slate-300">
                      {cacheReadStep2}
                    </span>
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        ) : (
          <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-2 text-[11px] text-slate-600 dark:text-slate-400">
            {(() => {
              const parts: string[] = [];
              if (typeof result.status === "number" && Number.isFinite(result.status)) {
                parts.push(`HTTP ${result.status}`);
              }
              if (typeof result.duration_ms === "number" && Number.isFinite(result.duration_ms)) {
                parts.push(`${result.duration_ms}ms`);
              }
              return parts.length > 0 ? parts.join(" · ") : "—";
            })()}
          </div>
        )}
      </Card>

      {/* 2. Detailed Checks Grid */}
      {(() => {
        const showRightColumn = showCapabilitiesSection || showOutputExpectSection;

        const grid = (
          <div className={cn("grid gap-6", showRightColumn ? "sm:grid-cols-2" : "sm:grid-cols-1")}>
            {/* Left Column: Core Checks */}
            <div className="space-y-6">
              <section>
                <SectionHeader title="协议 & 模型" icon={ShieldCheck} />
                <div className="space-y-1">
                  {reverseProxyCheck ? (
                    <CheckRow
                      label="疑似逆向/反代痕迹"
                      ok={reverseProxyCheck.ok}
                      value={reverseProxy.anyHit ? reverseProxy.hits.join(", ") : "—"}
                      helpText={reverseProxyCheck.title}
                    />
                  ) : null}
                  {requireModelConsistency ? (
                    <CheckRow
                      label="模型一致性"
                      ok={modelConsistencyCheck?.ok ?? modelConsistency ?? false}
                      value={respondedModel}
                      helpText={
                        modelConsistencyCheck?.title ??
                        `requested: ${requestedModel ?? "—"}; responded: ${respondedModel ?? "—"}`
                      }
                    />
                  ) : null}
                  {requireResponseId ? (
                    <CheckRow
                      label="响应 ID (ID)"
                      ok={responseIdCheck?.ok}
                      value="present"
                      helpText={responseIdCheck?.title ?? null}
                    />
                  ) : null}
                  {requireServiceTier ? (
                    <CheckRow
                      label="服务层级 (Tier)"
                      ok={serviceTierCheck?.ok}
                      value="present"
                      helpText={serviceTierCheck?.title ?? null}
                    />
                  ) : null}
                </div>
              </section>

              {requireThinkingOutput || requireSignature ? (
                <section>
                  <SectionHeader title="思考过程 (Thinking)" icon={BrainCircuit} />
                  <div className="space-y-1">
                    {requireThinkingOutput && thinkingCheck ? (
                      <CheckRow
                        label="思考输出"
                        ok={thinkingCheck.ok}
                        value={`${evaluation.derived.thinkingChars ?? 0} 字符`}
                        helpText={thinkingCheck.title}
                      />
                    ) : null}
                    {requireSignature && signatureCheck ? (
                      <CheckRow
                        label="思考签名"
                        ok={signatureCheck.ok}
                        value={`${evaluation.derived.signatureChars ?? 0} 字符`}
                        helpText={signatureCheck.title}
                      />
                    ) : null}
                    {signatureRoundtripCheck ? (
                      <CheckRow
                        label={signatureRoundtripCheck.label}
                        ok={signatureRoundtripCheck.ok}
                        helpText={signatureRoundtripCheck.title}
                      />
                    ) : null}
                    {crossProviderSignatureRoundtripCheck ? (
                      <CheckRow
                        label={crossProviderSignatureRoundtripCheck.label}
                        ok={crossProviderSignatureRoundtripCheck.ok}
                        helpText={crossProviderSignatureRoundtripCheck.title}
                      />
                    ) : null}
                    {thinkingPreservedCheck ? (
                      <CheckRow
                        label={thinkingPreservedCheck.label}
                        ok={thinkingPreservedCheck.ok}
                        helpText={thinkingPreservedCheck.title}
                      />
                    ) : null}
                    {signatureTamperCheck ? (
                      <CheckRow
                        label={signatureTamperCheck.label}
                        ok={signatureTamperCheck.ok}
                        helpText={signatureTamperCheck.title}
                      />
                    ) : null}
                  </div>
                </section>
              ) : null}
            </div>

            {/* Right Column: Capabilities & Output */}
            {showRightColumn ? (
              <div className="space-y-6">
                {showCapabilitiesSection ? (
                  <section>
                    <SectionHeader title="功能支持" icon={Terminal} />
                    <div className="space-y-1">
                      {requireOutputConfig ? (
                        <CheckRow
                          label="输出配置 (Output Config)"
                          ok={outputConfigCheck?.ok}
                          helpText={outputConfigCheck?.title ?? null}
                        />
                      ) : null}
                      {requireToolSupport ? (
                        <CheckRow
                          label="工具调用 (Tool Use)"
                          ok={toolSupportCheck?.ok}
                          helpText={toolSupportCheck?.title ?? null}
                        />
                      ) : null}
                      {requireMultiTurn ? (
                        <CheckRow
                          label="多轮对话 (Multi-turn)"
                          ok={multiTurnCheck?.ok}
                          helpText={multiTurnCheck?.title ?? null}
                        />
                      ) : null}
                      {requireCacheDetail ? (
                        <CheckRow
                          label="缓存明细 (Cache Breakdown)"
                          ok={cacheDetailCheck?.ok ?? cacheDetailPass}
                          value={`${cacheCreate ?? "-"}`}
                          helpText={cacheDetailCheck?.title ?? null}
                        />
                      ) : null}
                      {cacheReadHitCheck ? (
                        <CheckRow
                          label={cacheReadHitCheck.label}
                          ok={cacheReadHitCheck.ok}
                          helpText={cacheReadHitCheck.title}
                        />
                      ) : null}
                    </div>
                  </section>
                ) : null}

                {showOutputExpectSection ? (
                  <section>
                    <SectionHeader title="输出期望" icon={FileJson} />
                    <div className="space-y-1">
                      {evaluation.template.key === "official_max_tokens_5" ? (
                        <>
                          {outputTokensCheck ? (
                            <CheckRow
                              label={outputTokensCheck.label}
                              ok={outputTokensCheck.ok}
                              required={true}
                              value={typeof outputTokens === "number" ? outputTokens : "—"}
                              helpText={outputTokensCheck.title}
                            />
                          ) : null}
                        </>
                      ) : null}
                      {shouldShowSseStopReasonRow && sseStopReasonMaxTokensCheck ? (
                        <CheckRow
                          label={sseStopReasonMaxTokensCheck.label}
                          ok={sseStopReasonMaxTokensCheck.ok}
                          required={requireSseStopReasonMaxTokens}
                          value={sseStopReasonValue ?? "—"}
                          helpText={sseStopReasonMaxTokensCheck.title}
                        />
                      ) : null}
                      {outputCheck && outputExpectation ? (
                        <CheckRow
                          label={outputCheck.label}
                          ok={outputCheck.ok}
                          value={`${outputChars} 字符`}
                          helpText={outputCheck.title}
                        />
                      ) : null}
                    </div>
                  </section>
                ) : null}
              </div>
            ) : null}
          </div>
        );

        if (mode === "full") return grid;

        const open = evaluation.overallPass === false ? true : undefined;
        return (
          <details
            className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all"
            open={open}
          >
            <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
              <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
                <ChevronDown className="h-4 w-4 transition-transform group-open:rotate-180" />
                <span>检查点详情</span>
              </div>
              <div className="text-[10px] text-slate-500 dark:text-slate-400">
                {evaluation.overallPass === true ? "默认收起（通过）" : "默认展开（未通过）"}
              </div>
            </summary>
            <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3">{grid}</div>
          </details>
        );
      })()}

      {/* 3. Output Preview */}
      {outputPreviewForDisplay ? (
        mode === "full" ? (
          <section>
            <div className="mb-3 flex items-center justify-between">
              <SectionHeader title="输出预览" icon={Braces} />
              <Button
                size="sm"
                variant="secondary"
                className="!h-7 !px-2 text-xs"
                onClick={async () => {
                  try {
                    await copyText(outputPreviewForDisplay);
                    toast("已复制");
                  } catch {
                    toast.error("复制失败");
                  }
                }}
              >
                <Copy className="mr-1.5 h-3 w-3" />
              </Button>
            </div>
            <div className="group relative rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-900 p-4 font-mono text-xs leading-relaxed text-slate-300 shadow-sm transition-all hover:border-slate-300 dark:hover:border-slate-600">
              <span className="block whitespace-pre-wrap">{outputPreviewForDisplay}</span>
              <div className="pointer-events-none absolute inset-0 rounded-lg ring-1 ring-inset ring-slate-400/10" />
            </div>
          </section>
        ) : (
          <details className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
            <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
              <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
                <ChevronDown className="h-4 w-4 transition-transform group-open:rotate-180" />
                <span>输出预览</span>
              </div>
            </summary>
            <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3">
              <div className="mb-3 flex items-center justify-between">
                <div className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  输出预览
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  className="!h-7 !px-2 text-xs"
                  onClick={async () => {
                    try {
                      await copyText(outputPreviewForDisplay);
                      toast("已复制");
                    } catch {
                      toast.error("复制失败");
                    }
                  }}
                >
                  <Copy className="mr-1.5 h-3 w-3" />
                </Button>
              </div>
              <div className="group relative rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-900 p-4 font-mono text-xs leading-relaxed text-slate-300 shadow-sm transition-all hover:border-slate-300 dark:hover:border-slate-600">
                <span className="block whitespace-pre-wrap">{outputPreviewForDisplay}</span>
                <div className="pointer-events-none absolute inset-0 rounded-lg ring-1 ring-inset ring-slate-400/10" />
              </div>
            </div>
          </details>
        )
      ) : null}
    </div>
  );
}

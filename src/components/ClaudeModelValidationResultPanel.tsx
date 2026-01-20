import { toast } from "sonner";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Tooltip } from "../ui/Tooltip";
import type { ClaudeModelValidationResult } from "../services/claudeModelValidation";
import type { ClaudeValidationTemplateKey } from "../services/claudeValidationTemplates";
import {
  detectReverseProxyKeywords,
  evaluateClaudeValidation,
  getClaudeValidationOutputExpectation,
} from "../services/claudeValidationTemplates";
import { cn } from "../utils/cn";
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
      <div className="text-[11px] font-semibold text-amber-900">{title}</div>
      <div className="space-y-1">
        {lines.map((line) => (
          <div
            key={`${keyPrefix}_${line.lineNumber}`}
            className="rounded-md border border-amber-200 bg-amber-50 px-2 py-1"
          >
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <span className="font-mono text-[11px] text-amber-950">
                L{line.lineNumber}: {line.lineText || "—"}
              </span>
              {line.matchedKeywords.length > 0 ? (
                <span className="font-mono text-[10px] text-amber-700">
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
  icon?: any;
  subValue?: string;
}) {
  return (
    <div className="flex flex-col gap-1 rounded-lg border border-slate-200 bg-white p-3 shadow-sm">
      <div className="flex items-center gap-1.5 text-xs font-medium text-slate-500">
        {Icon && <Icon className="h-3.5 w-3.5" />}
        {label}
      </div>
      <div className="flex items-baseline gap-1.5">
        <span className="text-sm font-semibold text-slate-900">{value}</span>
        {subValue && <span className="text-xs text-slate-400">{subValue}</span>}
      </div>
    </div>
  );
}

function SectionHeader({ title, icon: Icon }: { title: string; icon: any }) {
  return (
    <div className="flex items-center gap-2 border-b border-slate-100 pb-2 mb-3">
      <div className="rounded p-1 bg-slate-100 text-slate-600">
        <Icon className="h-3.5 w-3.5" />
      </div>
      <span className="text-xs font-bold uppercase tracking-wider text-slate-500">{title}</span>
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
            <CheckCircle2
              className={cn(
                "h-4 w-4 shrink-0",
                "text-emerald-500"
              )}
            />
          ) : (
            <XCircle
              className={cn(
                "h-4 w-4 shrink-0",
                required ? "text-rose-500" : "text-slate-400"
              )}
            />
          )
        ) : (
          <div className="h-4 w-4 shrink-0 rounded-full border border-slate-300 bg-slate-100" />
        )}
        <span className={cn("text-slate-700", !required && "text-slate-400")}>{label}</span>
        {help ? (
          <Tooltip
            content={help}
            placement="top"
            contentClassName="whitespace-pre-line max-w-[420px]"
          >
            <span
              className="inline-flex h-4 w-4 items-center justify-center rounded-full border border-slate-300 bg-slate-100 text-[10px] font-bold leading-none text-slate-600 cursor-help"
              aria-label={`${label} 说明`}
              title="查看说明"
            >
              ?
            </span>
          </Tooltip>
        ) : null}
      </div>
      {value && <span className="font-mono text-xs text-slate-600">{value}</span>}
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

export function ClaudeModelValidationResultPanel({ templateKey, result }: Props) {
  if (!result) {
    return (
      <div className="flex h-40 flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-slate-200 bg-slate-50/50 text-slate-500">
        <Server className="h-8 w-8 text-slate-300" />
        <span className="text-sm">暂无验证结果</span>
      </div>
    );
  }

  // --- Failure View ---
  if (!result.ok) {
    const failure = formatClaudeValidationFailure(result);
    return (
      <Card className="overflow-hidden !p-0">
        <div className="border-b border-rose-100 bg-rose-50 px-4 py-3">
          <div className="flex items-center gap-2 text-rose-800">
            <XCircle className="h-5 w-5" />
            <span className="font-semibold">验证失败</span>
          </div>
        </div>
        <div className="p-4 space-y-4">
          <div className="space-y-1">
            <div className="text-lg font-medium text-slate-900">{failure.summary}</div>
            <div className="text-sm text-slate-500">{failure.detail}</div>
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
  const cacheDetailPass = cache5m != null;

  const inputTokens = get<number>(usage, "input_tokens");
  const outputTokens = get<number>(usage, "output_tokens");
  const cacheCreate = cache5m ?? get<number>(usage, "cache_creation_input_tokens");
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
    cacheDetail: cacheDetailCheck,
    sseStopReasonMaxTokens: sseStopReasonMaxTokensCheck,
    modelConsistency: modelConsistencyCheck,
    thinkingOutput: thinkingCheck,
    signature: signatureCheck,
    signatureRoundtrip: signatureRoundtripCheck,
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

  const expectedMaxTokens = (() => {
    const v = get<number>(evaluation.template.request.body as unknown, "max_tokens");
    return typeof v === "number" && Number.isFinite(v) ? v : null;
  })();
  const requestedMaxTokens = (() => {
    const req = (result as any)?.request as unknown;
    const body = get<unknown>(req, "body") ?? req;
    const v = get<number>(body, "max_tokens");
    return typeof v === "number" && Number.isFinite(v) ? v : null;
  })();
  const maxTokensConfigOk =
    expectedMaxTokens != null && requestedMaxTokens != null
      ? requestedMaxTokens === expectedMaxTokens
      : null;

  const shouldShowSseStopReasonRow =
    evaluation.template.key === "official_max_tokens_5" || requireSseStopReasonMaxTokens;

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

  return (
    <div className="space-y-6">
      {reverseProxy.anyHit ? (
        <Card className="overflow-hidden border border-amber-200 bg-amber-50/60">
          <div className="flex items-start gap-3 px-4 py-3">
            <div className="mt-0.5 rounded-lg bg-amber-100 p-1.5 text-amber-700 ring-1 ring-inset ring-amber-200">
              <ShieldAlert className="h-4 w-4" />
            </div>
            <div className="min-w-0">
              <div className="text-sm font-semibold text-amber-900">
                疑似逆向/反代痕迹（判定不通过）
              </div>
              <div className="mt-1 text-xs text-amber-800">
                命中关键字：{" "}
                <span className="font-mono">{reverseProxy.hits.join(", ") || "—"}</span>
              </div>
              <div className="mt-1 flex flex-wrap gap-1 text-xs text-amber-800">
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

              <details className="group mt-2 rounded-lg border border-amber-200 bg-white/60 shadow-sm open:ring-2 open:ring-amber-500/20 transition-all">
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

                <div className="border-t border-amber-100 px-3 py-2 space-y-3">
                  {reverseProxyEvidence.headers.length > 0 ? (
                    <div className="space-y-1.5">
                      <div className="text-[11px] font-semibold text-amber-900">
                        headers（响应头）
                      </div>
                      <div className="space-y-1">
                        {reverseProxyEvidence.headers.map((h) => (
                          <div
                            key={h.headerName}
                            className="rounded-md border border-amber-200 bg-amber-50 px-2 py-1"
                          >
                            <div className="flex flex-wrap items-baseline justify-between gap-2">
                              <span className="font-mono text-[11px] text-amber-950">
                                {h.headerName}
                              </span>
                              {h.matchedKeywords.length > 0 ? (
                                <span className="font-mono text-[10px] text-amber-700">
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
      ) : null}

      {/* 1. Header & Stats */}
      <Card className="overflow-hidden !p-0">
        <div className="flex items-center justify-between border-b border-emerald-100 bg-emerald-50/50 px-4 py-3">
          <div className="flex items-center gap-2 text-emerald-800">
            <CheckCircle2 className="h-5 w-5" />
            <span className="font-semibold">请求成功</span>
            {grade ? (
              <span
                className={cn(
                  "ml-2 inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-semibold ring-1 ring-inset",
                  grade.level === "A"
                    ? "bg-emerald-100 text-emerald-800 ring-emerald-200"
                    : grade.level === "B"
                      ? "bg-sky-100 text-sky-800 ring-sky-200"
                      : grade.level === "C"
                        ? "bg-amber-100 text-amber-900 ring-amber-200"
                        : "bg-rose-100 text-rose-800 ring-rose-200"
                )}
                title={grade.title}
              >
                {grade.level} · {grade.label}
              </span>
            ) : null}
          </div>
          <div className="flex items-center gap-2">
            {mentionsBedrock && (
              <span className="inline-flex items-center gap-1 rounded-full bg-slate-100 px-2.5 py-0.5 text-xs font-medium text-slate-600">
                <Server className="h-3 w-3" />
                Bedrock
              </span>
            )}
            <span className="font-mono text-xs text-emerald-700/70">#{result.requested_model}</span>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-px bg-slate-100 sm:grid-cols-4">
          <div className="bg-white p-4">
            <div className="flex items-center gap-2 text-xs text-slate-500 mb-1">
              <Activity className="h-3.5 w-3.5" />
              HTTP
            </div>
            <div className="text-lg font-semibold text-slate-900">{result.status}</div>
          </div>
          <div className="bg-white p-4">
            <div className="flex items-center gap-2 text-xs text-slate-500 mb-1">
              <Clock className="h-3.5 w-3.5" />
              延迟
            </div>
            <div className="text-lg font-semibold text-slate-900">{result.duration_ms}ms</div>
          </div>
          <div className="bg-white p-4">
            <div className="flex items-center gap-2 text-xs text-slate-500 mb-1">
              <Zap className="h-3.5 w-3.5" />
              消耗
            </div>
            <div className="text-lg font-semibold text-slate-900">
              {inputTokens} <span className="text-xs text-slate-400">输入</span> · {outputTokens}{" "}
              <span className="text-xs text-slate-400">输出</span>
            </div>
          </div>
          <div className="bg-white p-4">
            <div className="flex items-center gap-2 text-xs text-slate-500 mb-1">
              <Box className="h-3.5 w-3.5" />
              缓存
            </div>
            <div className="text-lg font-semibold text-slate-900">
              {cacheRead ?? 0} <span className="text-xs text-slate-400">读取</span> ·{" "}
              {cacheCreate ?? 0} <span className="text-xs text-slate-400">写入</span>
            </div>
            {typeof cacheReadStep2 === "number" && Number.isFinite(cacheReadStep2) ? (
              <div className="mt-1 text-[11px] text-slate-500">
                step2 read-hit:{" "}
                <span className="font-mono text-slate-700">{cacheReadStep2}</span>
              </div>
            ) : null}
          </div>
        </div>
      </Card>

      {/* 2. Detailed Checks Grid */}
      <div className="grid gap-6 sm:grid-cols-2">
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
                  ok={modelConsistencyCheck?.ok ?? (modelConsistency ?? false)}
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
        <div className="space-y-6">
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
                  value={`${cache5m ?? "-"}`}
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

          <section>
            <SectionHeader title="输出期望" icon={FileJson} />
            <div className="space-y-1">
              {evaluation.template.key === "official_max_tokens_5" ? (
                <>
                  <CheckRow
                    label={`请求 max_tokens=${expectedMaxTokens ?? "—"}`}
                    ok={maxTokensConfigOk === true}
                    required={true}
                    value={requestedMaxTokens ?? "—"}
                    helpText={[
                      "验证点：请求体 max_tokens 是否按模板配置发送。",
                      "说明：部分兼容层会忽略/重写 max_tokens；该项用于诊断“模板未生效 vs 上游未按 max_tokens 截断”。",
                      `观测：request.body.max_tokens=${requestedMaxTokens ?? "—"}; expected=${expectedMaxTokens ?? "—"}`,
                    ].join("\n")}
                  />
                  <CheckRow
                    label={`输出 tokens≤${expectedMaxTokens ?? "—"}`}
                    ok={
                      typeof outputTokens === "number" && expectedMaxTokens != null
                        ? outputTokens <= expectedMaxTokens
                        : undefined
                    }
                    required={typeof outputTokens === "number"}
                    value={typeof outputTokens === "number" ? outputTokens : "—"}
                    helpText={[
                      "验证点：usage.output_tokens 是否不超过 max_tokens。",
                      "说明：output_tokens 是更直接的 token 口径；若缺失则仅作为可选诊断项展示。",
                      `观测：output_tokens=${typeof outputTokens === "number" ? outputTokens : "—"}; max_tokens=${expectedMaxTokens ?? "—"}`,
                    ].join("\n")}
                  />
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
              {outputCheck && outputExpectation && (
                <CheckRow
                  label={outputCheck.label}
                  ok={outputCheck.ok}
                  value={`${outputChars} 字符`}
                  helpText={outputCheck.title}
                />
              )}
            </div>
          </section>
        </div>
      </div>

      {/* 3. Output Preview */}
      {outputPreviewForDisplay && (
        <section>
          <div className="mb-3 flex items-center justify-between">
            <SectionHeader title="输出预览" icon={Braces} />
            <Button
              size="sm"
              variant="secondary"
              className="!h-7 !px-2 text-xs"
              onClick={async () => {
                try {
                  await navigator.clipboard.writeText(outputPreviewForDisplay);
                  toast("已复制");
                } catch {
                  toast.error("复制失败");
                }
              }}
            >
              <Copy className="mr-1.5 h-3 w-3" />
            </Button>
          </div>
          <div className="group relative rounded-lg border border-slate-200 bg-slate-900 p-4 font-mono text-xs leading-relaxed text-slate-300 shadow-sm transition-all hover:border-slate-300">
            <span className="block whitespace-pre-wrap">{outputPreviewForDisplay}</span>
            <div className="pointer-events-none absolute inset-0 rounded-lg ring-1 ring-inset ring-slate-400/10" />
          </div>
        </section>
      )}
    </div>
  );
}

/**
 * Usage:
 *
 * <ClaudeModelValidationHistoryStepCard
 *   title="验证 1/8：Max Tokens"
 *   rightBadge={<OutcomePill pass={true} />}
 *   templateKey={templateKey}
 *   result={result}
 *   requestJsonText={requestJson}
 *   resultJsonText={resultJson}
 *   sseRawText={rawExcerpt}
 *   copyText={copyTextOrToast}
 * />
 */

import { useEffect, useMemo, useState, type MouseEvent, type ReactNode } from "react";
import type { ClaudeModelValidationResult } from "../services/claude/claudeModelValidation";
import { buildClaudeModelValidationRequestSnapshotTextFromResult } from "../services/claude/claudeModelValidationRequestSnapshot";
import {
  evaluateClaudeValidation,
  type ClaudeValidationTemplateKey,
} from "../services/claude/claudeValidationTemplates";
import { cn } from "../utils/cn";
import { Button } from "../ui/Button";
import { Textarea } from "../ui/Textarea";
import { ClaudeModelValidationResultPanel } from "./ClaudeModelValidationResultPanel";
import { Activity, ChevronDown, Copy, FileJson, Settings2 } from "lucide-react";

export type ClaudeModelValidationHistoryStepCardProps = {
  title: string;
  rightBadge?: ReactNode;
  templateKey: ClaudeValidationTemplateKey;
  result: ClaudeModelValidationResult | null;
  apiKeyPlaintext?: string | null;
  requestJsonText: string;
  resultJsonText: string;
  sseRawText: string;
  errorText?: string | null;
  defaultOpen?: boolean;
  copyText: (text: string, okMessage: string) => Promise<void> | void;
  className?: string;
};

function normalizeCopyText(value: string) {
  return typeof value === "string" ? value : "";
}

function stopDetailsToggle(e: MouseEvent) {
  e.preventDefault();
  e.stopPropagation();
}

export function ClaudeModelValidationHistoryStepCard({
  title,
  rightBadge,
  templateKey,
  result,
  apiKeyPlaintext,
  requestJsonText,
  resultJsonText,
  sseRawText,
  errorText,
  defaultOpen,
  copyText,
  className,
}: ClaudeModelValidationHistoryStepCardProps) {
  const evaluation = useMemo(
    () => evaluateClaudeValidation(templateKey, result),
    [templateKey, result]
  );
  const shouldAutoExpand = Boolean(errorText) || evaluation.overallPass === false;
  const [open, setOpen] = useState(Boolean(defaultOpen) || shouldAutoExpand);

  useEffect(() => {
    if (!shouldAutoExpand) return;
    setOpen(true);
  }, [shouldAutoExpand]);

  const executedRequestText = buildClaudeModelValidationRequestSnapshotTextFromResult(
    result,
    apiKeyPlaintext
  );
  const requestText = executedRequestText.trim()
    ? executedRequestText
    : normalizeCopyText(requestJsonText);
  const resultText = normalizeCopyText(resultJsonText);
  const sseText = normalizeCopyText(sseRawText);

  const canCopyRequest = Boolean(requestText.trim());
  const canCopyResultJson = Boolean(resultText.trim());
  const canCopySse = Boolean(sseText.trim());

  const metaText = (() => {
    if (!result) return null;
    const parts: string[] = [];
    const status =
      typeof result.status === "number" && Number.isFinite(result.status) ? result.status : null;
    const ms =
      typeof result.duration_ms === "number" && Number.isFinite(result.duration_ms)
        ? result.duration_ms
        : null;
    if (status != null) parts.push(`HTTP ${status}`);
    if (ms != null) parts.push(`${ms}ms`);
    return parts.length > 0 ? parts.join(" · ") : null;
  })();

  return (
    <details
      open={open}
      onToggle={(e) => setOpen((e.currentTarget as HTMLDetailsElement).open)}
      className={cn(
        "group/step rounded-2xl border border-slate-200/60 dark:border-slate-700/60 bg-white/50 dark:bg-slate-800/40 shadow-sm backdrop-blur-sm open:ring-2 open:ring-indigo-500/10 transition-all",
        className
      )}
    >
      <summary className="flex cursor-pointer items-start justify-between gap-3 px-4 py-3 select-none">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <div className="min-w-0 text-xs font-medium text-slate-700 dark:text-slate-300 truncate">
              {title}
            </div>
            {rightBadge ? <div className="shrink-0">{rightBadge}</div> : null}
          </div>
          {metaText ? (
            <div className="mt-0.5 text-[10px] text-slate-500 dark:text-slate-400 truncate">
              {metaText}
            </div>
          ) : null}
        </div>
        <ChevronDown className="mt-0.5 h-4 w-4 shrink-0 text-slate-400 dark:text-slate-500 transition-transform group-open/step:rotate-180" />
      </summary>

      <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3 space-y-3">
        {errorText ? (
          <div className="rounded bg-rose-50 dark:bg-rose-900/30 px-2 py-1 text-xs text-rose-700 dark:text-rose-400">
            {errorText}
          </div>
        ) : null}

        <details className="group rounded-xl border border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
          <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
            <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
              <Settings2 className="h-4 w-4" />
              <span>请求 JSON</span>
            </div>
            <div className="flex items-center gap-2">
              <Button
                onClick={(e) => {
                  stopDetailsToggle(e);
                  return void Promise.resolve(copyText(requestText, "已复制请求 JSON"));
                }}
                variant="ghost"
                size="sm"
                className="h-8 w-8 p-0"
                disabled={!canCopyRequest}
                title="复制请求 JSON"
                aria-label="复制请求 JSON"
              >
                <FileJson className="h-4 w-4" />
              </Button>
              <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
            </div>
          </summary>
          <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3">
            <Textarea
              mono
              readOnly
              className="h-[140px] resize-none text-[10px] leading-relaxed bg-white dark:bg-slate-900"
              value={requestText}
            />
          </div>
        </details>

        <ClaudeModelValidationResultPanel
          templateKey={templateKey}
          result={result}
          mode="compact"
        />

        <details className="group rounded-xl border border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
          <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
            <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
              <Activity className="h-4 w-4" />
              <span>响应原文</span>
            </div>
            <div className="flex items-center gap-2">
              <Button
                onClick={(e) => {
                  stopDetailsToggle(e);
                  return void Promise.resolve(copyText(resultText, "已复制 Result JSON"));
                }}
                variant="ghost"
                size="sm"
                className="h-8 w-8 p-0"
                disabled={!canCopyResultJson}
                title="复制 Result JSON"
                aria-label="复制 Result JSON"
              >
                <FileJson className="h-4 w-4" />
              </Button>
              <Button
                onClick={(e) => {
                  stopDetailsToggle(e);
                  return void Promise.resolve(copyText(sseText, "已复制 SSE 原文"));
                }}
                variant="ghost"
                size="sm"
                className="h-8 w-8 p-0"
                disabled={!canCopySse}
                title="复制 SSE 原文"
                aria-label="复制 SSE 原文"
              >
                <Copy className="h-4 w-4" />
              </Button>
              <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
            </div>
          </summary>

          <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3 space-y-3">
            <div className="space-y-2">
              <div className="text-[11px] font-semibold text-slate-700 dark:text-slate-300">
                Result JSON
              </div>
              <Textarea
                mono
                readOnly
                className="h-[160px] resize-none text-[10px] leading-relaxed bg-white dark:bg-slate-900"
                value={resultText || ""}
              />
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between gap-2">
                <div className="text-[11px] font-semibold text-slate-700 dark:text-slate-300">
                  SSE 原文
                </div>
                <Button
                  onClick={(e) => {
                    stopDetailsToggle(e);
                    return void Promise.resolve(copyText(sseText, "已复制 SSE 原文"));
                  }}
                  variant="ghost"
                  size="sm"
                  className="h-8 w-8 p-0"
                  disabled={!canCopySse}
                  title="复制 SSE 原文"
                  aria-label="复制 SSE 原文"
                >
                  <Copy className="h-4 w-4" />
                </Button>
              </div>
              <pre className="custom-scrollbar max-h-60 overflow-auto rounded-lg bg-slate-950 p-4 font-mono text-[10px] leading-relaxed text-slate-300">
                {sseText ? (
                  sseText
                ) : (
                  <span className="text-slate-600 dark:text-slate-400 italic">
                    // 暂无 SSE 数据
                  </span>
                )}
              </pre>
            </div>
          </div>
        </details>
      </div>
    </details>
  );
}

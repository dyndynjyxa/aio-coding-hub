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

import type { MouseEvent, ReactNode } from "react";
import type { ClaudeModelValidationResult } from "../services/claudeModelValidation";
import type { ClaudeValidationTemplateKey } from "../services/claudeValidationTemplates";
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
  requestJsonText: string;
  resultJsonText: string;
  sseRawText: string;
  errorText?: string | null;
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
  requestJsonText,
  resultJsonText,
  sseRawText,
  errorText,
  copyText,
  className,
}: ClaudeModelValidationHistoryStepCardProps) {
  const requestText = normalizeCopyText(requestJsonText);
  const resultText = normalizeCopyText(resultJsonText);
  const sseText = normalizeCopyText(sseRawText);

  const canCopyRequest = Boolean(requestText.trim());
  const canCopyResultJson = Boolean(resultText.trim());
  const canCopySse = Boolean(sseText.trim());

  return (
    <div className={cn("space-y-3", className)}>
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0 text-xs font-medium text-slate-700 truncate">{title}</div>
        {rightBadge ? <div className="shrink-0">{rightBadge}</div> : null}
      </div>

      {errorText ? (
        <div className="rounded bg-rose-50 px-2 py-1 text-xs text-rose-700">{errorText}</div>
      ) : null}

      <details className="group rounded-xl border border-slate-200 bg-white shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 group-open:text-indigo-600">
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
            <ChevronDown className="h-4 w-4 text-slate-400 transition-transform group-open:rotate-180" />
          </div>
        </summary>
        <div className="border-t border-slate-100 px-4 py-3">
          <Textarea
            mono
            readOnly
            className="h-[140px] resize-none text-[10px] leading-relaxed bg-white"
            value={requestText}
          />
        </div>
      </details>

      <ClaudeModelValidationResultPanel templateKey={templateKey} result={result} />

      <details className="group rounded-xl border border-slate-200 bg-white shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 group-open:text-indigo-600">
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
            <ChevronDown className="h-4 w-4 text-slate-400 transition-transform group-open:rotate-180" />
          </div>
        </summary>

        <div className="border-t border-slate-100 px-4 py-3 space-y-3">
          <div className="space-y-2">
            <div className="text-[11px] font-semibold text-slate-700">Result JSON</div>
            <Textarea
              mono
              readOnly
              className="h-[160px] resize-none text-[10px] leading-relaxed bg-white"
              value={resultText || ""}
            />
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between gap-2">
              <div className="text-[11px] font-semibold text-slate-700">SSE 原文</div>
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
              {sseText ? sseText : <span className="text-slate-600 italic">// 暂无 SSE 数据</span>}
            </pre>
          </div>
        </div>
      </details>
    </div>
  );
}

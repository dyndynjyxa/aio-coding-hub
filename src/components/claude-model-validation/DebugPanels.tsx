import type { ClaudeValidationTemplateKey } from "../../services/claude/claudeValidationTemplates";
import { getClaudeValidationTemplate } from "../../services/claude/claudeValidationTemplates";
import type { ClaudeModelValidationResult } from "../../services/claude/claudeModelValidation";
import { Button } from "../../ui/Button";
import { Textarea } from "../../ui/Textarea";
import {
  Settings2,
  ChevronDown,
  Activity,
  Copy,
  FileJson,
  CheckCircle2,
  XCircle,
} from "lucide-react";

import type { SuiteSummary } from "./types";
import { getTemplateDisplayLabel, stopDetailsToggle } from "./helpers";
import { OutcomePill } from "./OutcomePill";

export function SuiteDebugPanel({
  suiteSummary,
  copyTextOrToast,
}: {
  suiteSummary: SuiteSummary;
  copyTextOrToast: (text: string, okMessage: string) => Promise<void>;
}) {
  return (
    <div className="space-y-4">
      <details className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
            <FileJson className="h-4 w-4" />
            <span>\u53ef\u590d\u5236\u603b\u7ed3\uff08\u7eaf\u6587\u672c\uff09</span>
          </div>
          <div className="flex items-center gap-2">
            <Button
              onClick={(e) => {
                stopDetailsToggle(e);
                return void Promise.resolve(
                  copyTextOrToast(
                    suiteSummary.plainText,
                    "\u5df2\u590d\u5236\u9a8c\u8bc1\u603b\u7ed3"
                  )
                );
              }}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              disabled={!suiteSummary.plainText.trim()}
              title="\u590d\u5236\u603b\u7ed3"
              aria-label="\u590d\u5236\u603b\u7ed3"
            >
              <Copy className="h-4 w-4" />
            </Button>
            <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
          </div>
        </summary>
        <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3">
          <Textarea
            mono
            readOnly
            className="h-[220px] resize-none text-[11px] leading-relaxed bg-white dark:bg-slate-900"
            value={suiteSummary.plainText}
          />
        </div>
      </details>

      <details className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
            <Settings2 className="h-4 w-4" />
            <span>\u6267\u884c\u6a21\u677f\uff08\u5168\u90e8\uff09</span>
          </div>
          <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
        </summary>
        <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3 space-y-2">
          {suiteSummary.templateRows.map((r) => (
            <div
              key={r.templateKey}
              className="flex items-center justify-between gap-3 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900/30 px-3 py-2"
              title={r.grade?.title ?? ""}
            >
              <div className="min-w-0">
                <div className="text-xs font-medium text-slate-800 dark:text-slate-200 truncate">
                  {r.label}
                </div>
                <div className="mt-0.5 text-[10px] text-slate-500 dark:text-slate-400">
                  {r.templateKey}
                </div>
              </div>
              <div className="shrink-0">
                {r.status === "done" ? (
                  <OutcomePill pass={r.overallPass} />
                ) : r.status === "missing" ? (
                  <span className="rounded bg-slate-100 dark:bg-slate-700 px-1.5 py-0.5 text-[10px] font-semibold text-slate-600 dark:text-slate-400">
                    \u672a\u8bb0\u5f55
                  </span>
                ) : r.status === "running" ? (
                  <span className="rounded bg-sky-100 dark:bg-sky-900/30 px-1.5 py-0.5 text-[10px] font-semibold text-sky-700 dark:text-sky-400">
                    \u6267\u884c\u4e2d
                  </span>
                ) : r.status === "error" ? (
                  <span className="rounded bg-rose-100 dark:bg-rose-900/30 px-1.5 py-0.5 text-[10px] font-semibold text-rose-700 dark:text-rose-400">
                    \u5931\u8d25
                  </span>
                ) : (
                  <span className="rounded bg-slate-100 dark:bg-slate-700 px-1.5 py-0.5 text-[10px] font-semibold text-slate-600 dark:text-slate-400">
                    \u5f85\u6267\u884c
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      </details>

      <details
        open
        className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all"
      >
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
            <Activity className="h-4 w-4" />
            <span>\u5b98\u65b9\u534f\u8bae\u68c0\u67e5\u70b9\uff08\u5168\u90e8\uff09</span>
          </div>
          <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
        </summary>
        <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3 grid grid-cols-1 sm:grid-cols-2 gap-2">
          {suiteSummary.protocol.map((p) => (
            <div
              key={p.key}
              className="flex items-start justify-between gap-3 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900/30 px-3 py-2"
              title={p.detail ?? ""}
            >
              <div className="flex items-start gap-2 min-w-0">
                {p.ok == null ? (
                  <div className="mt-0.5 h-3.5 w-3.5 shrink-0 rounded-full border border-slate-300 dark:border-slate-600 bg-slate-100 dark:bg-slate-700" />
                ) : p.ok ? (
                  <CheckCircle2 className="mt-0.5 h-3.5 w-3.5 text-emerald-500 shrink-0" />
                ) : (
                  <XCircle className="mt-0.5 h-3.5 w-3.5 text-rose-500 shrink-0" />
                )}
                <div className="min-w-0">
                  <div className="text-xs text-slate-800 dark:text-slate-200">
                    {p.label}
                    {!p.required ? (
                      <span className="ml-1 text-[10px] text-slate-400 dark:text-slate-500">
                        (\u53c2\u8003)
                      </span>
                    ) : null}
                  </div>
                  {p.detail ? (
                    <div className="mt-0.5 text-[10px] text-slate-500 dark:text-slate-400">
                      {p.detail}
                    </div>
                  ) : null}
                </div>
              </div>
            </div>
          ))}
        </div>
      </details>
    </div>
  );
}

export function NonSuiteDebugPanel({
  requestJson,
  setRequestJson,
  activeResult,
  activeResultTemplateKey,
  copyTextOrToast,
}: {
  requestJson: string;
  setRequestJson: (v: string) => void;
  activeResult: ClaudeModelValidationResult | null;
  activeResultTemplateKey: ClaudeValidationTemplateKey;
  copyTextOrToast: (text: string, okMessage: string) => Promise<void>;
}) {
  return (
    <div className="space-y-4">
      <details className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
            <Settings2 className="h-4 w-4" />
            <span>\u9ad8\u7ea7\u8bf7\u6c42\u914d\u7f6e</span>
          </div>
          <div className="flex items-center gap-2">
            <Button
              onClick={(e) => {
                stopDetailsToggle(e);
                return void copyTextOrToast(
                  requestJson ?? "",
                  "\u5df2\u590d\u5236\u8bf7\u6c42 JSON"
                );
              }}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              disabled={!(requestJson ?? "").trim()}
              title="\u590d\u5236\u8bf7\u6c42 JSON"
              aria-label="\u590d\u5236\u8bf7\u6c42 JSON"
            >
              <FileJson className="h-4 w-4" />
            </Button>
            <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
          </div>
        </summary>
        <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3">
          <Textarea
            mono
            className="h-[220px] resize-none text-xs leading-5 bg-white dark:bg-slate-900 shadow-sm focus:ring-indigo-500"
            value={requestJson}
            onChange={(e) => {
              setRequestJson(e.currentTarget.value);
            }}
            placeholder='{"template_key":"official_max_tokens_5","headers":{...},"body":{...},"expect":{...}}'
          />
        </div>
      </details>

      <details className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
            <Activity className="h-4 w-4" />
            <span>SSE \u6d41\u5f0f\u54cd\u5e94\u9884\u89c8</span>
          </div>
          <div className="flex items-center gap-2">
            <Button
              onClick={(e) => {
                stopDetailsToggle(e);
                return void copyTextOrToast(
                  activeResult?.raw_excerpt ?? "",
                  "\u5df2\u590d\u5236 SSE \u539f\u6587"
                );
              }}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              disabled={!(activeResult?.raw_excerpt ?? "").trim()}
              title="\u590d\u5236 SSE \u539f\u6587"
              aria-label="\u590d\u5236 SSE \u539f\u6587"
            >
              <Copy className="h-4 w-4" />
            </Button>
            <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
          </div>
        </summary>
        <div className="border-t border-slate-100 dark:border-slate-700 p-0">
          <pre className="custom-scrollbar max-h-60 overflow-auto bg-slate-950 p-4 font-mono text-[10px] leading-relaxed text-slate-300">
            <span className="text-slate-500 dark:text-slate-400">
              {(() => {
                const t = getClaudeValidationTemplate(activeResultTemplateKey);
                return `// SSE: ${getTemplateDisplayLabel(t)} (${t.key})`;
              })()}
              {"\n"}
            </span>
            {activeResult?.raw_excerpt || (
              <span className="text-slate-600 dark:text-slate-400 italic">
                // \u6682\u65e0 SSE \u6570\u636e
              </span>
            )}
          </pre>
        </div>
      </details>
    </div>
  );
}

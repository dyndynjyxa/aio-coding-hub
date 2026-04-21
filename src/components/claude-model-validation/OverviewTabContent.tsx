import { buildClaudeModelValidationRequestSnapshotTextFromResult } from "../../services/claude/claudeModelValidationRequestSnapshot";
import type { ClaudeValidationTemplateKey } from "../../services/claude/claudeValidationTemplates";
import type { ClaudeModelValidationResult } from "../../services/claude/claudeModelValidation";
import { ClaudeModelValidationResultPanel } from "../ClaudeModelValidationResultPanel";
import { Button } from "../../ui/Button";
import { Textarea } from "../../ui/Textarea";
import { cn } from "../../utils/cn";
import { Settings2, ChevronDown, Activity, Copy, FileJson } from "lucide-react";

import type { ClaudeModelValidationHistoryGroup, SuiteSummary } from "./types";
import {
  prettyJsonOrFallback,
  getTemplateDisplayTitle,
  stopDetailsToggle,
  gradeColorClass,
} from "./helpers";
import { OutcomePill } from "./OutcomePill";
import { SuiteSummaryCard } from "./SuiteSummaryCard";

export function OverviewTabContent({
  isCurrentSuite,
  isHistorySuite,
  currentSuiteSummary,
  historySuiteSummary,
  selectedHistoryGroup,
  selectedHistoryLatest,
  activeResult,
  activeResultTemplateKey,
  apiKeyPlaintext,
  copyTextOrToast,
}: {
  isCurrentSuite: boolean;
  isHistorySuite: boolean;
  currentSuiteSummary: SuiteSummary | null;
  historySuiteSummary: SuiteSummary | null;
  selectedHistoryGroup: ClaudeModelValidationHistoryGroup | null;
  selectedHistoryLatest: ClaudeModelValidationHistoryGroup["runs"][number] | null;
  activeResult: ClaudeModelValidationResult | null;
  activeResultTemplateKey: ClaudeValidationTemplateKey;
  apiKeyPlaintext: string | null;
  copyTextOrToast: (text: string, okMessage: string) => Promise<void>;
}) {
  if (isCurrentSuite) {
    return currentSuiteSummary ? (
      <SuiteSummaryCard summary={currentSuiteSummary} copyText={copyTextOrToast} />
    ) : (
      <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3 text-xs text-slate-600 dark:text-slate-400">
        \u6682\u65e0\u7efc\u5408\u603b\u7ed3\uff08\u6267\u884c\u540e\u751f\u6210\uff09\u3002
      </div>
    );
  }
  if (isHistorySuite) {
    return historySuiteSummary ? (
      <SuiteSummaryCard summary={historySuiteSummary} copyText={copyTextOrToast} />
    ) : (
      <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3 text-xs text-slate-600 dark:text-slate-400">
        \u6682\u65e0\u7efc\u5408\u603b\u7ed3\uff08\u5386\u53f2\u6570\u636e\u4e0d\u8db3\uff09\u3002
      </div>
    );
  }
  if (selectedHistoryGroup && selectedHistoryLatest) {
    return (
      <SingleHistoryOverview
        selectedHistoryLatest={selectedHistoryLatest}
        apiKeyPlaintext={apiKeyPlaintext}
        copyTextOrToast={copyTextOrToast}
      />
    );
  }
  if (!selectedHistoryGroup && !selectedHistoryLatest) {
    return (
      <ClaudeModelValidationResultPanel
        templateKey={activeResultTemplateKey}
        result={activeResult}
        mode="compact"
      />
    );
  }
  return (
    <div className="flex h-40 items-center justify-center text-xs text-slate-400 dark:text-slate-500">
      \u6682\u65e0\u5386\u53f2\u6570\u636e
    </div>
  );
}

function SingleHistoryOverview({
  selectedHistoryLatest,
  apiKeyPlaintext,
  copyTextOrToast,
}: {
  selectedHistoryLatest: ClaudeModelValidationHistoryGroup["runs"][number];
  apiKeyPlaintext: string | null;
  copyTextOrToast: (text: string, okMessage: string) => Promise<void>;
}) {
  const ev = selectedHistoryLatest.evaluation;
  const result = selectedHistoryLatest.run.parsed_result;
  const grade = ev.grade;
  const evidenceGrade =
    grade && grade.label !== "\u901a\u8fc7" && grade.label !== "\u672a\u901a\u8fc7" ? grade : null;

  const evidencePill = evidenceGrade ? (
    <span
      className={cn(
        "rounded px-2 py-0.5 text-[10px] font-semibold",
        gradeColorClass(evidenceGrade.level)
      )}
      title={evidenceGrade.title}
    >
      \u8bc1\u636e {evidenceGrade.level} \u00b7 {evidenceGrade.label}
    </span>
  ) : null;

  const requestText = (() => {
    const executed = buildClaudeModelValidationRequestSnapshotTextFromResult(
      result,
      apiKeyPlaintext
    );
    return executed.trim() ? executed : (selectedHistoryLatest.run.request_json ?? "");
  })();
  const resultText = prettyJsonOrFallback(selectedHistoryLatest.run.result_json ?? "");
  const sseText = result?.raw_excerpt ?? "";

  const meta = (() => {
    const parts: string[] = [];
    if (typeof result?.status === "number") parts.push(`HTTP ${result.status}`);
    if (typeof result?.duration_ms === "number") parts.push(`${result.duration_ms}ms`);
    return parts.length > 0 ? parts.join(" \u00b7 ") : null;
  })();

  return (
    <div className="space-y-3">
      <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              \u9a8c\u8bc1\uff1a{getTemplateDisplayTitle(ev.template)}
            </div>
            <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400 truncate">
              {meta ? `${meta} \u00b7 ` : ""}
              {ev.templateKey}
            </div>
          </div>
          <div className="shrink-0 flex items-center gap-2">
            <OutcomePill pass={ev.overallPass} />
            {evidencePill}
            <Button
              onClick={(e) => {
                stopDetailsToggle(e);
                return void copyTextOrToast(requestText, "\u5df2\u590d\u5236\u8bf7\u6c42 JSON");
              }}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              disabled={!requestText.trim()}
              title="\u590d\u5236\u8bf7\u6c42 JSON"
              aria-label="\u590d\u5236\u8bf7\u6c42 JSON"
            >
              <FileJson className="h-4 w-4" />
            </Button>
            <Button
              onClick={(e) => {
                stopDetailsToggle(e);
                return void copyTextOrToast(resultText, "\u5df2\u590d\u5236 Result JSON");
              }}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              disabled={!resultText.trim()}
              title="\u590d\u5236 Result JSON"
              aria-label="\u590d\u5236 Result JSON"
            >
              <FileJson className="h-4 w-4" />
            </Button>
            <Button
              onClick={(e) => {
                stopDetailsToggle(e);
                return void copyTextOrToast(sseText, "\u5df2\u590d\u5236 SSE \u539f\u6587");
              }}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              disabled={!sseText.trim()}
              title="\u590d\u5236 SSE \u539f\u6587"
              aria-label="\u590d\u5236 SSE \u539f\u6587"
            >
              <Copy className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>
      <ClaudeModelValidationResultPanel
        templateKey={ev.templateKey}
        result={result}
        mode="compact"
      />
      <details className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
            <Settings2 className="h-4 w-4" />
            <span>\u8bf7\u6c42 JSON</span>
          </div>
          <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
        </summary>
        <div className="border-t border-slate-100 dark:border-slate-700 px-4 py-3">
          <Textarea
            mono
            readOnly
            className="h-[160px] resize-none text-[10px] leading-relaxed bg-white dark:bg-slate-900"
            value={requestText}
          />
        </div>
      </details>
      <details className="group rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-sm open:ring-2 open:ring-indigo-500/10 transition-all">
        <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-300 group-open:text-indigo-600 dark:group-open:text-indigo-400">
            <Activity className="h-4 w-4" />
            <span>\u54cd\u5e94\u539f\u6587</span>
          </div>
          <ChevronDown className="h-4 w-4 text-slate-400 dark:text-slate-500 transition-transform group-open:rotate-180" />
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
              value={resultText}
            />
          </div>
          <div className="space-y-2">
            <div className="text-[11px] font-semibold text-slate-700 dark:text-slate-300">
              SSE \u539f\u6587
            </div>
            <pre className="custom-scrollbar max-h-60 overflow-auto rounded-lg bg-slate-950 p-4 font-mono text-[10px] leading-relaxed text-slate-300">
              {sseText ? (
                sseText
              ) : (
                <span className="text-slate-600 dark:text-slate-400 italic">
                  // \u6682\u65e0 SSE \u6570\u636e
                </span>
              )}
            </pre>
          </div>
        </div>
      </details>
    </div>
  );
}

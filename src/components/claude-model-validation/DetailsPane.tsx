import {
  evaluateClaudeValidation,
  getClaudeTemplateApplicability,
  getClaudeValidationTemplate,
  type ClaudeValidationTemplateKey,
  DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY,
} from "../../services/claude/claudeValidationTemplates";
import { buildClaudeModelValidationRequestSnapshotTextFromResult } from "../../services/claude/claudeModelValidationRequestSnapshot";
import { ClaudeModelValidationResultPanel } from "../ClaudeModelValidationResultPanel";
import { ClaudeModelValidationHistoryStepCard } from "../ClaudeModelValidationHistoryStepCard";
import { Button } from "../../ui/Button";
import { Switch } from "../../ui/Switch";
import { TabList } from "../../ui/TabList";
import { Textarea } from "../../ui/Textarea";
import { cn } from "../../utils/cn";
import { formatUnixSeconds } from "../../utils/formatters";
import { Settings2, History, ChevronDown, Activity, Copy, FileJson } from "lucide-react";

import type { ClaudeModelValidationResult } from "../../services/claude/claudeModelValidation";
import type {
  ClaudeValidationSuiteStep,
  ClaudeModelValidationHistoryGroup,
  SuiteStepView,
  ValidationDetailsTab,
  SuiteSummary,
} from "./types";
import {
  prettyJsonOrFallback,
  getTemplateDisplayLabel,
  getTemplateDisplayTitle,
  stopDetailsToggle,
  gradeColorClass,
} from "./helpers";
import { OutcomePill } from "./OutcomePill";
import { OverviewTabContent } from "./OverviewTabContent";
import { SuiteDebugPanel, NonSuiteDebugPanel } from "./DebugPanels";

export type DetailsPaneProps = {
  suiteSteps: ClaudeValidationSuiteStep[];
  suiteProgress: { current: number; total: number; round: number; totalRounds: number } | null;
  suiteIssuesOnly: boolean;
  setSuiteIssuesOnly: (v: boolean) => void;
  suiteActiveStepIndex: number | null;
  setSuiteActiveStepIndex: (v: number | null) => void;
  detailsTab: ValidationDetailsTab;
  setDetailsTab: (tab: ValidationDetailsTab) => void;
  detailsTabItems: Array<{ key: ValidationDetailsTab; label: string; disabled?: boolean }>;
  selectedHistoryGroup: ClaudeModelValidationHistoryGroup | null;
  selectedHistoryLatest: ClaudeModelValidationHistoryGroup["runs"][number] | null;
  activeResult: ClaudeModelValidationResult | null;
  activeResultTemplateKey: ClaudeValidationTemplateKey;
  currentSuiteSummary: SuiteSummary | null;
  historySuiteSummary: SuiteSummary | null;
  hasSuiteContext: boolean;
  suiteHeaderMetaText: string | null;
  requestJson: string;
  setRequestJson: (v: string) => void;
  apiKeyPlaintext: string | null;
  templates: ReturnType<
    typeof import("../../services/claude/claudeValidationTemplates").listClaudeValidationTemplates
  >;
  copyTextOrToast: (text: string, okMessage: string) => Promise<void>;
};

export function DetailsPane(props: DetailsPaneProps) {
  const {
    suiteSteps,
    suiteProgress,
    suiteIssuesOnly,
    setSuiteIssuesOnly,
    suiteActiveStepIndex,
    setSuiteActiveStepIndex,
    detailsTab,
    setDetailsTab,
    detailsTabItems,
    selectedHistoryGroup,
    selectedHistoryLatest,
    activeResult,
    activeResultTemplateKey,
    currentSuiteSummary,
    historySuiteSummary,
    hasSuiteContext,
    suiteHeaderMetaText,
    requestJson,
    setRequestJson,
    apiKeyPlaintext,
    templates,
    copyTextOrToast,
  } = props;

  const isCurrentSuite = suiteSteps.length > 0 && !selectedHistoryGroup;
  const isHistorySuite = selectedHistoryGroup?.isSuite === true;
  const suiteSummary = isCurrentSuite
    ? currentSuiteSummary
    : isHistorySuite
      ? historySuiteSummary
      : null;

  const renderSuiteStepsMasterDetail = (allSteps: SuiteStepView[]) => {
    const visible = suiteIssuesOnly
      ? allSteps.filter((step) => {
          if (step.status === "error") return true;
          if (step.status === "running") return true;
          if (step.status === "missing") return true;
          if (step.status !== "done") return false;
          return step.evaluation.overallPass !== true;
        })
      : allSteps;

    if (suiteIssuesOnly && visible.length === 0) {
      return (
        <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3 text-xs text-slate-600 dark:text-slate-400">
          \u6682\u65e0\u5f02\u5e38\uff0c\u5df2\u9690\u85cf\u901a\u8fc7\u9879\uff08\u5173\u95ed\u201c\u53ea\u770b\u5f02\u5e38\u201d\u53ef\u67e5\u770b\u5168\u90e8\u6b65\u9aa4\uff09\u3002
        </div>
      );
    }

    const activeIndex = (() => {
      if (suiteActiveStepIndex != null && visible.some((s) => s.index === suiteActiveStepIndex)) {
        return suiteActiveStepIndex;
      }
      const running = visible.find((s) => s.status === "running");
      if (running) return running.index;
      const issue = visible.find(
        (s) =>
          s.status === "error" ||
          s.status === "missing" ||
          (s.status === "done" && s.evaluation.overallPass !== true)
      );
      if (issue) return issue.index;
      return visible[0]?.index ?? null;
    })();

    const activeStep =
      activeIndex != null ? (visible.find((s) => s.index === activeIndex) ?? null) : null;

    const statusBadge = (step: SuiteStepView) => {
      if (step.status === "done") return <OutcomePill pass={step.evaluation.overallPass} />;
      if (step.status === "running")
        return (
          <span className="rounded bg-sky-100 dark:bg-sky-900/30 px-1.5 py-0.5 text-[10px] font-semibold text-sky-700 dark:text-sky-400">
            \u6267\u884c\u4e2d
          </span>
        );
      if (step.status === "error")
        return (
          <span className="rounded bg-rose-100 dark:bg-rose-900/30 px-1.5 py-0.5 text-[10px] font-semibold text-rose-700 dark:text-rose-400">
            \u5931\u8d25
          </span>
        );
      if (step.status === "missing")
        return (
          <span className="rounded bg-slate-100 dark:bg-slate-700 px-1.5 py-0.5 text-[10px] font-semibold text-slate-600 dark:text-slate-400">
            \u672a\u8bb0\u5f55
          </span>
        );
      return (
        <span className="rounded bg-slate-100 dark:bg-slate-700 px-1.5 py-0.5 text-[10px] font-semibold text-slate-600 dark:text-slate-400">
          \u5f85\u6267\u884c
        </span>
      );
    };

    const evidenceBadge = (step: SuiteStepView) => {
      if (step.status !== "done") return null;
      const grade = step.evaluation.grade;
      if (!grade) return null;
      if (grade.label === "\u901a\u8fc7" || grade.label === "\u672a\u901a\u8fc7") return null;
      const hint = (() => {
        const label = grade.label ?? "";
        if (label.includes("\u7b2c\u4e00\u65b9") && label.includes("\u5f3a")) return "\u5f3a";
        if (label.includes("\u7b2c\u4e00\u65b9") && label.includes("\u4e2d")) return "\u4e2d";
        if (label.includes("\u5f31")) return "\u5f31";
        if (label.includes("\u98ce\u9669")) return "\u98ce\u9669";
        return label.replace(/[\uff08\uff09()]/g, "") || "\u2014";
      })();
      const cls = gradeColorClass(grade.level);
      return (
        <span
          className={cn("rounded px-2 py-0.5 text-[10px] font-semibold", cls)}
          title={grade.title}
        >
          {grade.level} {hint}
        </span>
      );
    };

    const stepMeta = (step: SuiteStepView) => {
      if (!step.result) return null;
      const parts: string[] = [];
      const status =
        typeof step.result.status === "number" && Number.isFinite(step.result.status)
          ? step.result.status
          : null;
      const ms =
        typeof step.result.duration_ms === "number" && Number.isFinite(step.result.duration_ms)
          ? step.result.duration_ms
          : null;
      if (status != null) parts.push(`HTTP ${status}`);
      if (ms != null) parts.push(`${ms}ms`);
      return parts.length > 0 ? parts.join(" \u00b7 ") : null;
    };

    const activeRequestText = (() => {
      if (!activeStep) return "";
      const executed = buildClaudeModelValidationRequestSnapshotTextFromResult(
        activeStep.result,
        apiKeyPlaintext
      );
      return executed.trim() ? executed : (activeStep.requestJsonText ?? "");
    })();
    const activeResultText = activeStep
      ? prettyJsonOrFallback(activeStep.resultJsonText ?? "")
      : "";
    const activeSseText = activeStep ? (activeStep.sseRawText ?? "") : "";

    return (
      <div className="grid gap-4 lg:grid-cols-5">
        <div className="lg:col-span-2">
          <div className="space-y-2">
            {visible.map((step) => {
              const isActive = step.index === activeIndex;
              const meta = stepMeta(step);
              return (
                <button
                  key={`${step.templateKey}_${step.index}`}
                  type="button"
                  onClick={() => setSuiteActiveStepIndex(step.index)}
                  className={cn(
                    "w-full text-left rounded-xl border px-3 py-2 transition-all",
                    isActive
                      ? "border-indigo-500 bg-indigo-50/50 dark:bg-indigo-900/20 shadow-sm ring-1 ring-indigo-500/20"
                      : "border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 hover:border-indigo-200 dark:hover:border-indigo-700 hover:shadow-sm"
                  )}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="text-xs font-semibold text-slate-800 dark:text-slate-200 truncate">
                        {step.index}/{step.total} \u00b7 {step.label}
                      </div>
                      <div className="mt-0.5 text-[10px] text-slate-500 dark:text-slate-400 truncate">
                        {meta ? `${meta} \u00b7 ` : ""}
                        {step.templateKey}
                      </div>
                    </div>
                    <div className="shrink-0 flex flex-col items-end gap-1">
                      {statusBadge(step)}
                      {evidenceBadge(step)}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        <div className="lg:col-span-3 space-y-3">
          {activeStep ? (
            <StepDetailPanel
              step={activeStep}
              stepMeta={stepMeta}
              requestText={activeRequestText}
              resultText={activeResultText}
              sseText={activeSseText}
              copyTextOrToast={copyTextOrToast}
            />
          ) : (
            <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3 text-xs text-slate-600 dark:text-slate-400">
              \u8bf7\u9009\u62e9\u4e00\u4e2a\u6b65\u9aa4\u67e5\u770b\u8be6\u60c5\u3002
            </div>
          )}
        </div>
      </div>
    );
  };

  const renderCurrentSuiteSteps = () => {
    if (suiteSteps.length === 0) return null;
    const total = suiteSteps.length;
    const stepViews: SuiteStepView[] = suiteSteps.map((s) => ({
      index: s.index,
      total,
      templateKey: s.templateKey,
      label: s.label,
      status: s.status,
      evaluation: evaluateClaudeValidation(s.templateKey, s.result),
      result: s.result,
      requestJsonText: s.request_json ?? "",
      resultJsonText: s.result_json ?? "",
      sseRawText: s.result?.raw_excerpt ?? "",
      errorText: s.error,
    }));
    return renderSuiteStepsMasterDetail(stepViews);
  };

  const renderHistorySuiteSteps = () => {
    if (!selectedHistoryGroup?.isSuite) return null;
    const expectedTotal = selectedHistoryGroup.expectedTotal;
    const expectedKeys = templates
      .filter((t) => getClaudeTemplateApplicability(t, selectedHistoryGroup.modelName).applicable)
      .map((t) => t.key);

    const byIndex = new Map<number, (typeof selectedHistoryGroup.runs)[number]>();
    for (const r of selectedHistoryGroup.runs) {
      const idx = r.meta.suiteStepIndex ?? 0;
      if (!Number.isFinite(idx) || idx <= 0) continue;
      const prev = byIndex.get(idx);
      if (!prev || r.run.id > prev.run.id) byIndex.set(idx, r);
    }

    const stepViews: SuiteStepView[] = [];
    for (let idx = 1; idx <= expectedTotal; idx += 1) {
      const step = byIndex.get(idx) ?? null;
      const expectedKey = expectedKeys[idx - 1] ?? step?.evaluation.templateKey;
      const templateKeyForUi = (expectedKey ??
        DEFAULT_CLAUDE_VALIDATION_TEMPLATE_KEY) as ClaudeValidationTemplateKey;
      const template = getClaudeValidationTemplate(templateKeyForUi);
      stepViews.push({
        index: idx,
        total: expectedTotal,
        templateKey: templateKeyForUi,
        label: getTemplateDisplayLabel(template),
        status: step ? "done" : "missing",
        evaluation: step ? step.evaluation : evaluateClaudeValidation(templateKeyForUi, null),
        result: step?.run.parsed_result ?? null,
        requestJsonText: step?.run.request_json ?? "",
        resultJsonText: prettyJsonOrFallback(step?.run.result_json ?? ""),
        sseRawText: step?.run.parsed_result?.raw_excerpt ?? "",
        errorText: step
          ? null
          : "\u8be5\u6b65\u9aa4\u672a\u51fa\u73b0\u5728\u5386\u53f2\u4e2d\uff1a\u53ef\u80fd\u662f\u5386\u53f2\u5199\u5165\u5931\u8d25\u3001\u88ab\u6e05\u7a7a\uff0c\u6216\u88ab\u4fdd\u7559\u6570\u91cf\u4e0a\u9650\u6dd8\u6c70\u3002\u8bf7\u5728\u201c\u5f53\u524d\u8fd0\u884c\u201d\u67e5\u770b\u5b8c\u6574\u8bca\u65ad\u3002",
      });
    }
    return renderSuiteStepsMasterDetail(stepViews);
  };

  return (
    <div className="flex flex-col gap-4 h-full min-h-0 min-w-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
      <div className="sticky top-0 z-20 bg-white/90 dark:bg-slate-900/80 backdrop-blur border-b border-slate-100 dark:border-slate-700 pb-3 pt-2">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div className="min-w-0">
            <div className="flex items-center gap-2 text-sm font-semibold text-slate-900 dark:text-slate-100">
              {suiteSteps.length > 0 && !selectedHistoryGroup ? (
                <>
                  <Activity className="h-4 w-4 text-sky-500" />
                  Running
                </>
              ) : selectedHistoryGroup ? (
                <>
                  <History className="h-4 w-4 text-slate-500 dark:text-slate-400" />
                  Details
                </>
              ) : (
                <>
                  <Settings2 className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                  Ready
                </>
              )}
            </div>
            <div className="mt-1 truncate text-xs text-slate-500 dark:text-slate-400">
              {selectedHistoryGroup
                ? selectedHistoryGroup.isSuite
                  ? `Suite #${selectedHistoryGroup.latestRunId} \u00b7 ${formatUnixSeconds(selectedHistoryGroup.createdAt)}${suiteHeaderMetaText ? ` \u00b7 ${suiteHeaderMetaText}` : ""}`
                  : `Log #${selectedHistoryGroup.latestRunId} \u00b7 ${formatUnixSeconds(selectedHistoryGroup.createdAt)}`
                : suiteSteps.length > 0
                  ? `Running ${suiteProgress?.current ?? 0}/${suiteSteps.length} templates...${suiteHeaderMetaText ? ` \u00b7 ${suiteHeaderMetaText}` : ""}`
                  : activeResult
                    ? "Latest (Unsaved)"
                    : "Waiting..."}
            </div>
          </div>
        </div>

        <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
          <TabList
            ariaLabel="\u9a8c\u8bc1\u8be6\u60c5\u89c6\u56fe"
            items={detailsTabItems}
            value={detailsTab}
            onChange={setDetailsTab}
            className="shrink-0"
            buttonClassName="!py-1.5"
          />
          {hasSuiteContext && detailsTab === "steps" ? (
            <div className="flex items-center gap-2">
              <span className="text-xs text-slate-500 dark:text-slate-400">
                \u53ea\u770b\u5f02\u5e38
              </span>
              <Switch size="sm" checked={suiteIssuesOnly} onCheckedChange={setSuiteIssuesOnly} />
            </div>
          ) : null}
        </div>
      </div>

      {/* Tab Content */}
      {detailsTab === "overview" ? (
        <OverviewTabContent
          isCurrentSuite={isCurrentSuite}
          isHistorySuite={isHistorySuite}
          currentSuiteSummary={currentSuiteSummary}
          historySuiteSummary={historySuiteSummary}
          selectedHistoryGroup={selectedHistoryGroup}
          selectedHistoryLatest={selectedHistoryLatest}
          activeResult={activeResult}
          activeResultTemplateKey={activeResultTemplateKey}
          apiKeyPlaintext={apiKeyPlaintext}
          copyTextOrToast={copyTextOrToast}
        />
      ) : detailsTab === "steps" ? (
        isCurrentSuite ? (
          renderCurrentSuiteSteps()
        ) : isHistorySuite ? (
          renderHistorySuiteSteps()
        ) : (
          <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3 text-xs text-slate-600 dark:text-slate-400">
            \u5f53\u524d\u4e0d\u662f\u6d4b\u8bd5\u5957\u4ef6\u89c6\u56fe\uff08\u8bf7\u9009\u62e9\u4e00\u6761
            suite \u5386\u53f2\u8bb0\u5f55\u6216\u8fd0\u884c\u5957\u4ef6\uff09\u3002
          </div>
        )
      ) : /* debug tab */ hasSuiteContext && suiteSummary ? (
        <SuiteDebugPanel suiteSummary={suiteSummary} copyTextOrToast={copyTextOrToast} />
      ) : !selectedHistoryGroup && suiteSteps.length === 0 ? (
        <NonSuiteDebugPanel
          requestJson={requestJson}
          setRequestJson={setRequestJson}
          activeResult={activeResult}
          activeResultTemplateKey={activeResultTemplateKey}
          copyTextOrToast={copyTextOrToast}
        />
      ) : selectedHistoryGroup && selectedHistoryLatest ? (
        <ClaudeModelValidationHistoryStepCard
          title={`\u9a8c\u8bc1\uff1a${getTemplateDisplayTitle(selectedHistoryLatest.evaluation.template)}`}
          rightBadge={<OutcomePill pass={selectedHistoryLatest.evaluation.overallPass} />}
          templateKey={selectedHistoryLatest.evaluation.templateKey}
          result={selectedHistoryLatest.run.parsed_result}
          apiKeyPlaintext={apiKeyPlaintext}
          requestJsonText={selectedHistoryLatest.run.request_json ?? ""}
          resultJsonText={prettyJsonOrFallback(selectedHistoryLatest.run.result_json ?? "")}
          sseRawText={selectedHistoryLatest.run.parsed_result?.raw_excerpt ?? ""}
          defaultOpen={true}
          copyText={copyTextOrToast}
        />
      ) : (
        <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3 text-xs text-slate-600 dark:text-slate-400">
          \u6682\u65e0\u8c03\u8bd5\u4fe1\u606f\u3002
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// StepDetailPanel (used in suite steps master-detail)
// ---------------------------------------------------------------------------

function StepDetailPanel({
  step,
  stepMeta,
  requestText,
  resultText,
  sseText,
  copyTextOrToast,
}: {
  step: SuiteStepView;
  stepMeta: (step: SuiteStepView) => string | null;
  requestText: string;
  resultText: string;
  sseText: string;
  copyTextOrToast: (text: string, okMessage: string) => Promise<void>;
}) {
  return (
    <>
      <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-4 py-3">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              \u6b65\u9aa4 {step.index}/{step.total}\uff1a{step.label}
            </div>
            <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400 truncate">
              {stepMeta(step) ? `${stepMeta(step)} \u00b7 ` : ""}
              {step.templateKey}
            </div>
          </div>
          <div className="shrink-0 flex items-center gap-2">
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
        {step.errorText ? (
          <div className="mt-3 rounded bg-rose-50 dark:bg-rose-900/30 px-3 py-2 text-xs text-rose-700 dark:text-rose-400">
            {step.errorText}
          </div>
        ) : null}
      </div>
      <ClaudeModelValidationResultPanel
        templateKey={step.templateKey}
        result={step.result}
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
              value={resultText || ""}
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
    </>
  );
}

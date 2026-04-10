import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { cn } from "../../utils/cn";
import { formatUnixSeconds } from "../../utils/formatters";
import { History, Trash2, RefreshCw, Cpu, ChevronRight } from "lucide-react";

import type { ClaudeModelValidationHistoryGroup } from "./types";
import type { ProviderSummary } from "../../services/providers/providers";
import { getTemplateDisplayLabel, gradeColorClass } from "./helpers";

export function HistoryListPanel({
  provider,
  historyAvailable,
  historyLoading,
  historyGroups,
  selectedHistoryKey,
  historyClearing,
  onSelectGroup,
  onRefresh,
  onClear,
}: {
  provider: ProviderSummary;
  historyAvailable: boolean | null;
  historyLoading: boolean;
  historyGroups: ClaudeModelValidationHistoryGroup[];
  selectedHistoryKey: string | null;
  historyClearing: boolean;
  onSelectGroup: (key: string) => void;
  onRefresh: () => void;
  onClear: () => void;
}) {
  return (
    <div className="flex flex-col gap-4 h-full min-h-0 w-full lg:flex-[0_1_420px] lg:max-w-[420px]">
      <Card padding="none" className="flex h-full flex-col overflow-hidden">
        <div className="flex items-center justify-between border-b border-slate-100 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-800/50 px-4 py-3">
          <div className="flex items-center gap-2">
            <History className="h-4 w-4 text-slate-500 dark:text-slate-400" />
            <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              History
            </span>
          </div>
          <div className="flex items-center gap-1">
            <Button
              onClick={onRefresh}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              disabled={historyLoading || historyAvailable === false}
              title="\u5237\u65b0"
            >
              <RefreshCw className={cn("h-4 w-4", historyLoading && "animate-spin")} />
            </Button>
            <Button
              onClick={() => {
                if (!provider) return;
                onClear();
              }}
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0 text-rose-500 hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-900/30"
              disabled={historyLoading || historyAvailable === false || historyClearing}
              title="\u6e05\u7a7a\u5386\u53f2"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div className="flex-1 overflow-hidden">
          {historyAvailable === false ? (
            <div className="flex h-40 flex-col items-center justify-center gap-2 text-slate-400 dark:text-slate-500">
              <Cpu className="h-8 w-8 text-slate-200 dark:text-slate-600" />
              <span className="text-xs">\u4ec5\u9650\u684c\u9762\u7aef</span>
            </div>
          ) : historyLoading && historyGroups.length === 0 ? (
            <div className="flex h-40 items-center justify-center text-xs text-slate-400 dark:text-slate-500">
              \u52a0\u8f7d\u4e2d...
            </div>
          ) : historyGroups.length === 0 ? (
            <div className="flex h-40 flex-col items-center justify-center gap-2 text-slate-400 dark:text-slate-500">
              <History className="h-8 w-8 text-slate-200 dark:text-slate-600" />
              <span className="text-xs">No History</span>
            </div>
          ) : (
            <div className="custom-scrollbar h-full overflow-y-auto p-3 space-y-2">
              {historyGroups.map((group) => (
                <HistoryGroupButton
                  key={group.key}
                  group={group}
                  active={group.key === selectedHistoryKey}
                  onSelect={() => onSelectGroup(group.key)}
                />
              ))}
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}

function HistoryGroupButton({
  group,
  active,
  onSelect,
}: {
  group: ClaudeModelValidationHistoryGroup;
  active: boolean;
  onSelect: () => void;
}) {
  const mentionsBedrock = group.runs.some((r) => {
    const signals = r.run.parsed_result?.signals;
    return Boolean(
      signals &&
      typeof signals === "object" &&
      (signals as Record<string, unknown>).mentions_amazon_bedrock
    );
  });

  const statusPill = (() => {
    if (!group.isSuite) {
      return {
        text: group.overallPass ? "\u901a\u8fc7" : "\u672a\u901a\u8fc7",
        cls: group.overallPass
          ? "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400"
          : "bg-rose-100 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400",
      };
    }
    if (group.overallPass) {
      return {
        text: `\u901a\u8fc7 ${group.passCount}/${group.expectedTotal}`,
        cls: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400",
      };
    }
    if (group.missingCount > 0 && group.failCount === 0) {
      return {
        text: `\u7f3a\u5931 ${group.missingCount}`,
        cls: "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-200",
      };
    }
    return {
      text: `\u672a\u901a\u8fc7 ${group.passCount}/${group.expectedTotal}`,
      cls: "bg-rose-100 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400",
    };
  })();

  const evidencePill = (() => {
    if (!group.grade) return null;
    if (group.grade.label === "\u901a\u8fc7" || group.grade.label === "\u672a\u901a\u8fc7")
      return null;

    const hint = (() => {
      const label = group.grade.label ?? "";
      if (label.includes("\u7b2c\u4e00\u65b9") && label.includes("\u5f3a")) return "\u5f3a";
      if (label.includes("\u7b2c\u4e00\u65b9") && label.includes("\u4e2d")) return "\u4e2d";
      if (label.includes("\u5f31")) return "\u5f31";
      if (label.includes("\u98ce\u9669")) return "\u98ce\u9669";
      return label.replace(/[\uff08\uff09()]/g, "") || "\u2014";
    })();

    const cls = gradeColorClass(group.grade.level);

    return (
      <span
        className={cn("rounded px-2 py-0.5 text-[10px] font-semibold", cls)}
        title={group.grade.title}
      >
        {group.grade.level} {hint}
      </span>
    );
  })();

  const metaText = (() => {
    const parts = [`#${group.latestRunId}`, formatUnixSeconds(group.createdAt)];
    if (group.isSuite) parts.unshift("Suite");
    return parts.join(" \u00b7 ");
  })();

  const titleText = (() => {
    if (group.isSuite) return group.modelName;
    const latest = group.runs[group.runs.length - 1]?.evaluation.template;
    if (!latest) return group.modelName;
    return getTemplateDisplayLabel(latest);
  })();

  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "group w-full text-left rounded-xl border px-3 py-2 transition-all",
        active
          ? "border-indigo-500 bg-indigo-50/50 dark:bg-indigo-900/20 shadow-sm ring-1 ring-indigo-500/20"
          : "border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 hover:border-indigo-200 dark:hover:border-indigo-700 hover:shadow-sm"
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <div className="flex items-center gap-2 min-w-0">
                <span className="text-xs font-semibold text-slate-800 dark:text-slate-200 truncate">
                  {titleText}
                </span>
                {!group.isSuite ? (
                  <span className="rounded bg-slate-100 dark:bg-slate-700 px-2 py-0.5 text-[10px] font-medium text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-slate-600 shrink-0">
                    {group.modelName}
                  </span>
                ) : (
                  <span className="rounded bg-slate-100 dark:bg-slate-700 px-2 py-0.5 text-[10px] font-medium text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-slate-600 shrink-0">
                    Suite
                  </span>
                )}
                {mentionsBedrock ? (
                  <span
                    className="rounded bg-slate-100 dark:bg-slate-700 px-2 py-0.5 text-[10px] font-medium text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-slate-600 shrink-0"
                    title="signals.mentions_amazon_bedrock=true"
                  >
                    Bedrock
                  </span>
                ) : null}
              </div>
              <div className="mt-1 text-[10px] text-slate-500 dark:text-slate-400 truncate">
                {metaText}
              </div>
            </div>

            <div className="shrink-0 flex flex-col items-end gap-1">
              <span className={cn("rounded px-2 py-0.5 text-[10px] font-semibold", statusPill.cls)}>
                {statusPill.text}
              </span>
              {evidencePill}
            </div>
          </div>
        </div>

        <ChevronRight
          className={cn(
            "mt-0.5 h-4 w-4 text-slate-300 transition-transform",
            active && "text-indigo-400"
          )}
        />
      </div>
    </button>
  );
}

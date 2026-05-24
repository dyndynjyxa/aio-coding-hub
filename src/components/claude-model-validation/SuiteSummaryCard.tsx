import { cn } from "../../utils/cn";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { XCircle, Copy } from "lucide-react";
import type { SuiteSummary } from "./types";
import { gradeColorClass } from "./helpers";

export function SuiteSummaryCard({
  summary,
  copyText,
}: {
  summary: SuiteSummary;
  copyText: (text: string, okMessage: string) => Promise<void> | void;
}) {
  const protocolBadge = (() => {
    if (summary.isRunning) {
      return {
        text: "协议：执行中",
        cls: "bg-sky-100 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400",
      };
    }
    if (summary.overallPass === true) {
      return {
        text: "协议：通过",
        cls: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400",
      };
    }
    if (summary.overallPass === false) {
      return {
        text: "协议：不通过",
        cls: "bg-rose-100 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400",
      };
    }
    return {
      text: "协议：未知",
      cls: "bg-secondary text-muted-foreground dark:bg-secondary dark:text-muted-foreground",
    };
  })();

  const metaLine = (() => {
    const nonPass = summary.stats.fail + summary.stats.error + summary.stats.missing;
    const parts = [
      `模型 ${summary.modelName}`,
      `完成 ${summary.stats.done}/${summary.stats.total}`,
      `通过 ${summary.stats.pass}`,
      `未通过 ${nonPass}`,
    ];
    if (summary.stats.missing > 0) parts.push(`缺失 ${summary.stats.missing}`);
    return parts.join(" \u00b7 ");
  })();

  const evidenceGrade =
    summary.grade && summary.grade.label !== "通过" && summary.grade.label !== "未通过"
      ? summary.grade
      : null;

  const evidenceBadge = evidenceGrade ? (
    <span
      className={cn(
        "rounded px-2 py-1 text-xs font-semibold",
        gradeColorClass(evidenceGrade.level)
      )}
      title={evidenceGrade.title}
    >
      证据：{evidenceGrade.level} {evidenceGrade.label}
    </span>
  ) : null;

  const protocolBox = (() => {
    const required = summary.protocol.filter((p) => p.required);
    if (required.length === 0) return null;
    const nonOk = required.filter((p) => p.ok !== true);

    if (nonOk.length === 0) {
      return (
        <div className="rounded-xl border border-emerald-100 dark:border-emerald-800 bg-emerald-50/60 dark:bg-emerald-900/20 px-4 py-3">
          <div className="text-xs font-semibold text-emerald-800 dark:text-emerald-300">
            协议检查点
          </div>
          <div className="mt-1 text-xs text-emerald-700 dark:text-emerald-400">
            全部满足（{required.length}/{required.length}）
          </div>
        </div>
      );
    }

    const shown = nonOk.slice(0, 4);
    const rest = Math.max(0, nonOk.length - shown.length);
    const hasFail = nonOk.some((p) => p.ok === false);
    const boxCls = hasFail
      ? "border-rose-200 dark:border-rose-800 bg-rose-50/60 dark:bg-rose-900/20"
      : "border-amber-200 dark:border-amber-800 bg-amber-50/60 dark:bg-amber-900/20";
    const titleCls = hasFail
      ? "text-rose-700 dark:text-rose-300"
      : "text-amber-800 dark:text-amber-200";

    return (
      <div className={cn("rounded-xl border px-4 py-3", boxCls)}>
        <div className={cn("text-xs font-semibold", titleCls)}>协议检查点（未满足/无法判断）</div>
        <div className="mt-2 space-y-1.5">
          {shown.map((p) => (
            <div key={p.key} className="flex items-start gap-2 text-xs">
              {p.ok === false ? (
                <XCircle className="mt-0.5 h-3.5 w-3.5 text-rose-500 shrink-0" />
              ) : (
                <div className="mt-0.5 h-3.5 w-3.5 shrink-0 rounded-full border border-amber-300 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30" />
              )}
              <div className="min-w-0">
                <div className="text-foreground">{p.label}</div>
                {p.detail ? (
                  <div className="mt-0.5 text-[10px] text-muted-foreground">{p.detail}</div>
                ) : null}
              </div>
            </div>
          ))}
          {rest > 0 ? (
            <div className="text-[10px] text-muted-foreground">其余 {rest} 项详见"调试"页。</div>
          ) : null}
        </div>
      </div>
    );
  })();

  const evidenceBox = (() => {
    if (!evidenceGrade) return null;
    const keys = new Set([
      "thinking_output",
      "signature",
      "signature_roundtrip",
      "signature_tamper",
      "cross_provider_signature",
      "thinking_preserved",
      "cache_detail",
      "cache_read_hit",
    ]);
    const items = summary.protocol.filter((p) => keys.has(p.key));
    if (items.length === 0) return null;

    const nonOk = items.filter((p) => p.ok !== true);
    if (nonOk.length === 0) {
      return (
        <div className="rounded-xl border border-emerald-100 dark:border-emerald-800 bg-emerald-50/60 dark:bg-emerald-900/20 px-4 py-3">
          <div className="text-xs font-semibold text-emerald-800 dark:text-emerald-300">
            第一方证据检查点
          </div>
          <div className="mt-1 text-xs text-emerald-700 dark:text-emerald-400">
            强证据链路已验证（{items.length}/{items.length}）
          </div>
        </div>
      );
    }

    const shown = nonOk.slice(0, 4);
    const rest = Math.max(0, nonOk.length - shown.length);
    const hasFail = nonOk.some((p) => p.ok === false);
    const boxCls = hasFail
      ? "border-rose-200 dark:border-rose-800 bg-rose-50/60 dark:bg-rose-900/20"
      : "border-amber-200 dark:border-amber-800 bg-amber-50/60 dark:bg-amber-900/20";
    const titleCls = hasFail
      ? "text-rose-700 dark:text-rose-300"
      : "text-amber-800 dark:text-amber-200";

    return (
      <div className={cn("rounded-xl border px-4 py-3", boxCls)}>
        <div className={cn("text-xs font-semibold", titleCls)}>
          第一方证据检查点（未满足/无法判断）
        </div>
        <div className="mt-2 space-y-1.5">
          {shown.map((p) => (
            <div key={p.key} className="flex items-start gap-2 text-xs">
              {p.ok === false ? (
                <XCircle className="mt-0.5 h-3.5 w-3.5 text-rose-500 shrink-0" />
              ) : (
                <div className="mt-0.5 h-3.5 w-3.5 shrink-0 rounded-full border border-amber-300 dark:border-amber-700 bg-amber-50 dark:bg-amber-900/30" />
              )}
              <div className="min-w-0">
                <div className="text-foreground">{p.label}</div>
                {p.detail ? (
                  <div className="mt-0.5 text-[10px] text-muted-foreground">{p.detail}</div>
                ) : null}
              </div>
            </div>
          ))}
          {rest > 0 ? (
            <div className="text-[10px] text-muted-foreground">其余 {rest} 项详见"调试"页。</div>
          ) : null}
        </div>
      </div>
    );
  })();

  const interpretLine = (() => {
    if (!evidenceGrade) return null;
    if (summary.isRunning) {
      return "说明：证据等级会随着 Step2/Step3 探针执行逐步收敛。";
    }
    if (summary.overallPass === true && evidenceGrade.level !== "A") {
      return "说明：协议\u201c通过\u201d只代表 /v1/messages 行为符合；证据等级用于判断是否存在官方第一方链路信号。";
    }
    return null;
  })();

  return (
    <Card padding="sm" className="space-y-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <div className="text-sm font-semibold text-foreground">
              综合结论（Anthropic /v1/messages）
            </div>
          </div>
          <div className="mt-1 text-[11px] text-muted-foreground">{metaLine}</div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <span className={cn("rounded px-2 py-1 text-xs font-semibold", protocolBadge.cls)}>
            {protocolBadge.text}
          </span>
          {evidenceBadge}
          <Button
            onClick={() => void Promise.resolve(copyText(summary.plainText, "已复制验证总结"))}
            variant="ghost"
            size="sm"
            className="h-8 w-8 p-0"
            title="复制总结"
            aria-label="复制总结"
            disabled={!summary.plainText.trim()}
          >
            <Copy className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {evidenceGrade ? (
        <div className="text-[11px] text-muted-foreground">证据解释：{evidenceGrade.title}</div>
      ) : null}

      {interpretLine ? (
        <div className="text-[11px] text-muted-foreground dark:text-muted-foreground">
          {interpretLine}
        </div>
      ) : null}

      <div className={cn("grid gap-3", evidenceBox ? "sm:grid-cols-2" : "sm:grid-cols-1")}>
        {protocolBox}
        {evidenceBox}
      </div>

      <div className="text-[10px] text-muted-foreground">
        更多步骤明细请切到"步骤"，更多诊断信息请切到"调试"页。
      </div>
    </Card>
  );
}

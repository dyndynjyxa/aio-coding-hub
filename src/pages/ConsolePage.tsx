// Usage: Runtime log console. Shows in-memory app logs (time / level / title) with optional on-demand details.
// Request log details are persisted separately and should not be displayed here.

import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  clearConsoleLogs,
  formatConsoleLogDetails,
  formatConsoleLogDetailsSmart,
  getConsoleDebugEnabled,
  setConsoleDebugEnabled,
  type ConsoleLogEntry,
  type ConsoleLogLevel,
  useConsoleLogs,
} from "../services/consoleLog";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronRight, Search, Filter } from "lucide-react";
import { toast } from "sonner";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { PageHeader } from "../ui/PageHeader";
import { Switch } from "../ui/Switch";
import { cn } from "../utils/cn";

function copyToClipboard(text: string) {
  navigator.clipboard
    .writeText(text)
    .then(() => {
      toast("已复制到剪贴板");
    })
    .catch(() => {});
}

function levelText(level: ConsoleLogEntry["level"]) {
  switch (level) {
    case "error":
      return "ERROR";
    case "warn":
      return "WARN";
    case "debug":
      return "DEBUG";
    default:
      return "INFO";
  }
}

function getLevelBadgeStyles(level: ConsoleLogEntry["level"]) {
  switch (level) {
    case "error":
      return "bg-red-50 text-red-700 border-red-200 dark:bg-red-500/10 dark:text-red-400 dark:border-red-500/20";
    case "warn":
      return "bg-amber-50 text-amber-700 border-amber-200 dark:bg-amber-500/10 dark:text-amber-400 dark:border-amber-500/20";
    case "debug":
      return "bg-slate-100 text-slate-600 border-slate-200 dark:bg-slate-500/10 dark:text-slate-400 dark:border-slate-500/20";
    default:
      return "bg-emerald-50 text-emerald-700 border-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-400 dark:border-emerald-500/20";
  }
}
/** Left color bar indicator class based on event type and level */
function getRowIndicatorClass(entry: ConsoleLogEntry): string | null {
  if (entry.eventType === "gateway:circuit") {
    if (entry.level === "warn") return "border-l-2 border-red-500";
    if (entry.level === "info") return "border-l-2 border-green-500";
  }
  if (entry.level === "error") return "border-l-2 border-red-500 bg-red-50/80 dark:bg-red-500/5";
  return null;
}

const ROW_GRID_CLASS = "grid grid-cols-[150px_72px_1fr_20px] gap-2";

const LEVEL_CHIPS: ConsoleLogLevel[] = ["error", "warn", "info", "debug"];

function matchesSearch(entry: ConsoleLogEntry, query: string): boolean {
  if (!query) return true;
  const q = query.toLowerCase();
  if (entry.title.toLowerCase().includes(q)) return true;
  if (entry.meta?.trace_id?.toLowerCase().includes(q)) return true;
  if (entry.meta?.cli_key?.toLowerCase().includes(q)) return true;
  if (entry.meta?.error_code?.toLowerCase().includes(q)) return true;
  if (entry.meta?.providers?.some((p) => p.toLowerCase().includes(q))) return true;
  return false;
}

// ---------------------------------------------------------------------------
// Meta tags component
// ---------------------------------------------------------------------------

function MetaTags({ meta }: { meta: ConsoleLogEntry["meta"] }) {
  if (!meta) return null;
  const hasAny =
    meta.trace_id ||
    meta.cli_key ||
    meta.error_code ||
    (meta.providers && meta.providers.length > 0);
  if (!hasAny) return null;

  return (
    <div className="flex flex-wrap gap-1.5 mt-1">
      {meta.trace_id ? (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            copyToClipboard(meta.trace_id!);
          }}
          className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-colors cursor-pointer"
          title="点击复制 Trace ID"
        >
          trace:{meta.trace_id.slice(0, 8)}
        </button>
      ) : null}
      {meta.cli_key ? (
        <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono bg-purple-500/10 text-purple-400 border border-purple-500/20">
          cli:{meta.cli_key}
        </span>
      ) : null}
      {meta.error_code ? (
        <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono bg-red-500/10 text-red-400 border border-red-500/20">
          {meta.error_code}
        </span>
      ) : null}
      {meta.providers?.map((p) => (
        <span
          key={p}
          className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
        >
          {p}
        </span>
      ))}
    </div>
  );
}

type ConsoleLogRowProps = {
  entry: ConsoleLogEntry;
  /** Callback ref from virtualizer for dynamic size measurement */
  measureRef: (node: HTMLElement | null) => void;
  dataIndex: number;
};

const ConsoleLogRow = memo(function ConsoleLogRow({
  entry,
  measureRef,
  dataIndex,
}: ConsoleLogRowProps) {
  const hasDetails = entry.details !== undefined;
  const [smartText, setSmartText] = useState<string | null>(null);
  const [rawText, setRawText] = useState<string | null>(null);
  const [isOpen, setIsOpen] = useState(false);

  const indicatorClass = getRowIndicatorClass(entry);

  const row = (
    <div
      className={cn(
        ROW_GRID_CLASS,
        "items-start px-4 py-3 group-hover:bg-slate-100/80 dark:group-hover:bg-slate-800/40 transition-colors duration-200"
      )}
    >
      <span className="shrink-0 text-slate-500 dark:text-slate-400 font-mono text-[11px] pt-0.5">
        {entry.tsText}
      </span>
      <div className="flex items-center pt-0.5">
        <span
          className={cn(
            "shrink-0 font-medium text-[10px] px-1.5 py-0.5 rounded-md inline-flex items-center justify-center border",
            getLevelBadgeStyles(entry.level)
          )}
        >
          {levelText(entry.level)}
        </span>
      </div>
      <div className="min-w-0">
        <span className="whitespace-pre-wrap break-words text-slate-700 dark:text-slate-300 text-[13px] leading-relaxed font-normal">
          {entry.title}
        </span>
        <MetaTags meta={entry.meta} />
      </div>
      <span className="flex justify-end text-slate-600 dark:text-slate-400 group-open:text-slate-400 transition-colors duration-200 pt-0.5">
        {hasDetails ? (
          <ChevronRight className="h-4 w-4 transition-transform duration-200 group-open:rotate-90" />
        ) : null}
      </span>
    </div>
  );

  if (!hasDetails) {
    return (
      <div
        ref={measureRef}
        data-index={dataIndex}
        className={cn(
          "group border-b border-white/5 transition-colors duration-200",
          indicatorClass
        )}
      >
        {row}
      </div>
    );
  }

  return (
    <details
      ref={measureRef}
      data-index={dataIndex}
      className={cn("group border-b border-white/5 transition-colors duration-200", indicatorClass)}
      onToggle={(e) => {
        const nextOpen = e.currentTarget.open;
        setIsOpen(nextOpen);

        if (!nextOpen) return;
        if (smartText != null) return;
        const nextSmart = formatConsoleLogDetailsSmart(entry);
        setSmartText(nextSmart ?? "");
        const nextRaw = formatConsoleLogDetails(entry.details);
        setRawText(nextRaw ?? "");
      }}
    >
      <summary
        className={cn(
          "block cursor-pointer select-none outline-none transition-colors duration-200",
          "list-none [&::-webkit-details-marker]:hidden [&::marker]:content-none",
          "focus-visible:ring-2 focus-visible:ring-blue-500/50 focus-visible:ring-inset"
        )}
      >
        {row}
      </summary>
      {isOpen ? (
        <div className={cn(ROW_GRID_CLASS, "px-4 pb-4 pt-0")}>
          <div className="col-start-3 col-span-2 space-y-2">
            <pre className="custom-scrollbar max-h-60 overflow-auto rounded-md bg-slate-100 dark:bg-slate-950 p-3 text-[11px] leading-relaxed text-slate-700 dark:text-slate-400 font-mono border border-slate-200 dark:border-white/5 mx-1 whitespace-pre-wrap">
              {smartText == null ? "加载中…" : smartText ? smartText : "// 无可显示的详情"}
            </pre>
            <details className="group/raw">
              <summary className="text-[10px] text-slate-500 cursor-pointer hover:text-slate-700 dark:hover:text-slate-400 select-none mx-1">
                原始数据
              </summary>
              <pre className="custom-scrollbar max-h-40 overflow-auto rounded-md bg-slate-100 dark:bg-slate-950 p-3 text-[11px] leading-relaxed text-slate-600 dark:text-slate-500 font-mono border border-slate-200 dark:border-white/5 mx-1 mt-1">
                {rawText == null ? "加载中…" : rawText ? rawText : "// 无原始数据"}
              </pre>
            </details>
          </div>
        </div>
      ) : null}
    </details>
  );
});

// Estimated row height for collapsed console log entries (px)
const ESTIMATED_ROW_HEIGHT = 56;

export function ConsolePage() {
  const logs = useConsoleLogs();
  const [autoScroll, setAutoScroll] = useState(true);
  const [debugEnabled, setDebugEnabled] = useState(() => getConsoleDebugEnabled());
  const [showFilters, setShowFilters] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [levelFilter, setLevelFilter] = useState<Set<ConsoleLogLevel>>(
    () => new Set(["error", "warn", "info", "debug"])
  );
  const logsContainerRef = useRef<HTMLDivElement | null>(null);

  const visibleLogs = useMemo(() => {
    let filtered = debugEnabled ? logs : logs.filter((entry) => entry.level !== "debug");

    // Apply level filter (only when filters are shown and user has deselected something)
    if (showFilters && levelFilter.size < 4) {
      filtered = filtered.filter((entry) => levelFilter.has(entry.level));
    }

    // Apply text search
    if (showFilters && searchQuery) {
      filtered = filtered.filter((entry) => matchesSearch(entry, searchQuery));
    }

    return filtered;
  }, [logs, debugEnabled, showFilters, levelFilter, searchQuery]);

  const hiddenCount = logs.length - visibleLogs.length;

  const virtualizer = useVirtualizer({
    count: visibleLogs.length,
    getScrollElement: () => logsContainerRef.current,
    estimateSize: () => ESTIMATED_ROW_HEIGHT,
    overscan: 10,
  });

  const virtualItems = virtualizer.getVirtualItems();

  // Auto-scroll to bottom when new logs arrive
  const prevCountRef = useRef(visibleLogs.length);
  useEffect(() => {
    if (!autoScroll) {
      prevCountRef.current = visibleLogs.length;
      return;
    }
    if (visibleLogs.length > 0) {
      virtualizer.scrollToIndex(visibleLogs.length - 1, { align: "end" });
    }
    prevCountRef.current = visibleLogs.length;
  }, [autoScroll, visibleLogs.length, virtualizer]);

  // Detect user scroll to auto-disable/enable auto-scroll
  const handleScroll = useCallback(() => {
    const el = logsContainerRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50;
    setAutoScroll(atBottom);
  }, []);

  const toggleLevel = useCallback((level: ConsoleLogLevel) => {
    setLevelFilter((prev) => {
      const next = new Set(prev);
      if (next.has(level)) {
        next.delete(level);
      } else {
        next.add(level);
      }
      return next;
    });
  }, []);

  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <div className="shrink-0">
        <PageHeader
          title="控制台"
          actions={
            <div className="flex flex-wrap items-center gap-3">
              <div className="flex items-center gap-2">
                <span className="text-sm text-slate-600 dark:text-slate-400">自动滚动</span>
                <Switch checked={autoScroll} onCheckedChange={setAutoScroll} size="sm" />
              </div>
              <div className="flex items-center gap-2">
                <span className="text-sm text-slate-600 dark:text-slate-400">调试日志</span>
                <Switch
                  checked={debugEnabled}
                  onCheckedChange={(next) => {
                    setConsoleDebugEnabled(next);
                    setDebugEnabled(next);
                    toast(next ? "已开启调试日志" : "已关闭调试日志");
                  }}
                  size="sm"
                />
              </div>
              <Button onClick={() => setShowFilters((v) => !v)} variant="secondary">
                <Filter className="h-3.5 w-3.5 mr-1.5" />
                过滤
              </Button>
              <Button
                onClick={() => {
                  clearConsoleLogs();
                  toast("已清空控制台日志");
                }}
                variant="secondary"
              >
                清空日志
              </Button>
            </div>
          }
        />
      </div>

      {/* Filter bar */}
      {showFilters ? (
        <Card padding="none">
          <div className="px-4 py-3 flex flex-wrap items-center gap-3">
            <div className="relative flex-1 min-w-[200px] max-w-[360px]">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-slate-400" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="搜索标题、trace_id、错误码..."
                className="h-8 w-full rounded-md border border-slate-200 bg-white pl-8 pr-3 text-xs text-slate-700 placeholder:text-slate-400 outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-200 dark:placeholder:text-slate-500"
              />
            </div>
            <div className="flex items-center gap-1.5">
              {LEVEL_CHIPS.map((level) => (
                <button
                  key={level}
                  type="button"
                  onClick={() => toggleLevel(level)}
                  className={cn(
                    "px-2 py-1 rounded text-[10px] font-medium border transition-colors cursor-pointer",
                    levelFilter.has(level)
                      ? getLevelBadgeStyles(level)
                      : "bg-slate-50 text-slate-500 border-slate-200 opacity-70 dark:bg-slate-800 dark:text-slate-500 dark:border-slate-700 dark:opacity-50"
                  )}
                >
                  {levelText(level)}
                </button>
              ))}
            </div>
          </div>
        </Card>
      ) : null}

      <Card padding="none" className="min-h-0 flex-1 flex flex-col overflow-hidden">
        <div className="shrink-0 border-b border-slate-200 dark:border-slate-700 bg-gradient-to-r from-slate-50 to-slate-100/50 dark:from-slate-800 dark:to-slate-800/50 px-6 py-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-3">
              <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                日志{" "}
                <span className="ml-1.5 inline-flex items-center rounded-full bg-accent/10 px-2.5 py-0.5 text-xs font-medium text-accent">
                  {visibleLogs.length}
                </span>
              </div>
              {hiddenCount > 0 ? (
                <div className="text-xs text-slate-500 dark:text-slate-400 flex items-center gap-1.5">
                  <span className="inline-block h-1 w-1 rounded-full bg-slate-400 dark:bg-slate-500"></span>
                  已隐藏 {hiddenCount} 条日志
                </div>
              ) : null}
            </div>
            <div className="text-xs text-slate-500 dark:text-slate-400 flex items-center gap-1.5">
              <svg
                className="h-3.5 w-3.5 text-slate-400 dark:text-slate-500"
                fill="none"
                viewBox="0 0 24 24"
                strokeWidth="2"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z"
                />
              </svg>
              点击单条日志可展开详情
            </div>
          </div>
        </div>

        <div
          ref={logsContainerRef}
          onScroll={handleScroll}
          className={cn(
            "custom-scrollbar min-h-0 flex-1 overflow-auto",
            "bg-gradient-to-b from-slate-50 to-white dark:from-slate-950 dark:to-slate-900 font-mono text-[12px] leading-relaxed text-slate-700 dark:text-slate-200",
            "shadow-inner"
          )}
        >
          {visibleLogs.length === 0 ? (
            <div className="flex flex-col items-center justify-center px-4 py-16 text-center">
              <div className="mb-3 rounded-full bg-slate-100 p-4 border border-slate-200 dark:bg-slate-800/50 dark:border-slate-700/50">
                <svg
                  className="h-8 w-8 text-slate-600 dark:text-slate-400"
                  fill="none"
                  viewBox="0 0 24 24"
                  strokeWidth="1.5"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z"
                  />
                </svg>
              </div>
              <p className="text-sm font-medium text-slate-400 dark:text-slate-500">
                {logs.length === 0 ? "暂无日志" : "暂无可显示的日志"}
              </p>
              <p className="mt-1 text-xs text-slate-600 dark:text-slate-400">
                {logs.length === 0 ? "系统日志将在这里显示" : "调整过滤器以查看更多日志"}
              </p>
            </div>
          ) : (
            <div
              style={{
                height: virtualizer.getTotalSize(),
                width: "100%",
                position: "relative",
              }}
            >
              <div
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  transform: `translateY(${virtualItems[0]?.start ?? 0}px)`,
                }}
              >
                {virtualItems.map((virtualRow) => (
                  <ConsoleLogRow
                    key={visibleLogs[virtualRow.index].id}
                    entry={visibleLogs[virtualRow.index]}
                    measureRef={virtualizer.measureElement}
                    dataIndex={virtualRow.index}
                  />
                ))}
              </div>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}

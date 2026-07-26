// Usage:
// - Small badge components shared by the Home "request logs" list and "realtime traces" cards.
// - Designed to keep session reuse / free / fast mode / folder badges consistent across the Home page.

import { Tooltip } from "../../ui/Tooltip";
import { FolderOpen } from "lucide-react";

const SESSION_REUSE_TOOLTIP =
  "同一 session_id 在 5 分钟 TTL 内优先复用上一次成功 provider，减少抖动/提升缓存命中";

export function SessionReuseBadge({ showCustomTooltip }: { showCustomTooltip: boolean }) {
  const className =
    "inline-flex shrink-0 items-center whitespace-nowrap rounded-md bg-blue-50/85 px-2 py-0.5 text-[11px] font-semibold text-blue-600 ring-1 ring-inset ring-blue-400/35 dark:bg-blue-500/15 dark:text-blue-300 dark:ring-blue-400/20 cursor-help";
  return showCustomTooltip ? (
    <Tooltip content={SESSION_REUSE_TOOLTIP}>
      <span className={className}>会话复用</span>
    </Tooltip>
  ) : (
    <span className={className} title={SESSION_REUSE_TOOLTIP}>
      会话复用
    </span>
  );
}

export function FreeBadge() {
  return (
    <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-md bg-emerald-50/80 px-2 py-0.5 text-[11px] font-semibold text-emerald-600 ring-1 ring-inset ring-emerald-500/10 dark:bg-emerald-500/15 dark:text-emerald-300 dark:ring-emerald-400/20">
      免费
    </span>
  );
}

const FAST_MODE_TOOLTIP = "Codex 优先服务层 (fast mode) - 使用更高优先级资源，费率更高";

export function FastModeBadge({ showCustomTooltip }: { showCustomTooltip: boolean }) {
  const className =
    "inline-flex shrink-0 items-center whitespace-nowrap rounded-md bg-orange-50/80 px-2 py-0.5 text-[11px] font-semibold text-orange-600 ring-1 ring-inset ring-orange-500/10 dark:bg-orange-500/15 dark:text-orange-300 dark:ring-orange-400/20 cursor-help";

  return showCustomTooltip ? (
    <Tooltip content={FAST_MODE_TOOLTIP}>
      <span className={className}>fast</span>
    </Tooltip>
  ) : (
    <span className={className} title={FAST_MODE_TOOLTIP}>
      fast
    </span>
  );
}

export function FolderBadge({
  folderName,
  folderPath,
  allowWrap = false,
}: {
  folderName: string;
  folderPath: string;
  allowWrap?: boolean;
}) {
  return (
    <span
      className="inline-flex min-w-0 items-center gap-1 rounded-md bg-muted/65 px-2 py-0.5 text-[11px] font-medium text-muted-foreground border border-border/45 dark:bg-muted/40 dark:border-border/30 shadow-[0_1px_2px_rgba(0,0,0,0.01)]"
      title={folderPath}
    >
      <FolderOpen className="h-3 w-3 shrink-0 text-muted-foreground/60" />
      <span className={allowWrap ? "whitespace-normal break-all" : "truncate"}>{folderName}</span>
    </span>
  );
}

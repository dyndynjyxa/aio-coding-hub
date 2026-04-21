// Usage: Shared error card for query failures with retry button.

import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";

export function QueryErrorCard({
  errorText,
  loading,
  onRetry,
  message = "用量刷新失败，请重试；必要时查看 Console 日志。",
}: {
  errorText: string | null;
  loading: boolean;
  onRetry: () => void;
  message?: string;
}) {
  if (!errorText) return null;

  return (
    <Card
      padding="md"
      className="shrink-0 border-rose-200 bg-rose-50 dark:border-rose-700 dark:bg-rose-900/30"
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <div className="text-sm font-semibold text-rose-900 dark:text-rose-300">加载失败</div>
          <div className="mt-1 text-sm text-rose-800 dark:text-rose-200">{message}</div>
        </div>
        <Button
          size="sm"
          variant="secondary"
          onClick={onRetry}
          disabled={loading}
          className="border-rose-200 bg-white text-rose-800 hover:bg-rose-50 dark:border-rose-700 dark:bg-slate-800 dark:text-rose-200 dark:hover:bg-rose-900/30"
        >
          重试
        </Button>
      </div>
      <div className="mt-3 rounded-lg border border-rose-200 bg-white/70 p-3 font-mono text-xs text-slate-800 dark:border-rose-700 dark:bg-slate-800/70 dark:text-slate-200">
        {errorText}
      </div>
    </Card>
  );
}

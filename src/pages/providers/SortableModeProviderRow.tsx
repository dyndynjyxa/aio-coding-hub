import { memo } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { ProviderSummary } from "../../services/providers/providers";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import { providerBaseUrlSummary } from "./baseUrl";

export type SortableModeProviderRowProps = {
  providerId: number;
  provider: ProviderSummary | null;
  modeEnabled: boolean;
  disabled: boolean;
  onToggleEnabled: (providerId: number, enabled: boolean) => void;
  onRemove: (providerId: number) => void;
};

export const SortableModeProviderRow = memo(function SortableModeProviderRow({
  providerId,
  provider,
  modeEnabled,
  disabled,
  onToggleEnabled,
  onRemove,
}: SortableModeProviderRowProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: providerId,
    disabled,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style}>
      <Card
        padding="sm"
        className={cn(
          "flex cursor-grab flex-col gap-2 transition-shadow duration-200 active:cursor-grabbing sm:flex-row sm:items-center sm:justify-between",
          isDragging && "z-10 scale-[1.02] opacity-90 shadow-lg ring-2 ring-accent/30",
          disabled && "opacity-70",
          !modeEnabled && "bg-slate-50 dark:bg-slate-800"
        )}
        {...attributes}
        {...listeners}
      >
        <div className="flex min-w-0 items-start gap-3 sm:items-center">
          <div className="mt-0.5 inline-flex h-8 w-8 select-none items-center justify-center rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-slate-400 dark:text-slate-500 sm:mt-0">
            ⠿
          </div>
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-2">
              <div className="truncate text-sm font-semibold">
                {provider?.name?.trim() ? provider.name : `未知 Provider #${providerId}`}
              </div>
              {!modeEnabled ? (
                <span className="shrink-0 rounded-full bg-slate-100 dark:bg-slate-700 px-2 py-0.5 font-mono text-[10px] text-slate-600 dark:text-slate-400">
                  模板关闭
                </span>
              ) : null}
            </div>
            <div className="truncate text-xs text-slate-500 dark:text-slate-400">
              {providerBaseUrlSummary(provider)}
            </div>
          </div>
        </div>

        <div
          className="flex flex-wrap items-center gap-2"
          onPointerDown={(e) => e.stopPropagation()}
        >
          <div className="flex items-center gap-2">
            <span className="text-xs text-slate-600 dark:text-slate-400">启用</span>
            <Switch
              checked={modeEnabled}
              onCheckedChange={(checked) => onToggleEnabled(providerId, checked)}
              disabled={disabled}
              size="sm"
            />
          </div>
          <Button
            onClick={() => onRemove(providerId)}
            variant="secondary"
            size="sm"
            className="hover:!bg-rose-50 hover:!text-rose-600"
            disabled={disabled}
          >
            移除
          </Button>
        </div>
      </Card>
    </div>
  );
});

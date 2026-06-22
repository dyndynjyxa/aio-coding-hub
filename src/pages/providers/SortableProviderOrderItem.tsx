import { memo, type HTMLAttributes, type ReactNode } from "react";
import { GripVertical } from "lucide-react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { ProviderSummary } from "../../services/providers/providers";
import { cn } from "../../utils/cn";

export type ProviderOrderItemProps = {
  provider: ProviderSummary | null;
  providerId?: number;
  index: number;
  trailing?: ReactNode;
  className?: string;
  dragProps?: HTMLAttributes<HTMLDivElement>;
  showProviderDisabledBadge?: boolean;
};

export const ProviderOrderItem = memo(function ProviderOrderItem({
  provider,
  providerId,
  trailing = null,
  className,
  dragProps,
  showProviderDisabledBadge = true,
}: ProviderOrderItemProps) {
  const label = provider?.name?.trim()
    ? provider.name
    : `未知 Provider #${provider?.id ?? providerId ?? "?"}`;

  return (
    <div
      className={cn(
        "flex items-center gap-2 rounded-md border border-border bg-card px-2.5 py-2 text-sm shadow-sm transition-shadow duration-200",
        className
      )}
    >
      {dragProps ? (
        <div
          className="inline-flex h-7 w-7 shrink-0 cursor-grab items-center justify-center rounded-md text-muted-foreground hover:bg-secondary active:cursor-grabbing"
          title="拖拽调整顺序"
          aria-label={`拖拽调整 ${label} 顺序`}
          {...dragProps}
        >
          <GripVertical className="h-4 w-4" aria-hidden="true" />
        </div>
      ) : null}
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium text-foreground">{label}</div>
      </div>
      {showProviderDisabledBadge && provider && !provider.enabled ? (
        <span className="shrink-0 rounded-full bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
          关闭
        </span>
      ) : null}
      {trailing}
    </div>
  );
});

export type SortableProviderOrderItemProps = Omit<
  ProviderOrderItemProps,
  "dragProps" | "className"
> & {
  disabled?: boolean;
};

export const SortableProviderOrderItem = memo(function SortableProviderOrderItem({
  provider,
  providerId,
  index,
  trailing = null,
  showProviderDisabledBadge = true,
  disabled = false,
}: SortableProviderOrderItemProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: provider?.id ?? providerId ?? 0,
    disabled,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style}>
      <ProviderOrderItem
        provider={provider}
        providerId={providerId}
        index={index}
        trailing={trailing}
        showProviderDisabledBadge={showProviderDisabledBadge}
        className={cn(
          isDragging && "z-10 scale-[1.02] opacity-95 shadow-lg ring-2 ring-ring/30",
          disabled && "opacity-70"
        )}
        dragProps={disabled ? undefined : { ...attributes, ...listeners }}
      />
    </div>
  );
});

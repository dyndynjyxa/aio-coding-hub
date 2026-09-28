import { cn } from "@/ui/shadcn/utils";

export type TabListSize = "sm" | "md";
export type TabListVariant = "default" | "compact";

export type TabListItem<T extends string> = {
  key: T;
  label: string;
  disabled?: boolean;
};

export type TabListProps<T extends string> = {
  ariaLabel: string;
  items: Array<TabListItem<T>>;
  value: T;
  onChange: (next: T) => void;
  className?: string;
  size?: TabListSize;
  variant?: TabListVariant;
  buttonClassName?: string;
};

export function TabList<T extends string>({
  ariaLabel,
  items,
  value,
  onChange,
  className,
  size = "sm",
  variant = "default",
  buttonClassName,
}: TabListProps<T>) {
  function handleKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (
      event.key !== "ArrowRight" &&
      event.key !== "ArrowLeft" &&
      event.key !== "Home" &&
      event.key !== "End"
    ) {
      return;
    }

    const enabledItems = items.filter((item) => !item.disabled);
    if (enabledItems.length === 0) return;

    event.preventDefault();

    const currentIndex = Math.max(
      0,
      enabledItems.findIndex((item) => item.key === value)
    );
    const nextIndex =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? enabledItems.length - 1
          : event.key === "ArrowRight"
            ? (currentIndex + 1) % enabledItems.length
            : (currentIndex - 1 + enabledItems.length) % enabledItems.length;
    const next = enabledItems[nextIndex];
    onChange(next.key);

    const nextTab = event.currentTarget.querySelector<HTMLButtonElement>(
      `[data-tab-key="${next.key}"]`
    );
    nextTab?.focus();
  }

  const isCompact = variant === "compact";

  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      tabIndex={-1}
      onKeyDown={handleKeyDown}
      className={cn(
        isCompact
          ? [
              "grid overflow-hidden rounded-lg border border-border bg-secondary",
              items.length === 5 && "grid-cols-5",
            ]
          : "inline-flex items-center overflow-hidden rounded-lg border border-border bg-secondary p-[3px]",
        className
      )}
      style={
        isCompact && items.length !== 5
          ? { gridTemplateColumns: `repeat(${items.length}, 1fr)` }
          : undefined
      }
    >
      {items.map((item) => {
        const active = value === item.key;
        return (
          <button
            key={item.key}
            type="button"
            onClick={() => onChange(item.key)}
            role="tab"
            aria-selected={active}
            tabIndex={active ? 0 : -1}
            data-tab-key={item.key}
            disabled={item.disabled}
            className={cn(
              "inline-flex items-center justify-center font-semibold transition-all h-auto",
              isCompact
                ? [
                    "w-full justify-center rounded-none border-r border-border px-3 py-1.5 text-sm last:border-r-0",
                    active
                      ? "bg-primary text-primary-foreground font-medium"
                      : "text-muted-foreground hover:bg-state-hover",
                  ]
                : [
                    `rounded-lg font-bold ${size === "sm" ? "text-sm" : "text-base"} gap-2 border`,
                    size === "sm" ? "px-3 py-1.5" : "px-3.5 py-2",
                    active
                      ? "bg-primary text-primary-foreground border-primary shadow-sm shadow-primary/10 cursor-default"
                      : "text-muted-foreground hover:bg-state-hover hover:text-foreground border-transparent cursor-pointer",
                  ],
              "disabled:cursor-not-allowed disabled:opacity-50",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
              buttonClassName
            )}
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}

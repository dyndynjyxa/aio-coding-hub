import { cn } from "@/ui/shadcn/utils";

export type TabListSize = "sm" | "md";

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
  buttonClassName?: string;
};

export function TabList<T extends string>({
  ariaLabel,
  items,
  value,
  onChange,
  className,
  size = "sm",
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

  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      onKeyDown={handleKeyDown}
      className={cn(
        "inline-flex items-center rounded-2xl overflow-hidden border border-line-subtle bg-surface-inset p-[3px]",
        className
      )}
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
              "inline-flex items-center justify-center gap-2 rounded-lg font-bold text-sm transition-all border h-auto",
              size === "sm" ? "px-3 py-1.5" : "px-3.5 py-2",
              active
                ? "bg-primary text-primary-foreground border-primary shadow-sm shadow-primary/10 cursor-default"
                : "text-muted-foreground hover:bg-state-hover hover:text-foreground border-transparent cursor-pointer",
              "disabled:cursor-not-allowed disabled:opacity-50",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/35 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
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

import { Button, type ButtonSize } from "@/ui/shadcn/button";
import { cn } from "@/ui/shadcn/utils";

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
  size?: ButtonSize;
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
        "inline-flex items-center rounded-xl border border-line-subtle bg-surface-inset p-[3px]",
        className
      )}
    >
      {items.map((item) => {
        const active = value === item.key;
        return (
          <Button
            key={item.key}
            onClick={() => onChange(item.key)}
            variant={active ? "primary" : "ghost"}
            size={size}
            role="tab"
            aria-selected={active}
            tabIndex={active ? 0 : -1}
            data-tab-key={item.key}
            disabled={item.disabled}
            className={cn("h-auto rounded-lg px-3 py-2 shadow-none", buttonClassName)}
          >
            <span className="text-sm font-semibold">{item.label}</span>
          </Button>
        );
      })}
    </div>
  );
}

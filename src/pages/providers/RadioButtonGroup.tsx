// Usage: Generic radio-button group used in ProviderEditorDialog for mode selection.

import { cn } from "../../utils/cn";

type RadioButtonGroupProps<T extends string> = {
  value: T;
  onChange: (value: T) => void;
  disabled?: boolean;
  ariaLabel: string;
  items: Array<{ value: T; label: string }>;
  size?: "default" | "compact";
  fullWidth?: boolean;
};

export function RadioButtonGroup<T extends string>({
  value,
  onChange,
  disabled,
  ariaLabel,
  items,
  size = "default",
  fullWidth = true,
}: RadioButtonGroupProps<T>) {
  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className={cn(
        "inline-flex overflow-hidden rounded-lg border border-border bg-white shadow-sm dark:border-border dark:bg-secondary",
        fullWidth ? "w-full" : "w-auto",
        disabled ? "opacity-60" : null
      )}
    >
      {items.map((item, index) => {
        const active = value === item.value;
        return (
          <button
            key={item.value}
            type="button"
            onClick={() => onChange(item.value)}
            role="radio"
            aria-checked={active}
            disabled={disabled}
            className={cn(
              fullWidth ? "flex-1" : null,
              size === "compact"
                ? "px-2.5 py-1.5 text-xs font-medium"
                : "px-3 py-2 text-sm font-medium",
              "transition",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/30 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-background",
              index < items.length - 1 ? "border-r border-border dark:border-border" : null,
              active ? "bg-gradient-to-br from-accent to-accent-secondary text-white" : null,
              !active
                ? "bg-white text-secondary-foreground hover:bg-secondary dark:text-secondary-foreground dark:hover:bg-secondary"
                : null,
              disabled ? "cursor-not-allowed" : null
            )}
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}

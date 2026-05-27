import { cn } from "@/ui/shadcn/utils";

export interface RadioGroupProps {
  name: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<{
    value: string;
    label: string;
  }>;
  disabled?: boolean;
}

export function RadioGroup({ name, value, onChange, options, disabled }: RadioGroupProps) {
  return (
    <div className="flex flex-wrap items-center gap-3">
      {options.map((option) => {
        const isSelected = value === option.value;
        return (
          <label
            key={option.value}
            className={cn(
              "flex items-center gap-2.5 px-3.5 py-2 rounded-xl border cursor-pointer transition-all duration-200 select-none",
              isSelected
                ? "bg-state-selected border-state-selected-border text-state-selected-foreground shadow-sm shadow-primary/5"
                : "bg-card border-line-subtle hover:bg-state-hover hover:border-line text-muted-foreground hover:text-foreground",
              disabled && "opacity-50 cursor-not-allowed"
            )}
          >
            <div className="relative flex items-center justify-center">
              <input
                type="radio"
                name={name}
                value={option.value}
                checked={isSelected}
                onChange={(e) => onChange(e.currentTarget.value)}
                disabled={disabled}
                className="peer sr-only"
              />
              <div
                className={cn(
                  "h-4 w-4 rounded-full border flex items-center justify-center transition-all duration-200",
                  "peer-focus-visible:outline-none peer-focus-visible:ring-2 peer-focus-visible:ring-ring/35 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-background",
                  isSelected
                    ? "border-primary bg-primary scale-100"
                    : "border-border bg-card hover:border-border-strong"
                )}
              >
                <div
                  className={cn(
                    "h-1.5 w-1.5 rounded-full bg-white dark:bg-background transition-transform duration-200 scale-0",
                    isSelected && "scale-100"
                  )}
                />
              </div>
            </div>
            <span className="text-sm font-semibold tracking-wide">{option.label}</span>
          </label>
        );
      })}
    </div>
  );
}

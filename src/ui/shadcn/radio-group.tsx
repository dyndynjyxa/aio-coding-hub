import { useId } from "react";
import { cn } from "@/ui/shadcn/utils";

export interface RadioGroupProps {
  name: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<{
    value: string;
    label: string;
    description?: string | null;
  }>;
  ariaLabel: string;
  disabled?: boolean;
  ariaDescription?: string | null;
}

export function RadioGroup({
  name,
  value,
  onChange,
  options,
  disabled,
  ariaLabel,
  ariaDescription,
}: RadioGroupProps) {
  const accessibilityId = useId();
  const groupDescriptionId = ariaDescription ? `${accessibilityId}-group-description` : undefined;

  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      aria-describedby={groupDescriptionId}
      className="flex flex-wrap items-center gap-2"
    >
      {ariaDescription ? (
        <span id={groupDescriptionId} className="sr-only">
          {ariaDescription}
        </span>
      ) : null}
      {options.map((option, index) => {
        const isSelected = value === option.value;
        const optionLabelId = `${accessibilityId}-option-${index}-label`;
        const optionDescriptionId = option.description
          ? `${accessibilityId}-option-${index}-description`
          : undefined;
        return (
          <label
            key={option.value}
            title={option.description ?? undefined}
            className={cn(
              "flex items-center gap-2 px-2.5 py-1.5 rounded-md border cursor-pointer transition-all duration-200 select-none",
              isSelected
                ? "bg-state-selected border-state-selected-border text-state-selected-foreground shadow-md shadow-primary/10"
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
                aria-labelledby={optionLabelId}
                aria-describedby={optionDescriptionId}
                className="peer sr-only"
              />
              <div
                className={cn(
                  "h-3.5 w-3.5 rounded-full border flex items-center justify-center transition-all duration-200",
                  "peer-focus-visible:outline-none peer-focus-visible:ring-2 peer-focus-visible:ring-ring/30 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-background",
                  isSelected
                    ? "border-primary bg-primary scale-100"
                    : "border-border bg-card hover:border-border-strong"
                )}
              >
                <div
                  className={cn(
                    "h-1.5 w-1.5 rounded-full bg-primary-foreground transition-transform duration-200 scale-0",
                    isSelected && "scale-100"
                  )}
                />
              </div>
            </div>
            <span id={optionLabelId} className="whitespace-nowrap text-xs font-medium">
              {option.label}
            </span>
            {option.description ? (
              <span id={optionDescriptionId} className="sr-only">
                {option.description}
              </span>
            ) : null}
          </label>
        );
      })}
    </div>
  );
}

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
    <div className="flex flex-wrap items-center gap-2">
      {options.map((option) => (
        <label
          key={option.value}
          className={cn(
            "flex items-center gap-2 px-3 py-1.5 rounded-md border border-input cursor-pointer transition-colors",
            value === option.value
              ? "bg-secondary border-ring text-foreground"
              : "bg-card hover:bg-secondary text-muted-foreground",
            disabled && "opacity-50 cursor-not-allowed"
          )}
        >
          <input
            type="radio"
            name={name}
            value={option.value}
            checked={value === option.value}
            onChange={(e) => onChange(e.currentTarget.value)}
            disabled={disabled}
            className="cursor-pointer"
          />
          <span className="text-sm font-medium">{option.label}</span>
        </label>
      ))}
    </div>
  );
}

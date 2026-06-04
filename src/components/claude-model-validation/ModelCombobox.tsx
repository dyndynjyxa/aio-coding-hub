import { useRef, useState } from "react";
import { Popover as PopoverRoot, PopoverContent, PopoverTrigger } from "../../ui/shadcn/popover";
import { cn } from "../../utils/cn";
import { ChevronsUpDown, Check } from "lucide-react";
import { PRESET_MODEL_OPTIONS } from "./helpers";

/** Select + Input combo: click to show preset list, also accepts free input */
export function ModelCombobox({
  value,
  onChange,
  disabled,
}: {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  return (
    <PopoverRoot open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          disabled={disabled}
          className={cn(
            "flex h-10 w-full items-center justify-between rounded-md border border-border",
            "bg-white/80 dark:bg-card/80 px-3 text-xs font-mono shadow-sm",
            "focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50",
            !value.trim() && "text-muted-foreground"
          )}
        >
          <span className="truncate">{value.trim() || "选择或输入模型..."}</span>
          <ChevronsUpDown className="ml-2 h-3.5 w-3.5 shrink-0 opacity-50" />
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-[var(--radix-popover-trigger-width)] p-0">
        <div className="border-b border-border px-2 py-1.5">
          <input
            ref={inputRef}
            value={value}
            onChange={(e) => onChange(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") setOpen(false);
            }}
            placeholder="输入模型名称..."
            autoFocus
            className="h-8 w-full rounded-md border-0 bg-transparent px-1 text-xs font-mono focus:outline-none placeholder:text-muted-foreground dark:placeholder:text-muted-foreground"
          />
        </div>
        <div className="max-h-48 overflow-y-auto py-1">
          {PRESET_MODEL_OPTIONS.map((m) => (
            <button
              key={m}
              type="button"
              className={cn(
                "flex w-full items-center gap-2 px-3 py-1.5 text-xs font-mono text-left",
                "hover:bg-secondary dark:hover:bg-secondary/60 transition-colors",
                m === value && "bg-secondary dark:bg-secondary/40"
              )}
              onClick={() => {
                onChange(m);
                setOpen(false);
              }}
            >
              <Check
                className={cn("h-3.5 w-3.5 shrink-0", m === value ? "opacity-100" : "opacity-0")}
              />
              {m}
            </button>
          ))}
        </div>
      </PopoverContent>
    </PopoverRoot>
  );
}

import * as SwitchPrimitive from "@radix-ui/react-switch";
import { forwardRef } from "react";
import { cn } from "@/ui/shadcn/utils";

export type SwitchProps = Omit<
  React.ComponentPropsWithoutRef<typeof SwitchPrimitive.Root>,
  "checked" | "defaultChecked" | "onCheckedChange"
> & {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  size?: "sm" | "md";
};

export const Switch = forwardRef<React.ElementRef<typeof SwitchPrimitive.Root>, SwitchProps>(
  function Switch({ checked, onCheckedChange, size = "md", className, type, ...props }, ref) {
    const isSmall = size === "sm";
    return (
      <SwitchPrimitive.Root
        ref={ref}
        checked={checked}
        onCheckedChange={onCheckedChange}
        type={type ?? "button"}
        className={cn(
          [
            "inline-flex shrink-0 items-center rounded-full border-2 border-transparent transition-colors",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
            "disabled:cursor-not-allowed disabled:opacity-50",
            "data-[state=checked]:bg-accent",
            "data-[state=unchecked]:bg-slate-300 dark:data-[state=unchecked]:bg-slate-700",
          ].join(" "),
          isSmall ? "h-5 w-9" : "h-6 w-11",
          className
        )}
        {...props}
      >
        <SwitchPrimitive.Thumb
          className={cn(
            "pointer-events-none block rounded-full bg-white shadow-sm transition-transform dark:bg-slate-100",
            isSmall
              ? "h-4 w-4 data-[state=checked]:translate-x-4"
              : "h-5 w-5 data-[state=checked]:translate-x-5"
          )}
        />
      </SwitchPrimitive.Root>
    );
  }
);

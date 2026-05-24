import { useId, type ReactNode } from "react";
import { cn } from "../utils/cn";

export type FormFieldProps = {
  /** Visible label text. */
  label: string;
  /** Optional hint shown to the right of the label. */
  hint?: ReactNode;
  /** The form control(s) rendered inside this field. */
  children: ReactNode | ((id: string) => ReactNode);
  className?: string;
  /** Explicit id to associate the label with the control. When omitted a stable id is generated automatically. */
  htmlFor?: string;
};

export function FormField({ label, hint, children, className, htmlFor }: FormFieldProps) {
  const autoId = useId();
  const fieldId = htmlFor ?? autoId;

  return (
    <div className={cn("space-y-1.5", className)}>
      <div className="flex items-center justify-between gap-3">
        <label htmlFor={fieldId} className="text-sm font-medium text-foreground">
          {label}
        </label>
        {hint ? <div className="text-xs text-muted-foreground">{hint}</div> : null}
      </div>
      {typeof children === "function" ? children(fieldId) : children}
    </div>
  );
}

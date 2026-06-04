import { cn } from "../../utils/cn";

export function OutcomePill({ pass }: { pass: boolean | null }) {
  if (pass == null) {
    return (
      <span className="rounded bg-secondary px-1.5 py-0.5 text-[10px] font-semibold text-muted-foreground">
        未知
      </span>
    );
  }
  return (
    <span
      className={cn(
        "rounded px-1.5 py-0.5 text-[10px] font-semibold",
        pass
          ? "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400"
          : "bg-rose-100 text-rose-700 dark:bg-rose-900/30 dark:text-rose-400"
      )}
    >
      {pass ? "通过" : "不通过"}
    </span>
  );
}

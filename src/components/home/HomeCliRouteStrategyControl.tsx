import type { CliKey } from "../../services/providers/providers";
import type { SortModeSummary } from "../../services/providers/sortModes";
import { Select } from "../../ui/Select";
import { cn } from "../../utils/cn";

function measureLabelUnits(label: string) {
  let units = 0;
  for (const char of label) {
    units += /[\u0000-\u00ff]/.test(char) ? 1 : 2;
  }
  return units;
}

export type HomeCliRouteStrategyControlProps = {
  cliKey: CliKey;
  cliLabel: string;
  sortModes: SortModeSummary[];
  sortModesLoading: boolean;
  sortModesAvailable: boolean | null;
  activeModeByCli: Record<CliKey, number | null>;
  activeModeToggling: Record<CliKey, boolean>;
  onSetCliActiveMode: (cliKey: CliKey, modeId: number | null) => void;
  orientation?: "horizontal" | "vertical";
  className?: string;
  selectClassName?: string;
};

export function HomeCliRouteStrategyControl({
  cliKey,
  cliLabel,
  sortModes,
  sortModesLoading,
  sortModesAvailable,
  activeModeByCli,
  activeModeToggling,
  onSetCliActiveMode,
  orientation = "horizontal",
  className,
  selectClassName,
}: HomeCliRouteStrategyControlProps) {
  const vertical = orientation === "vertical";
  const selectedModeValue = String(activeModeByCli[cliKey] ?? "");
  const sortModeSelectDisabled =
    sortModesLoading || sortModesAvailable === false || activeModeToggling[cliKey];
  const selectWidthCh =
    Math.max(8, ...["Default", ...sortModes.map((mode) => mode.name)].map(measureLabelUnits)) + 4;

  return (
    <div className={cn("flex min-w-0 shrink-0 items-center", className)}>
      <Select
        value={selectedModeValue}
        onChange={(event) => {
          const nextValue = event.currentTarget.value;
          onSetCliActiveMode(cliKey, nextValue === "" ? null : Number(nextValue));
        }}
        disabled={sortModeSelectDisabled}
        className={cn(
          vertical
            ? "h-6 max-w-full border-border bg-white px-2 pr-5 text-[12px] dark:border-border dark:bg-card"
            : "h-7 max-w-full flex-none border-border bg-white px-2 pr-6 text-sm dark:border-border dark:bg-card",
          selectClassName
        )}
        style={{ width: `${selectWidthCh}ch`, maxWidth: "100%" }}
        aria-label={`${cliLabel} 路由策略`}
      >
        <option value="">Default</option>
        {sortModes.map((mode) => (
          <option key={mode.id} value={String(mode.id)}>
            {mode.name}
          </option>
        ))}
      </Select>
    </div>
  );
}

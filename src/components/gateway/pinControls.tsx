// Usage:
// - Shared pin/tier + sort_mode controls used by both the active-session card
//   (HomeActiveSessionsCard) and the session list rows (SessionRowPinControl).
// - Extracted so the two surfaces share *exactly* the same visual + behavioural
//   language: same icons, same colors, same tooltips, same mutation wiring.
// - All four pin mutations are wired here; consumers only need to provide the
//   current tier + mode + a key.

import { Pin, Lock, PinOff } from "lucide-react";
import {
  useGatewaySessionPinSortModeMutation,
  useGatewaySessionUnpinSortModeMutation,
  useGatewaySessionPersistSortModeMutation,
  useGatewaySessionUnpersistSortModeMutation,
} from "../../query/gateway";
import { validateGatewayCliKey } from "../../services/gateway/gateway";
import type { CliKey } from "../../services/providers/providers";
import type { SortModeSummary } from "../../services/providers/sortModes";
import { Select } from "../../ui/Select";
import { Tooltip } from "../../ui/Tooltip";
import { cn } from "../../utils/cn";

export type PinTier = "none" | "ephemeral" | "persistent";

function isSupportedCliKey(cliKey: string): cliKey is CliKey {
  try {
    validateGatewayCliKey(cliKey);
    return true;
  } catch {
    return false;
  }
}

const DEFAULT_MODE_VALUE = "default";
const AUTO_MODE_VALUE = "auto";

/** Map a (pinned?, modeId) pair to the <select> value string. */
function pinStateToValue(pinned: boolean, modeId: number | null): string {
  if (!pinned) return AUTO_MODE_VALUE;
  return modeId == null ? DEFAULT_MODE_VALUE : String(modeId);
}

/** Parse a <select> value back to a sortModeId (`undefined` = "auto/clear"). */
function valueToSortModeId(raw: string): number | null | undefined {
  if (raw === AUTO_MODE_VALUE) return undefined;
  if (raw === DEFAULT_MODE_VALUE) return null;
  const id = Number(raw);
  if (!Number.isSafeInteger(id) || id <= 0) return undefined;
  return id;
}

export type SortModeSelectProps = {
  cliKey: string;
  sessionId: string;
  tier: PinTier;
  selectedModeId: number | null | undefined;
  modes: SortModeSummary[];
  /** Render the "auto" placeholder as either "自动（跟随激活）" or "自动（清除 pin）". */
  autoLabel: "follow" | "clear";
  /** Optional className for the select (e.g. w-[150px] vs w-full). */
  className?: string;
  /** Whether the row's surrounding clickable surface should not bubble. */
  stopClickPropagation?: boolean;
};

/**
 * Sort-mode picker. Choosing a mode applies it at the *current* pin tier:
 * - not pinned → ephemeral pin (lowest friction, 5-min TTL)
 * - ephemeral  → re-pin ephemeral to the new mode
 * - persistent → re-pin persistent to the new mode
 * "自动" clears whatever pin is active.
 */
export function SortModeSelect({
  cliKey,
  sessionId,
  tier,
  selectedModeId,
  modes,
  autoLabel,
  className,
  stopClickPropagation = false,
}: SortModeSelectProps) {
  const supported = isSupportedCliKey(cliKey);
  const pinMutation = useGatewaySessionPinSortModeMutation();
  const unpinMutation = useGatewaySessionUnpinSortModeMutation();
  const persistMutation = useGatewaySessionPersistSortModeMutation();
  const unpersistMutation = useGatewaySessionUnpersistSortModeMutation();

  const busy =
    pinMutation.isPending ||
    unpinMutation.isPending ||
    persistMutation.isPending ||
    unpersistMutation.isPending;
  const selectedValue = pinStateToValue(tier !== "none", selectedModeId ?? null);

  const stop = stopClickPropagation
    ? {
        onClick: (e: React.MouseEvent) => e.stopPropagation(),
        onKeyDown: (e: React.KeyboardEvent) => e.stopPropagation(),
      }
    : {};

  return (
    <div className={className} {...stop}>
      <Select
        aria-label="路由策略"
        className="h-7 px-2 text-xs"
        disabled={!supported || busy}
        value={selectedValue}
        onChange={(event) => {
          const raw = event.target.value;
          if (raw === selectedValue) return;
          const sortModeId = valueToSortModeId(raw);
          if (sortModeId === undefined) {
            // "自动" — clear whatever pin is active.
            if (tier === "persistent") {
              unpersistMutation.mutate({ cliKey: cliKey as CliKey, sessionId });
            } else if (tier === "ephemeral") {
              unpinMutation.mutate({ cliKey: cliKey as CliKey, sessionId });
            }
            return;
          }
          // Apply at the current tier; if not pinned, default to ephemeral.
          if (tier === "persistent") {
            persistMutation.mutate({ cliKey: cliKey as CliKey, sessionId, sortModeId });
          } else {
            pinMutation.mutate({ cliKey: cliKey as CliKey, sessionId, sortModeId });
          }
        }}
      >
        <option value={AUTO_MODE_VALUE}>
          {autoLabel === "follow" && tier === "none"
            ? "自动（跟随激活）"
            : autoLabel === "clear" && tier !== "none"
              ? "自动（清除 pin）"
              : "自动"}
        </option>
        <option value={DEFAULT_MODE_VALUE}>Default</option>
        {modes.map((mode) => (
          <option key={mode.id} value={String(mode.id)}>
            {mode.name}
          </option>
        ))}
      </Select>
    </div>
  );
}

export type PinTierButtonProps = {
  cliKey: string;
  sessionId: string;
  tier: PinTier;
  selectedModeId: number | null | undefined;
  /**
   * Optional stop-propagation, useful when the row itself is a clickable
   * container (e.g. SessionProjectPage) and we don't want pin clicks to
   * trigger row-level navigation.
   */
  stopClickPropagation?: boolean;
  /** Optional tooltip override (rare). */
  tooltipOverride?: {
    none: string;
    ephemeral: string;
    persistent: string;
  };
};

/**
 * Three-state pin-level toggle. Cycles none → ephemeral → persistent → none.
 * - none → ephemeral: pin at the currently-selected mode (or Default if none)
 * - ephemeral → persistent: promote to disk-backed (same mode)
 * - persistent → none: clear
 * Mutual exclusion is enforced server-side; the client also clears the
 * superseded tier explicitly for instant feedback.
 */
export function PinTierButton({
  cliKey,
  sessionId,
  tier,
  selectedModeId,
  stopClickPropagation = false,
  tooltipOverride,
}: PinTierButtonProps) {
  const supported = isSupportedCliKey(cliKey);
  const pinMutation = useGatewaySessionPinSortModeMutation();
  const unpinMutation = useGatewaySessionUnpinSortModeMutation();
  const persistMutation = useGatewaySessionPersistSortModeMutation();
  const unpersistMutation = useGatewaySessionUnpersistSortModeMutation();
  const busy =
    pinMutation.isPending ||
    unpinMutation.isPending ||
    persistMutation.isPending ||
    unpersistMutation.isPending;

  /** Resolve the mode id to pin at, falling back to Default. */
  const resolveModeId = (): number | null => {
    if (selectedModeId != null) return selectedModeId;
    return null; // Default
  };

  const handleClick = (e: React.MouseEvent) => {
    if (stopClickPropagation) e.stopPropagation();
    if (busy) return;
    if (tier === "none") {
      // none → ephemeral
      pinMutation.mutate({ cliKey: cliKey as CliKey, sessionId, sortModeId: resolveModeId() });
    } else if (tier === "ephemeral") {
      // ephemeral → persistent (promote). Persist writes to disk, then clear the
      // in-memory ephemeral pin so the persistent one actually takes effect
      // (ephemeral otherwise outranks persistent in the routing chain).
      persistMutation.mutate(
        { cliKey: cliKey as CliKey, sessionId, sortModeId: resolveModeId() },
        {
          onSuccess: () => {
            unpinMutation.mutate({ cliKey: cliKey as CliKey, sessionId });
          },
        }
      );
    } else {
      // persistent → none (clear)
      unpersistMutation.mutate({ cliKey: cliKey as CliKey, sessionId });
    }
  };

  const config = (() => {
    if (tier === "ephemeral") {
      return {
        icon: Pin,
        label: "本次",
        tone: "bg-indigo-500/15 text-indigo-600 dark:text-indigo-300 ring-1 ring-indigo-300/40 dark:ring-indigo-600/40",
        tooltip:
          tooltipOverride?.ephemeral ??
          "本次（临时）已指派 · 5 分钟会话 TTL 后自动清除。点击切换为持久。",
      };
    }
    if (tier === "persistent") {
      return {
        icon: Lock,
        label: "持久",
        tone: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-300 ring-1 ring-emerald-300/40 dark:ring-emerald-600/40",
        tooltip:
          tooltipOverride?.persistent ??
          "持久指派 · 落盘，跨重启 / 会话 resume 仍生效。点击解除 pin。",
      };
    }
    return {
      icon: PinOff,
      label: "未 pin",
      tone: "bg-muted/40 text-muted-foreground hover:bg-muted",
      tooltip: tooltipOverride?.none ?? "未指派 · 跟随激活策略。点击设为本次（5 分钟）。",
    };
  })();

  const Icon = config.icon;

  return (
    <Tooltip content={config.tooltip} placement="top">
      <button
        type="button"
        aria-label={`pin 状态：${config.label}`}
        disabled={!supported || busy}
        onClick={handleClick}
        className={cn(
          "inline-flex items-center gap-1 rounded-md px-2 py-1 text-[10px] font-semibold transition-colors",
          "disabled:opacity-50 disabled:cursor-not-allowed",
          config.tone
        )}
      >
        <Icon className="h-3 w-3" />
        {config.label}
      </button>
    </Tooltip>
  );
}

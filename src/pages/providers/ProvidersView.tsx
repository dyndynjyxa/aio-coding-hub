// Usage: Rendered by ProvidersPage when `view === "providers"`.

import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  type DragEndEvent,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { CLIS } from "../../constants/clis";
import { ClaudeModelValidationDialog } from "../../components/ClaudeModelValidationDialog";
import { logToConsole } from "../../services/consoleLog";
import {
  providerDelete,
  providerSetEnabled,
  providersReorder,
  type CliKey,
  type ProviderSummary,
} from "../../services/providers";
import {
  gatewayCircuitResetCli,
  gatewayCircuitResetProvider,
  gatewayCircuitStatus,
  type GatewayProviderCircuitStatus,
} from "../../services/gateway";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Dialog } from "../../ui/Dialog";
import { Switch } from "../../ui/Switch";
import { cn } from "../../utils/cn";
import { formatCountdownSeconds, formatUnixSeconds } from "../../utils/formatters";
import { hasTauriRuntime } from "../../services/tauriInvoke";
import { providerBaseUrlSummary } from "./baseUrl";
import { ProviderEditorDialog } from "./ProviderEditorDialog";

const CIRCUIT_EVENT_REFRESH_THROTTLE_MS = 1000;

type SortableProviderCardProps = {
  provider: ProviderSummary;
  circuit: GatewayProviderCircuitStatus | null;
  circuitResetting: boolean;
  onToggleEnabled: (provider: ProviderSummary) => void;
  onResetCircuit: (provider: ProviderSummary) => void;
  onValidateModel?: (provider: ProviderSummary) => void;
  onEdit: (provider: ProviderSummary) => void;
  onDelete: (provider: ProviderSummary) => void;
};

function SortableProviderCard({
  provider,
  circuit,
  circuitResetting,
  onToggleEnabled,
  onResetCircuit,
  onValidateModel,
  onEdit,
  onDelete,
}: SortableProviderCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: provider.id,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  const claudeModelsCount = Object.values(provider.claude_models ?? {}).filter((value) => {
    if (typeof value !== "string") return false;
    return Boolean(value.trim());
  }).length;
  const hasClaudeModels = claudeModelsCount > 0;

  const isOpen = circuit?.state === "OPEN";
  const cooldownUntil = circuit?.cooldown_until ?? null;
  const isUnavailable = isOpen || (cooldownUntil != null && Number.isFinite(cooldownUntil));
  const [nowUnix, setNowUnix] = useState(() => Math.floor(Date.now() / 1000));
  useEffect(() => {
    if (!isUnavailable) return;
    setNowUnix(Math.floor(Date.now() / 1000));
    const timer = window.setInterval(() => {
      setNowUnix(Math.floor(Date.now() / 1000));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [isUnavailable]);

  const unavailableUntil = isUnavailable
    ? (() => {
        const openUntil = isOpen ? (circuit?.open_until ?? null) : null;
        if (openUntil == null) return cooldownUntil;
        if (cooldownUntil == null) return openUntil;
        return Math.max(openUntil, cooldownUntil);
      })()
    : null;
  const unavailableRemaining =
    unavailableUntil != null ? Math.max(0, unavailableUntil - nowUnix) : null;
  const unavailableCountdown =
    unavailableRemaining != null ? formatCountdownSeconds(unavailableRemaining) : null;

  return (
    <div ref={setNodeRef} style={style} className="relative">
      <Card
        padding="sm"
        className={cn(
          "flex cursor-grab flex-col gap-2 transition-shadow duration-200 active:cursor-grabbing sm:flex-row sm:items-center sm:justify-between",
          isDragging && "z-10 scale-[1.02] shadow-lg ring-2 ring-[#0052FF]/30"
        )}
        {...attributes}
        {...listeners}
      >
        <div className="flex min-w-0 items-center gap-3">
          <div className="inline-flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-lg border border-slate-200 bg-white text-slate-400">
            ⠿
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 items-center gap-2">
              <div className="truncate text-sm font-semibold">{provider.name}</div>
              {isUnavailable ? (
                <span
                  className="shrink-0 rounded-full bg-rose-50 px-2 py-0.5 font-mono text-[10px] text-rose-700"
                  title={
                    unavailableUntil != null
                      ? `熔断至 ${formatUnixSeconds(unavailableUntil)}`
                      : "熔断"
                  }
                >
                  熔断{unavailableCountdown ? ` ${unavailableCountdown}` : ""}
                </span>
              ) : null}
            </div>
            <div className="mt-1 flex items-center gap-2">
              <span className="shrink-0 rounded-full bg-slate-50 px-2 py-0.5 font-mono text-[10px] text-slate-700">
                {provider.base_url_mode === "ping" ? "Ping" : "顺序"}
              </span>
              <span className="shrink-0 rounded-full bg-slate-50 px-2 py-0.5 font-mono text-[10px] text-slate-700">
                倍率 {provider.cost_multiplier}x
              </span>
              {provider.cli_key === "claude" && hasClaudeModels ? (
                <span
                  className="shrink-0 rounded-full bg-sky-50 px-2 py-0.5 font-mono text-[10px] text-sky-700"
                  title={`已配置 Claude 模型映射（${claudeModelsCount}/5）`}
                >
                  Claude Models
                </span>
              ) : null}
            </div>
            <div
              className="mt-1 truncate font-mono text-xs text-slate-500 cursor-default"
              title={provider.base_urls.join("\n")}
            >
              {providerBaseUrlSummary(provider)}
            </div>
          </div>
        </div>

        <div
          className="flex flex-wrap items-center gap-3"
          onPointerDown={(e) => e.stopPropagation()}
        >
          <div className="flex items-center gap-2">
            <span className="text-xs text-slate-600">启用</span>
            <Switch checked={provider.enabled} onCheckedChange={() => onToggleEnabled(provider)} />
          </div>

          {isUnavailable ? (
            <Button
              onClick={() => onResetCircuit(provider)}
              variant="secondary"
              disabled={circuitResetting}
            >
              {circuitResetting ? "处理中…" : "解除熔断"}
            </Button>
          ) : null}

          {onValidateModel ? (
            <Button
              onClick={() => onValidateModel(provider)}
              variant="secondary"
              size="icon"
              title="模型验证"
            >
              <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12l2 2 4-4m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            </Button>
          ) : null}

          <Button onClick={() => onEdit(provider)} variant="secondary" size="icon" title="编辑">
            <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
              />
            </svg>
          </Button>

          <Button
            onClick={() => onDelete(provider)}
            variant="secondary"
            size="icon"
            className="hover:!bg-rose-50 hover:!text-rose-600"
            title="删除"
          >
            <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
              />
            </svg>
          </Button>
        </div>
      </Card>
    </div>
  );
}

export type ProvidersViewProps = {
  activeCli: CliKey;
  setActiveCli: (cliKey: CliKey) => void;
  providers: ProviderSummary[];
  setProviders: React.Dispatch<React.SetStateAction<ProviderSummary[]>>;
  providersLoading: boolean;
  refreshProviders: (cliKey: CliKey) => Promise<void>;
};

export function ProvidersView({
  activeCli,
  setActiveCli,
  providers,
  setProviders,
  providersLoading,
  refreshProviders,
}: ProvidersViewProps) {
  const activeCliRef = useRef(activeCli);
  useEffect(() => {
    activeCliRef.current = activeCli;
  }, [activeCli]);

  const providersRef = useRef(providers);
  useEffect(() => {
    providersRef.current = providers;
  }, [providers]);

  const [circuitByProviderId, setCircuitByProviderId] = useState<
    Record<number, GatewayProviderCircuitStatus>
  >({});
  const [circuitLoading, setCircuitLoading] = useState(false);
  const [circuitResetting, setCircuitResetting] = useState<Record<number, boolean>>({});
  const [circuitResettingAll, setCircuitResettingAll] = useState(false);
  const circuitRefreshInFlightRef = useRef(false);
  const circuitRefreshQueuedRef = useRef(false);
  const circuitEventRefreshTimerRef = useRef<number | null>(null);
  const circuitAutoRefreshTimerRef = useRef<number | null>(null);

  const hasUnavailableCircuit = useMemo(
    () =>
      Object.values(circuitByProviderId).some(
        (row) =>
          row.state === "OPEN" ||
          (row.cooldown_until != null && Number.isFinite(row.cooldown_until))
      ),
    [circuitByProviderId]
  );

  const [createOpen, setCreateOpen] = useState(false);
  const [createCliKeyLocked, setCreateCliKeyLocked] = useState<CliKey | null>(null);

  const [editTarget, setEditTarget] = useState<ProviderSummary | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ProviderSummary | null>(null);
  const [deleting, setDeleting] = useState(false);

  const [validateDialogOpen, setValidateDialogOpen] = useState(false);
  const [validateProvider, setValidateProvider] = useState<ProviderSummary | null>(null);

  useEffect(() => {
    if (activeCli !== "claude" && validateDialogOpen) {
      setValidateDialogOpen(false);
      setValidateProvider(null);
    }
  }, [activeCli, validateDialogOpen]);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    })
  );

  const refreshCircuit = useMemo(
    () => async (cliKey: CliKey) => {
      if (circuitRefreshInFlightRef.current) {
        circuitRefreshQueuedRef.current = true;
        return;
      }

      circuitRefreshInFlightRef.current = true;
      if (activeCliRef.current === cliKey) setCircuitLoading(true);

      try {
        const rows = await gatewayCircuitStatus(cliKey);
        if (activeCliRef.current !== cliKey) return;
        if (!rows) {
          setCircuitByProviderId({});
          return;
        }

        const next: Record<number, GatewayProviderCircuitStatus> = {};
        for (const row of rows) {
          next[row.provider_id] = row;
        }
        setCircuitByProviderId(next);
      } catch (err) {
        if (activeCliRef.current !== cliKey) return;
        logToConsole("warn", "读取熔断状态失败", { cli: cliKey, error: String(err) });
        setCircuitByProviderId({});
      } finally {
        circuitRefreshInFlightRef.current = false;

        if (circuitRefreshQueuedRef.current) {
          circuitRefreshQueuedRef.current = false;
          void refreshCircuit(activeCliRef.current);
          return;
        }

        if (activeCliRef.current === cliKey) {
          setCircuitLoading(false);
        }
      }
    },
    []
  );

  const scheduleRefreshCircuit = useMemo(
    () => () => {
      if (circuitEventRefreshTimerRef.current != null) return;
      circuitEventRefreshTimerRef.current = window.setTimeout(() => {
        circuitEventRefreshTimerRef.current = null;
        void refreshCircuit(activeCliRef.current);
      }, CIRCUIT_EVENT_REFRESH_THROTTLE_MS);
    },
    [refreshCircuit]
  );

  useEffect(() => {
    setCircuitByProviderId({});
    setCircuitResetting({});
    setCircuitResettingAll(false);
    void refreshCircuit(activeCli);
  }, [activeCli, refreshCircuit]);

  useEffect(() => {
    if (circuitAutoRefreshTimerRef.current != null) {
      window.clearTimeout(circuitAutoRefreshTimerRef.current);
      circuitAutoRefreshTimerRef.current = null;
    }

    if (!hasUnavailableCircuit) return;

    const nowUnix = Math.floor(Date.now() / 1000);
    let nextAvailableUntil: number | null = null;
    for (const row of Object.values(circuitByProviderId)) {
      const cooldownUntil = row.cooldown_until ?? null;
      const isUnavailable =
        row.state === "OPEN" || (cooldownUntil != null && Number.isFinite(cooldownUntil));
      if (!isUnavailable) continue;

      const openUntil = row.state === "OPEN" ? (row.open_until ?? null) : null;
      const until =
        openUntil == null
          ? cooldownUntil
          : cooldownUntil == null
            ? openUntil
            : Math.max(openUntil, cooldownUntil);

      if (until == null) {
        nextAvailableUntil = nowUnix;
        break;
      }
      if (nextAvailableUntil == null || until < nextAvailableUntil) nextAvailableUntil = until;
    }
    if (nextAvailableUntil == null) return;

    const delayMs = Math.max(200, (nextAvailableUntil - nowUnix) * 1000 + 250);
    circuitAutoRefreshTimerRef.current = window.setTimeout(() => {
      circuitAutoRefreshTimerRef.current = null;
      void refreshCircuit(activeCliRef.current);
    }, delayMs);

    return () => {
      if (circuitAutoRefreshTimerRef.current != null) {
        window.clearTimeout(circuitAutoRefreshTimerRef.current);
        circuitAutoRefreshTimerRef.current = null;
      }
    };
  }, [circuitByProviderId, hasUnavailableCircuit, refreshCircuit]);

  useEffect(() => {
    if (!hasTauriRuntime()) return;

    let cancelled = false;
    let unlisten: null | (() => void) = null;

    import("@tauri-apps/api/event")
      .then(({ listen }) =>
        listen("gateway:circuit", (event) => {
          if (cancelled) return;
          const payload = event.payload as any;
          if (!payload) return;
          if (payload.cli_key && payload.cli_key !== activeCliRef.current) return;
          scheduleRefreshCircuit();
        })
      )
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // ignore: events unavailable in non-tauri environment
      });

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
      if (circuitEventRefreshTimerRef.current != null) {
        window.clearTimeout(circuitEventRefreshTimerRef.current);
        circuitEventRefreshTimerRef.current = null;
      }
    };
  }, [scheduleRefreshCircuit]);

  async function toggleProviderEnabled(provider: ProviderSummary) {
    try {
      const next = await providerSetEnabled(provider.id, !provider.enabled);
      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      setProviders((prev) => prev.map((p) => (p.id === next.id ? next : p)));
      logToConsole("info", "更新 Provider 状态", { id: next.id, enabled: next.enabled });
      toast(next.enabled ? "已启用 Provider" : "已禁用 Provider");
    } catch (err) {
      logToConsole("error", "更新 Provider 状态失败", { error: String(err), id: provider.id });
      toast(`更新失败：${String(err)}`);
    }
  }

  async function resetCircuit(provider: ProviderSummary) {
    if (circuitResetting[provider.id]) return;
    setCircuitResetting((cur) => ({ ...cur, [provider.id]: true }));

    try {
      const ok = await gatewayCircuitResetProvider(provider.id);
      if (!ok) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      toast("已解除熔断");
      void refreshCircuit(provider.cli_key);
    } catch (err) {
      logToConsole("error", "解除熔断失败", { provider_id: provider.id, error: String(err) });
      toast(`解除熔断失败：${String(err)}`);
    } finally {
      setCircuitResetting((cur) => ({ ...cur, [provider.id]: false }));
    }
  }

  async function resetCircuitAll(cliKey: CliKey) {
    if (circuitResettingAll) return;
    setCircuitResettingAll(true);

    try {
      const count = await gatewayCircuitResetCli(cliKey);
      if (count == null) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      toast(count > 0 ? `已解除 ${count} 个 Provider 的熔断` : "无 Provider 需要处理");
      void refreshCircuit(cliKey);
    } catch (err) {
      logToConsole("error", "解除熔断（全部）失败", { cli: cliKey, error: String(err) });
      toast(`解除熔断失败：${String(err)}`);
    } finally {
      setCircuitResettingAll(false);
    }
  }

  function requestValidateProviderModel(provider: ProviderSummary) {
    if (activeCliRef.current !== "claude") return;
    setValidateProvider(provider);
    setValidateDialogOpen(true);
  }

  async function confirmRemoveProvider() {
    if (!deleteTarget || deleting) return;
    setDeleting(true);
    try {
      const ok = await providerDelete(deleteTarget.id);
      if (!ok) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      setProviders((prev) => prev.filter((p) => p.id !== deleteTarget.id));
      logToConsole("info", "删除 Provider", {
        id: deleteTarget.id,
        name: deleteTarget.name,
      });
      toast("Provider 已删除");
      setDeleteTarget(null);
    } catch (err) {
      logToConsole("error", "删除 Provider 失败", {
        error: String(err),
        id: deleteTarget.id,
      });
      toast(`删除失败：${String(err)}`);
    } finally {
      setDeleting(false);
    }
  }

  async function persistProvidersOrder(
    cliKey: CliKey,
    nextProviders: ProviderSummary[],
    prevProviders: ProviderSummary[]
  ) {
    try {
      const saved = await providersReorder(
        cliKey,
        nextProviders.map((p) => p.id)
      );
      if (!saved) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      if (activeCliRef.current !== cliKey) {
        return;
      }

      setProviders(saved);
      logToConsole("info", "更新 Provider 顺序", {
        cli: cliKey,
        order: saved.map((p) => p.id),
      });
      toast("顺序已更新");
    } catch (err) {
      if (activeCliRef.current === cliKey) {
        setProviders(prevProviders);
      }
      logToConsole("error", "更新 Provider 顺序失败", {
        cli: cliKey,
        error: String(err),
      });
      toast(`顺序更新失败：${String(err)}`);
    }
  }

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const cliKey = activeCliRef.current;
    const prevProviders = providersRef.current;
    const oldIndex = prevProviders.findIndex((p) => p.id === active.id);
    const newIndex = prevProviders.findIndex((p) => p.id === over.id);

    if (oldIndex === -1 || newIndex === -1) return;

    const nextProviders = arrayMove(prevProviders, oldIndex, newIndex);
    setProviders(nextProviders);
    void persistProvidersOrder(cliKey, nextProviders, prevProviders);
  }

  return (
    <>
      <div className="flex flex-col gap-3 lg:min-h-0 lg:flex-1">
        <div className="flex items-center justify-between gap-3">
          <div className="flex flex-wrap items-center gap-2">
            {CLIS.map((cli) => (
              <Button
                key={cli.key}
                onClick={() => setActiveCli(cli.key)}
                variant={activeCli === cli.key ? "primary" : "secondary"}
                size="sm"
              >
                {cli.name}
              </Button>
            ))}
          </div>
        </div>

        <div className="flex items-center justify-between gap-2">
          <div className="text-[11px] text-slate-500">路由顺序：按拖拽顺序（上→下）</div>
          <div className="flex items-center gap-2">
            {hasUnavailableCircuit ? (
              <Button
                onClick={() => void resetCircuitAll(activeCli)}
                variant="secondary"
                size="sm"
                disabled={circuitResettingAll || circuitLoading || providers.length === 0}
              >
                {circuitResettingAll
                  ? "处理中…"
                  : circuitLoading
                    ? "熔断加载中…"
                    : "解除熔断（全部）"}
              </Button>
            ) : null}

            <Button
              onClick={() => {
                setCreateCliKeyLocked(activeCli);
                setCreateOpen(true);
              }}
              variant="secondary"
              size="sm"
            >
              添加
            </Button>
          </div>
        </div>

        <div className="lg:min-h-0 lg:flex-1 lg:overflow-auto lg:pr-1">
          {providersLoading ? (
            <div className="text-sm text-slate-600">加载中…</div>
          ) : providers.length === 0 ? (
            <div className="text-sm text-slate-600">暂无 Provider。请点击「添加」新增。</div>
          ) : (
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={providers.map((p) => p.id)}
                strategy={verticalListSortingStrategy}
              >
                <div className="space-y-3">
                  {providers.map((provider) => (
                    <SortableProviderCard
                      key={provider.id}
                      provider={provider}
                      circuit={circuitByProviderId[provider.id] ?? null}
                      circuitResetting={Boolean(circuitResetting[provider.id]) || circuitLoading}
                      onToggleEnabled={toggleProviderEnabled}
                      onResetCircuit={resetCircuit}
                      onValidateModel={
                        activeCli === "claude" ? requestValidateProviderModel : undefined
                      }
                      onEdit={setEditTarget}
                      onDelete={setDeleteTarget}
                    />
                  ))}
                </div>
              </SortableContext>
            </DndContext>
          )}
        </div>
      </div>

      <ClaudeModelValidationDialog
        open={validateDialogOpen}
        onOpenChange={(open) => {
          setValidateDialogOpen(open);
          if (!open) setValidateProvider(null);
        }}
        provider={validateProvider}
      />

      {createCliKeyLocked ? (
        <ProviderEditorDialog
          mode="create"
          open={createOpen}
          onOpenChange={(nextOpen) => {
            setCreateOpen(nextOpen);
            if (!nextOpen) setCreateCliKeyLocked(null);
          }}
          cliKey={createCliKeyLocked}
          onSaved={(cliKey) => {
            void refreshProviders(cliKey);
            void refreshCircuit(cliKey);
          }}
        />
      ) : null}

      {editTarget ? (
        <ProviderEditorDialog
          mode="edit"
          open={true}
          onOpenChange={(nextOpen) => {
            if (!nextOpen) setEditTarget(null);
          }}
          provider={editTarget}
          onSaved={(cliKey) => {
            void refreshProviders(cliKey);
            void refreshCircuit(cliKey);
          }}
        />
      ) : null}

      <Dialog
        open={!!deleteTarget}
        onOpenChange={(nextOpen) => {
          if (!nextOpen && deleting) return;
          if (!nextOpen) setDeleteTarget(null);
        }}
        title="确认删除 Provider"
        description={deleteTarget ? `将删除：${deleteTarget.name}` : undefined}
        className="max-w-lg"
      >
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button onClick={() => setDeleteTarget(null)} variant="secondary" disabled={deleting}>
            取消
          </Button>
          <Button onClick={confirmRemoveProvider} variant="primary" disabled={deleting}>
            {deleting ? "删除中…" : "确认删除"}
          </Button>
        </div>
      </Dialog>
    </>
  );
}

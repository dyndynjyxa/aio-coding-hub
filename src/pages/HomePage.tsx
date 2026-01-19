// Usage: Dashboard / overview page. Backend commands: `request_logs_*`, `request_attempt_logs_*`, `usage_*`, `gateway_*`, `providers_*`, `sort_modes_*`.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { CLIS } from "../constants/clis";
import { HomeCostPanel } from "../components/home/HomeCostPanel";
import { HomeOverviewPanel } from "../components/home/HomeOverviewPanel";
import { RequestLogDetailDialog } from "../components/home/RequestLogDetailDialog";
import { logToConsole } from "../services/consoleLog";
import {
  requestAttemptLogsByTraceId,
  requestLogGet,
  requestLogsListAfterIdAll,
  requestLogsListAll,
  type RequestAttemptLog,
  type RequestLogDetail,
  type RequestLogSummary,
} from "../services/requestLogs";
import { usageHourlySeries, type UsageHourlyRow } from "../services/usage";
import { ProviderCircuitBadge, type OpenCircuitRow } from "../components/ProviderCircuitBadge";
import {
  gatewayCircuitStatus,
  gatewayCircuitResetProvider,
  gatewaySessionsList,
  type GatewayActiveSession,
} from "../services/gateway";
import { providersList, type CliKey } from "../services/providers";
import {
  sortModeActiveList,
  sortModeActiveSet,
  sortModesList,
  type SortModeSummary,
} from "../services/sortModes";
import { useCliProxy } from "../hooks/useCliProxy";
import { useWindowForeground } from "../hooks/useWindowForeground";
import { Card } from "../ui/Card";
import { TabList } from "../ui/TabList";
import { hasTauriRuntime } from "../services/tauriInvoke";
import { useTraceStore } from "../services/traceStore";

type HomeTabKey = "overview" | "cost" | "more";
type UsageHeatmapRefreshReason = "initial" | "manual" | "foreground" | "tab";

const HOME_TABS: Array<{ key: HomeTabKey; label: string }> = [
  { key: "overview", label: "概览" },
  { key: "cost", label: "花费" },
  { key: "more", label: "更多" },
];

const REALTIME_TRACE_EXIT_TOTAL_MS = 1000;
const OPEN_CIRCUITS_EVENT_REFRESH_THROTTLE_MS = 1000;

function openCircuitRowsEqual(a: OpenCircuitRow[], b: OpenCircuitRow[]) {
  if (a === b) return true;
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    const left = a[i];
    const right = b[i];
    if (left.cli_key !== right.cli_key) return false;
    if (left.provider_id !== right.provider_id) return false;
    if (left.provider_name !== right.provider_name) return false;
    if (left.open_until !== right.open_until) return false;
  }
  return true;
}

export function HomePage() {
  const { traces } = useTraceStore();
  const tauriRuntime = hasTauriRuntime();
  const showCustomTooltip = tauriRuntime;

  const cliProxy = useCliProxy();

  const [tab, setTab] = useState<HomeTabKey>("overview");
  const tabRef = useRef(tab);
  const isMountedRef = useRef(true);

  const [sortModes, setSortModes] = useState<SortModeSummary[]>([]);
  const [sortModesLoading, setSortModesLoading] = useState(false);
  const [sortModesAvailable, setSortModesAvailable] = useState<boolean | null>(null);
  const [activeModeByCli, setActiveModeByCli] = useState<Record<CliKey, number | null>>({
    claude: null,
    codex: null,
    gemini: null,
  });
  const [activeModeToggling, setActiveModeToggling] = useState<Record<CliKey, boolean>>({
    claude: false,
    codex: false,
    gemini: false,
  });

  const [requestLogs, setRequestLogs] = useState<RequestLogSummary[]>([]);
  const [requestLogsLoading, setRequestLogsLoading] = useState(false);
  const [requestLogsRefreshing, setRequestLogsRefreshing] = useState(false);
  const [requestLogsAvailable, setRequestLogsAvailable] = useState<boolean | null>(null);
  const [selectedLogId, setSelectedLogId] = useState<number | null>(null);
  const [selectedLog, setSelectedLog] = useState<RequestLogDetail | null>(null);
  const [selectedLogLoading, setSelectedLogLoading] = useState(false);
  const [attemptLogs, setAttemptLogs] = useState<RequestAttemptLog[]>([]);
  const [attemptLogsLoading, setAttemptLogsLoading] = useState(false);

  const [usageHeatmapRows, setUsageHeatmapRows] = useState<UsageHourlyRow[]>([]);
  const [usageHeatmapLoading, setUsageHeatmapLoading] = useState(false);
  const [, setUsageHeatmapAvailable] = useState<boolean | null>(null);
  const usageHeatmapRefreshInFlightRef = useRef(false);

  const [activeSessions, setActiveSessions] = useState<GatewayActiveSession[]>([]);
  const [activeSessionsLoading, setActiveSessionsLoading] = useState(false);
  const [activeSessionsAvailable, setActiveSessionsAvailable] = useState<boolean | null>(null);

  const [openCircuits, setOpenCircuits] = useState<OpenCircuitRow[]>([]);
  const [resettingProviderIds, setResettingProviderIds] = useState<Set<number>>(new Set());
  const openCircuitsRefreshInFlightRef = useRef(false);
  const openCircuitsRefreshQueuedRef = useRef(false);
  const openCircuitsEventRefreshTimerRef = useRef<number | null>(null);
  const openCircuitsAutoRefreshTimerRef = useRef<number | null>(null);

  const requestLogsRef = useRef<RequestLogSummary[]>([]);
  const requestLogsInFlightRef = useRef(false);
  const requestLogsAutoRefreshTimerRef = useRef<number | null>(null);
  const completedTraceIdsSeenRef = useRef<Set<string>>(new Set());
  const initializedTraceSeenRef = useRef(false);
  const [, setRealtimeExitHoldUntilMs] = useState(0);
  const realtimeExitHoldUntilRef = useRef(0);
  const realtimeExitHoldTimerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      isMountedRef.current = false;
      if (realtimeExitHoldTimerRef.current != null) {
        window.clearTimeout(realtimeExitHoldTimerRef.current);
        realtimeExitHoldTimerRef.current = null;
      }
      if (openCircuitsAutoRefreshTimerRef.current != null) {
        window.clearTimeout(openCircuitsAutoRefreshTimerRef.current);
        openCircuitsAutoRefreshTimerRef.current = null;
      }
      if (openCircuitsEventRefreshTimerRef.current != null) {
        window.clearTimeout(openCircuitsEventRefreshTimerRef.current);
        openCircuitsEventRefreshTimerRef.current = null;
      }
      if (requestLogsAutoRefreshTimerRef.current != null) {
        window.clearTimeout(requestLogsAutoRefreshTimerRef.current);
        requestLogsAutoRefreshTimerRef.current = null;
      }
    };
  }, []);

  const refreshOpenCircuits = useCallback(async () => {
    if (!hasTauriRuntime()) {
      setOpenCircuits((prev) => (prev.length === 0 ? prev : []));
      return;
    }

    if (openCircuitsRefreshInFlightRef.current) {
      openCircuitsRefreshQueuedRef.current = true;
      return;
    }

    openCircuitsRefreshInFlightRef.current = true;
    try {
      const rowsByCli = await Promise.all(
        CLIS.map(async (cli) => {
          const cliKey = cli.key as CliKey;
          try {
            const circuits = await gatewayCircuitStatus(cliKey);
            const unavailable = (circuits ?? []).filter(
              (row) =>
                row.state === "OPEN" ||
                (row.cooldown_until != null && Number.isFinite(row.cooldown_until))
            );

            if (unavailable.length === 0) {
              return [] as OpenCircuitRow[];
            }

            const providers = await providersList(cliKey);
            const providerNameById: Record<number, string> = {};
            for (const provider of providers ?? []) {
              const name = provider.name?.trim();
              if (!name) continue;
              providerNameById[provider.id] = name;
            }

            return unavailable.map((row) => {
              const cooldownUntil = row.cooldown_until ?? null;
              if (row.state !== "OPEN") {
                return {
                  cli_key: cliKey,
                  provider_id: row.provider_id,
                  provider_name: providerNameById[row.provider_id] ?? "未知",
                  open_until: cooldownUntil,
                };
              }

              const openUntil = row.open_until ?? null;
              const until =
                openUntil == null
                  ? cooldownUntil
                  : cooldownUntil == null
                    ? openUntil
                    : Math.max(openUntil, cooldownUntil);

              return {
                cli_key: cliKey,
                provider_id: row.provider_id,
                provider_name: providerNameById[row.provider_id] ?? "未知",
                open_until: until,
              };
            });
          } catch (err) {
            logToConsole("warn", "读取熔断状态失败", { cli: cliKey, error: String(err) });
            return [] as OpenCircuitRow[];
          }
        })
      );

      const next = rowsByCli.flat();
      next.sort((a, b) => {
        const aUntil = a.open_until ?? Number.POSITIVE_INFINITY;
        const bUntil = b.open_until ?? Number.POSITIVE_INFINITY;
        if (aUntil !== bUntil) return aUntil - bUntil;
        if (a.cli_key !== b.cli_key) return a.cli_key.localeCompare(b.cli_key);
        return a.provider_name.localeCompare(b.provider_name);
      });

      setOpenCircuits((prev) => (openCircuitRowsEqual(prev, next) ? prev : next));
    } finally {
      openCircuitsRefreshInFlightRef.current = false;
      if (openCircuitsRefreshQueuedRef.current) {
        openCircuitsRefreshQueuedRef.current = false;
        void refreshOpenCircuits();
      }
    }
  }, []);

  const scheduleRefreshOpenCircuits = useCallback(() => {
    if (!hasTauriRuntime()) return;

    if (openCircuitsEventRefreshTimerRef.current != null) return;
    openCircuitsEventRefreshTimerRef.current = window.setTimeout(() => {
      openCircuitsEventRefreshTimerRef.current = null;
      void refreshOpenCircuits();
    }, OPEN_CIRCUITS_EVENT_REFRESH_THROTTLE_MS);
  }, [refreshOpenCircuits]);

  const handleResetProvider = useCallback(
    async (providerId: number) => {
      if (resettingProviderIds.has(providerId)) return;

      setResettingProviderIds((prev) => new Set(prev).add(providerId));
      try {
        const result = await gatewayCircuitResetProvider(providerId);
        if (result) {
          toast.success("已解除熔断");
        } else {
          toast.error("解除熔断失败");
        }
        void refreshOpenCircuits();
      } catch (err) {
        logToConsole("error", "解除熔断失败", { providerId, error: String(err) });
        toast.error("解除熔断失败");
      } finally {
        setResettingProviderIds((prev) => {
          const next = new Set(prev);
          next.delete(providerId);
          return next;
        });
      }
    },
    [resettingProviderIds, refreshOpenCircuits]
  );

  useEffect(() => {
    void refreshOpenCircuits();
  }, [refreshOpenCircuits]);

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
          scheduleRefreshOpenCircuits();
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
    };
  }, [scheduleRefreshOpenCircuits]);

  useEffect(() => {
    if (openCircuitsAutoRefreshTimerRef.current != null) {
      window.clearTimeout(openCircuitsAutoRefreshTimerRef.current);
      openCircuitsAutoRefreshTimerRef.current = null;
    }

    if (openCircuits.length === 0) return;

    const nowUnix = Math.floor(Date.now() / 1000);
    let nextOpenUntil: number | null = null;
    for (const row of openCircuits) {
      const until = row.open_until;
      if (until == null) continue;
      if (nextOpenUntil == null || until < nextOpenUntil) nextOpenUntil = until;
    }

    const delayMs =
      nextOpenUntil != null ? Math.max(200, (nextOpenUntil - nowUnix) * 1000 + 250) : 30_000;

    openCircuitsAutoRefreshTimerRef.current = window.setTimeout(() => {
      openCircuitsAutoRefreshTimerRef.current = null;
      void refreshOpenCircuits();
    }, delayMs);

    return () => {
      if (openCircuitsAutoRefreshTimerRef.current != null) {
        window.clearTimeout(openCircuitsAutoRefreshTimerRef.current);
        openCircuitsAutoRefreshTimerRef.current = null;
      }
    };
  }, [openCircuits, refreshOpenCircuits]);

  useEffect(() => {
    let cancelled = false;
    setSortModesLoading(true);
    Promise.all([sortModesList(), sortModeActiveList()])
      .then(([modes, active]) => {
        if (cancelled) return;
        if (!modes || !active) {
          setSortModesAvailable(false);
          setSortModes([]);
          setActiveModeByCli({ claude: null, codex: null, gemini: null });
          return;
        }

        setSortModesAvailable(true);
        setSortModes(modes);

        const nextActive: Record<CliKey, number | null> = {
          claude: null,
          codex: null,
          gemini: null,
        };
        for (const row of active) {
          nextActive[row.cli_key] = row.mode_id ?? null;
        }
        setActiveModeByCli(nextActive);
      })
      .catch((err) => {
        if (cancelled) return;
        setSortModesAvailable(true);
        setSortModes([]);
        setActiveModeByCli({ claude: null, codex: null, gemini: null });
        logToConsole("error", "读取排序模板失败", { error: String(err) });
        toast(`读取排序模板失败：${String(err)}`);
      })
      .finally(() => {
        if (cancelled) return;
        setSortModesLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  function setCliActiveMode(cliKey: CliKey, modeId: number | null) {
    if (activeModeToggling[cliKey]) return;

    const prev = activeModeByCli[cliKey];
    if (prev === modeId) return;

    setActiveModeByCli((cur) => ({ ...cur, [cliKey]: modeId }));
    setActiveModeToggling((cur) => ({ ...cur, [cliKey]: true }));

    sortModeActiveSet({ cli_key: cliKey, mode_id: modeId })
      .then((res) => {
        if (!res) {
          toast("仅在 Tauri Desktop 环境可用");
          setActiveModeByCli((cur) => ({ ...cur, [cliKey]: prev }));
          return;
        }

        const next = res.mode_id ?? null;
        setActiveModeByCli((cur) => ({ ...cur, [cliKey]: next }));
        if (next == null) {
          toast("已切回：Default");
          return;
        }
        const label = sortModes.find((m) => m.id === next)?.name ?? `#${next}`;
        toast(`已激活：${label}`);
      })
      .catch((err) => {
        toast(`切换排序模板失败：${String(err)}`);
        logToConsole("error", "切换排序模板失败", {
          cli: cliKey,
          mode_id: modeId,
          error: String(err),
        });
        setActiveModeByCli((cur) => ({ ...cur, [cliKey]: prev }));
      })
      .finally(() => {
        setActiveModeToggling((cur) => ({ ...cur, [cliKey]: false }));
      });
  }

  const refreshUsageHeatmap = useCallback(
    (input?: { silent?: boolean; reason?: UsageHeatmapRefreshReason }) => {
      if (usageHeatmapRefreshInFlightRef.current) return;
      if (!isMountedRef.current) return;
      usageHeatmapRefreshInFlightRef.current = true;

      const reason: UsageHeatmapRefreshReason = input?.reason ?? "manual";
      const silent = Boolean(input?.silent);
      const logTitle =
        reason === "initial"
          ? "加载用量热力图失败"
          : reason === "foreground"
            ? "前台自动刷新用量失败"
            : reason === "tab"
              ? "切回概览自动刷新用量失败"
              : "刷新用量热力图失败";
      const toastText =
        reason === "initial" ? "加载用量失败：请查看控制台日志" : "刷新用量失败：请查看控制台日志";

      setUsageHeatmapLoading(true);
      usageHourlySeries(15)
        .then((rows) => {
          if (!isMountedRef.current) return;
          if (!rows) {
            setUsageHeatmapAvailable(false);
            if (!silent) {
              setUsageHeatmapRows([]);
            }
            return;
          }
          setUsageHeatmapAvailable(true);
          setUsageHeatmapRows(rows);
        })
        .catch((err) => {
          logToConsole(reason === "foreground" || reason === "tab" ? "warn" : "error", logTitle, {
            error: String(err),
            reason,
          });
          if (!isMountedRef.current) return;
          setUsageHeatmapAvailable(true);
          if (!silent) setUsageHeatmapRows([]);
          if (!silent) toast(toastText);
        })
        .finally(() => {
          usageHeatmapRefreshInFlightRef.current = false;
          if (!isMountedRef.current) return;
          setUsageHeatmapLoading(false);
        });
    },
    []
  );

  useEffect(() => {
    refreshUsageHeatmap({ reason: "initial" });
  }, [refreshUsageHeatmap]);

  useEffect(() => {
    const prev = tabRef.current;
    tabRef.current = tab;
    if (!tauriRuntime) return;
    if (prev !== "overview" && tab === "overview") {
      refreshUsageHeatmap({ silent: true, reason: "tab" });
    }
  }, [tab, tauriRuntime, refreshUsageHeatmap]);

  useWindowForeground({
    enabled: tauriRuntime && tab === "overview",
    throttleMs: 1000,
    onForeground: () => refreshUsageHeatmap({ silent: true, reason: "foreground" }),
  });

  useEffect(() => {
    let cancelled = false;
    let timer: number | null = null;

    const load = async (showLoading: boolean) => {
      if (showLoading) setActiveSessionsLoading(true);
      try {
        const rows = await gatewaySessionsList(50);
        if (cancelled) return;

        if (!rows) {
          setActiveSessionsAvailable(false);
          setActiveSessions([]);
          if (timer != null) {
            window.clearInterval(timer);
            timer = null;
          }
          return;
        }

        setActiveSessionsAvailable(true);
        setActiveSessions(rows);
      } catch (err) {
        if (cancelled) return;
        if (showLoading) {
          logToConsole("warn", "读取活跃 Session 失败", { error: String(err) });
        }
        setActiveSessionsAvailable(true);
        setActiveSessions([]);
      } finally {
        if (showLoading && !cancelled) setActiveSessionsLoading(false);
      }
    };

    void load(true);
    timer = window.setInterval(() => void load(false), 5000);

    return () => {
      cancelled = true;
      if (timer != null) window.clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    requestLogsRef.current = requestLogs;
  }, [requestLogs]);

  function requestLogCreatedAtMs(log: Pick<RequestLogSummary, "created_at" | "created_at_ms">) {
    const ms = log.created_at_ms ?? 0;
    if (Number.isFinite(ms) && ms > 0) return ms;
    return log.created_at * 1000;
  }

  function sortRequestLogsDesc(a: RequestLogSummary, b: RequestLogSummary) {
    const aTsMs = requestLogCreatedAtMs(a);
    const bTsMs = requestLogCreatedAtMs(b);
    if (aTsMs !== bTsMs) return bTsMs - aTsMs;
    return b.id - a.id;
  }

  function computeRequestLogsCursorId(rows: RequestLogSummary[]) {
    let maxId = 0;
    for (const row of rows) {
      if (Number.isFinite(row.id) && row.id > maxId) maxId = row.id;
    }
    return maxId;
  }

  function mergeRequestLogs(prev: RequestLogSummary[], incoming: RequestLogSummary[]) {
    const byId = new Map<number, RequestLogSummary>();
    for (const row of incoming) byId.set(row.id, row);
    for (const row of prev) {
      if (!byId.has(row.id)) byId.set(row.id, row);
    }
    const merged = Array.from(byId.values());
    merged.sort(sortRequestLogsDesc);
    return merged.slice(0, 50);
  }

  async function refreshRequestLogs(mode: "blocking" | "background" = "blocking") {
    if (requestLogsInFlightRef.current) return;
    requestLogsInFlightRef.current = true;
    if (mode === "blocking") setRequestLogsLoading(true);
    if (mode === "background") setRequestLogsRefreshing(true);

    try {
      const items = await requestLogsListAll(50);
      if (!items) {
        setRequestLogsAvailable(false);
        setRequestLogs([]);
        return;
      }
      setRequestLogsAvailable(true);
      setRequestLogs(items);
    } catch (err) {
      setRequestLogsAvailable(true);
      logToConsole("error", "读取使用记录失败", {
        error: String(err),
      });
      toast("读取使用记录失败：请查看控制台日志");
    } finally {
      requestLogsInFlightRef.current = false;
      if (mode === "blocking") setRequestLogsLoading(false);
      if (mode === "background") setRequestLogsRefreshing(false);
    }
  }

  async function refreshRequestLogsIncremental() {
    const prev = requestLogsRef.current;
    if (prev.length === 0) {
      await refreshRequestLogs("background");
      return;
    }

    if (requestLogsInFlightRef.current) return;
    requestLogsInFlightRef.current = true;
    setRequestLogsRefreshing(true);

    try {
      const afterId = computeRequestLogsCursorId(prev);
      const items = await requestLogsListAfterIdAll(afterId, 50);
      if (!items) {
        setRequestLogsAvailable(false);
        setRequestLogs([]);
        return;
      }
      setRequestLogsAvailable(true);
      const incoming = items ?? [];
      if (incoming.length === 0) return;
      setRequestLogs((cur) => {
        const merged = mergeRequestLogs(cur, incoming);
        if (merged.length === cur.length && merged.every((row, idx) => row.id === cur[idx]?.id)) {
          return cur;
        }
        return merged;
      });
    } catch (err) {
      logToConsole("warn", "增量刷新使用记录失败", {
        error: String(err),
      });
    } finally {
      requestLogsInFlightRef.current = false;
      setRequestLogsRefreshing(false);
    }
  }

  useEffect(() => {
    setSelectedLogId(null);
    setSelectedLog(null);
    requestLogsInFlightRef.current = false;
    setRequestLogsLoading(false);
    setRequestLogsRefreshing(false);
    setRequestLogs([]);
    void refreshRequestLogs("blocking");
  }, []);

  const completedTraceMarks = useMemo(() => {
    return traces
      .filter((t) => Boolean(t.summary))
      .map((t) => `${t.cli_key}:${t.trace_id}:${t.last_seen_ms}`);
  }, [traces]);

  useEffect(() => {
    if (!initializedTraceSeenRef.current) {
      initializedTraceSeenRef.current = true;
      completedTraceIdsSeenRef.current = new Set(completedTraceMarks);
      return;
    }

    let hasNewCompletion = false;
    const seen = completedTraceIdsSeenRef.current;
    for (const mark of completedTraceMarks) {
      if (seen.has(mark)) continue;
      seen.add(mark);
      hasNewCompletion = true;
    }
    if (!hasNewCompletion) return;

    const now = Date.now();
    const nextHoldUntil = Math.max(
      realtimeExitHoldUntilRef.current,
      now + REALTIME_TRACE_EXIT_TOTAL_MS
    );
    realtimeExitHoldUntilRef.current = nextHoldUntil;
    setRealtimeExitHoldUntilMs(nextHoldUntil);

    if (realtimeExitHoldTimerRef.current != null) {
      window.clearTimeout(realtimeExitHoldTimerRef.current);
    }
    const holdDelayMs = Math.max(200, nextHoldUntil - now + 50);
    realtimeExitHoldTimerRef.current = window.setTimeout(() => {
      realtimeExitHoldTimerRef.current = null;
      if (realtimeExitHoldUntilRef.current !== nextHoldUntil) return;
      realtimeExitHoldUntilRef.current = 0;
      setRealtimeExitHoldUntilMs(0);
    }, holdDelayMs);

    if (requestLogsAutoRefreshTimerRef.current != null) {
      window.clearTimeout(requestLogsAutoRefreshTimerRef.current);
    }
    requestLogsAutoRefreshTimerRef.current = window.setTimeout(() => {
      requestLogsAutoRefreshTimerRef.current = null;
      void refreshRequestLogsIncremental();
    }, 700);
  }, [completedTraceMarks]);

  useEffect(() => {
    if (selectedLogId == null) {
      setSelectedLog(null);
      setSelectedLogLoading(false);
      setAttemptLogs([]);
      setAttemptLogsLoading(false);
      return;
    }

    let cancelled = false;
    setSelectedLogLoading(true);
    requestLogGet(selectedLogId)
      .then((detail) => {
        if (cancelled) return;
        if (!detail) {
          setSelectedLog(null);
          return;
        }
        setSelectedLog(detail);
      })
      .catch((err) => {
        if (cancelled) return;
        logToConsole("error", "读取使用记录详情失败", {
          log_id: selectedLogId,
          error: String(err),
        });
        toast(`读取详情失败：${String(err)}`);
      })
      .finally(() => {
        if (cancelled) return;
        setSelectedLogLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [selectedLogId]);

  useEffect(() => {
    if (!selectedLog) {
      setAttemptLogs([]);
      setAttemptLogsLoading(false);
      return;
    }

    let cancelled = false;
    setAttemptLogsLoading(true);

    requestAttemptLogsByTraceId(selectedLog.trace_id, 50)
      .then((items) => {
        if (cancelled) return;
        setAttemptLogs(items ?? []);
      })
      .catch((err) => {
        if (cancelled) return;
        logToConsole("error", "读取 attempt logs 失败", {
          trace_id: selectedLog.trace_id,
          error: String(err),
        });
        setAttemptLogs([]);
      })
      .finally(() => {
        if (cancelled) return;
        setAttemptLogsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [selectedLog]);

  return (
    <div className="flex flex-col gap-6 pb-10">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="h-8 w-1 rounded-full bg-gradient-to-b from-accent to-accent-secondary" />
          <h1 className="text-2xl font-semibold tracking-tight text-slate-900">首页</h1>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <ProviderCircuitBadge
            rows={openCircuits}
            onResetProvider={handleResetProvider}
            resettingProviderIds={resettingProviderIds}
          />
          <TabList ariaLabel="首页视图切换" items={HOME_TABS} value={tab} onChange={setTab} />
        </div>
      </div>

      {tab === "overview" ? (
        <HomeOverviewPanel
          showCustomTooltip={showCustomTooltip}
          usageHeatmapRows={usageHeatmapRows}
          usageHeatmapLoading={usageHeatmapLoading}
          onRefreshUsageHeatmap={refreshUsageHeatmap}
          sortModes={sortModes}
          sortModesLoading={sortModesLoading}
          sortModesAvailable={sortModesAvailable}
          activeModeByCli={activeModeByCli}
          activeModeToggling={activeModeToggling}
          onSetCliActiveMode={setCliActiveMode}
          cliProxyEnabled={cliProxy.enabled}
          cliProxyToggling={cliProxy.toggling}
          onSetCliProxyEnabled={cliProxy.setCliProxyEnabled}
          activeSessions={activeSessions}
          activeSessionsLoading={activeSessionsLoading}
          activeSessionsAvailable={activeSessionsAvailable}
          traces={traces}
          requestLogs={requestLogs}
          requestLogsLoading={requestLogsLoading}
          requestLogsRefreshing={requestLogsRefreshing}
          requestLogsAvailable={requestLogsAvailable}
          onRefreshRequestLogs={() => void refreshRequestLogs("blocking")}
          selectedLogId={selectedLogId}
          onSelectLogId={setSelectedLogId}
        />
      ) : tab === "cost" ? (
        <HomeCostPanel onSelectLogId={setSelectedLogId} />
      ) : (
        <Card padding="md">
          <div className="text-sm text-slate-600">更多功能开发中…</div>
        </Card>
      )}

      <RequestLogDetailDialog
        selectedLogId={selectedLogId}
        onSelectLogId={setSelectedLogId}
        selectedLog={selectedLog}
        selectedLogLoading={selectedLogLoading}
        attemptLogs={attemptLogs}
        attemptLogsLoading={attemptLogsLoading}
      />
    </div>
  );
}

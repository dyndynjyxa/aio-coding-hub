import { commands, type AppMemoryDiagnosticsSnapshot } from "../../generated/bindings";
import { queryClient } from "../../query/queryClient";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import { logToConsole } from "../consoleLog";

const ESTIMATE_MAX_NODES = 200_000;
const TOP_QUERY_LIMIT = 20;

export type { AppMemoryDiagnosticsSnapshot };

type SizeEstimate = {
  bytes: number;
  truncated: boolean;
};

type FrontendQueryDiagnostic = {
  query_hash: string;
  query_key: string;
  status: string;
  fetch_status: string;
  observers: number | null;
  estimated_bytes: number;
  truncated: boolean;
};

type FrontendQueryGroupDiagnostic = {
  key: string;
  count: number;
  estimated_bytes: number;
};

export type AppMemoryDiagnosticsReport = {
  backend: AppMemoryDiagnosticsSnapshot;
  frontend: {
    href: string;
    query_count: number;
    query_estimated_bytes: number;
    query_groups: FrontendQueryGroupDiagnostic[];
    top_queries: FrontendQueryDiagnostic[];
    js_heap?: {
      used_js_heap_size?: number;
      total_js_heap_size?: number;
      js_heap_size_limit?: number;
    };
  };
};

function estimateValueSize(value: unknown): SizeEstimate {
  const seen = new WeakSet<object>();
  let nodes = 0;
  let truncated = false;

  function walk(current: unknown): number {
    nodes += 1;
    if (nodes > ESTIMATE_MAX_NODES) {
      truncated = true;
      return 0;
    }

    if (current == null) return 4;
    if (typeof current === "string") return current.length * 2;
    if (typeof current === "number") return 8;
    if (typeof current === "boolean") return 4;
    if (typeof current === "bigint") return 8;
    if (typeof current !== "object") return 0;
    if (seen.has(current)) return 0;

    seen.add(current);

    if (Array.isArray(current)) {
      let size = 16;
      for (const item of current) {
        size += 8 + walk(item);
        if (truncated) break;
      }
      return size;
    }

    let size = 32;
    for (const [key, item] of Object.entries(current as Record<string, unknown>)) {
      size += key.length * 2 + 8 + walk(item);
      if (truncated) break;
    }
    return size;
  }

  return {
    bytes: walk(value),
    truncated,
  };
}

function safeStringifyKey(value: unknown): string {
  try {
    const text = JSON.stringify(value);
    if (!text) return String(value);
    return text.length > 512 ? `${text.slice(0, 512)}[Truncated]` : text;
  } catch {
    return String(value);
  }
}

function queryGroupKey(queryKey: unknown): string {
  if (Array.isArray(queryKey)) {
    return typeof queryKey[0] === "string" ? queryKey[0] : "unknown";
  }
  return "unknown";
}

function readJsHeap() {
  const memory = (
    performance as Performance & {
      memory?: {
        usedJSHeapSize?: number;
        totalJSHeapSize?: number;
        jsHeapSizeLimit?: number;
      };
    }
  ).memory;
  if (!memory) return undefined;
  return {
    used_js_heap_size: memory.usedJSHeapSize,
    total_js_heap_size: memory.totalJSHeapSize,
    js_heap_size_limit: memory.jsHeapSizeLimit,
  };
}

function collectFrontendDiagnostics(): AppMemoryDiagnosticsReport["frontend"] {
  const queries = queryClient.getQueryCache().getAll();
  const topQueries: FrontendQueryDiagnostic[] = [];
  const groups = new Map<string, FrontendQueryGroupDiagnostic>();
  let queryEstimatedBytes = 0;

  for (const query of queries) {
    const estimate = estimateValueSize(query.state.data);
    queryEstimatedBytes += estimate.bytes;

    const groupKey = queryGroupKey(query.queryKey);
    const group = groups.get(groupKey) ?? {
      key: groupKey,
      count: 0,
      estimated_bytes: 0,
    };
    group.count += 1;
    group.estimated_bytes += estimate.bytes;
    groups.set(groupKey, group);

    const observers =
      typeof query.getObserversCount === "function" ? query.getObserversCount() : null;
    topQueries.push({
      query_hash: query.queryHash,
      query_key: safeStringifyKey(query.queryKey),
      status: String(query.state.status),
      fetch_status: String(query.state.fetchStatus),
      observers,
      estimated_bytes: estimate.bytes,
      truncated: estimate.truncated,
    });
  }

  topQueries.sort((a, b) => b.estimated_bytes - a.estimated_bytes);
  const queryGroups = Array.from(groups.values()).sort(
    (a, b) => b.estimated_bytes - a.estimated_bytes
  );

  return {
    href: typeof window === "undefined" ? "" : window.location.href,
    query_count: queries.length,
    query_estimated_bytes: queryEstimatedBytes,
    query_groups: queryGroups,
    top_queries: topQueries.slice(0, TOP_QUERY_LIMIT),
    js_heap: typeof performance === "undefined" ? undefined : readJsHeap(),
  };
}

export async function appMemoryDiagnosticsGet() {
  return invokeGeneratedIpc<AppMemoryDiagnosticsSnapshot>({
    title: "采集后端内存诊断失败",
    cmd: "app_memory_diagnostics_get",
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.appMemoryDiagnosticsGet(),
        (value) => value as AppMemoryDiagnosticsSnapshot
      ),
  });
}

export async function collectAppMemoryDiagnostics(): Promise<AppMemoryDiagnosticsReport> {
  const backend = await appMemoryDiagnosticsGet();
  const report: AppMemoryDiagnosticsReport = {
    backend,
    frontend: collectFrontendDiagnostics(),
  };

  logToConsole("info", "内存诊断快照已生成", report);
  return report;
}

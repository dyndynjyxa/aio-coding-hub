/**
 * 任务结束提醒模块
 *
 * 监听 gateway:request / gateway:request_start 事件，使用去抖机制检测 AI CLI 任务完成。
 * 当某个 cli_key 在静默期（QUIET_PERIOD_MS_DEFAULT）内无新请求时，判定任务完成并发送系统通知。
 *
 * 参考：https://github.com/ZekerTop/ai-cli-complete-notify
 */

import { useSyncExternalStore } from "react";
import { cliShortLabel } from "../../constants/clis";
import { gatewayEventNames } from "../../constants/gatewayEvents";
import { logToConsole } from "../consoleLog";
import { subscribeGatewayEvent } from "../gateway/gatewayEventBus";
import { noticeSend } from "./notice";
import type { GatewayRequestEvent, GatewayRequestStartEvent } from "../gateway/gatewayEvents";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** 静默期：最后一次请求完成后等待多久判定任务结束（ms） */
const QUIET_PERIOD_MS_DEFAULT = 30_000;

// ---------------------------------------------------------------------------
// Module-level enabled flag (reactive via useSyncExternalStore)
// ---------------------------------------------------------------------------

let enabled = true;
const subscribers = new Set<() => void>();

function notifySubscribers() {
  for (const fn of subscribers) fn();
}

export function setTaskCompleteNotifyEnabled(value: boolean) {
  const next = value === true;
  if (enabled === next) return;
  enabled = next;
  if (!enabled) resetSessions();
  notifySubscribers();
}

export function getTaskCompleteNotifyEnabled(): boolean {
  return enabled;
}

function subscribeEnabled(callback: () => void): () => void {
  subscribers.add(callback);
  return () => subscribers.delete(callback);
}

/** React hook：读取当前 enabled 状态 */
export function useTaskCompleteNotifyEnabled(): boolean {
  return useSyncExternalStore(subscribeEnabled, getTaskCompleteNotifyEnabled);
}

// ---------------------------------------------------------------------------
// Session state per cli_key
// ---------------------------------------------------------------------------

type SessionState = {
  /** 本轮会话首个请求完成时间戳 (ms) */
  firstRequestAt: number;
  /** 本轮会话最后一个请求完成时间戳 (ms) */
  lastRequestAt: number;
  /** 本轮会话请求计数 */
  requestCount: number;
  /**
   * 当前 in-flight 请求集合（按 trace_id 去重）。
   *
   * 关键点：
   * - 必须按 trace_id 追踪，否则当出现“request 完成事件但缺失对应 start 事件”
   *   （例如某些早期错误路径未 emit request_start）时，会把其它正在 in-flight 的请求错误减到 0，
   *   导致静默定时器误触发通知。
   */
  inFlightTraceIds: Set<string>;
  /** 最后使用的模型名（来自 request_start） */
  lastRequestedModel: string | null;
  /** 去抖定时器 ID */
  pendingTimer: ReturnType<typeof setTimeout> | null;
  /** 本轮是否已发送通知（避免重复通知） */
  notified: boolean;
};

const sessions = new Map<string, SessionState>();

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function cliKeyDisplayName(cliKey: string): string {
  return cliShortLabel(cliKey);
}

function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  if (totalSeconds < 60) return `${totalSeconds} 秒`;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes < 60) return seconds > 0 ? `${minutes} 分 ${seconds} 秒` : `${minutes} 分`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return remainingMinutes > 0 ? `${hours} 时 ${remainingMinutes} 分` : `${hours} 时`;
}

function normalizeModelName(value: unknown): string | null {
  const raw = typeof value === "string" ? value : "";
  const trimmed = raw.trim();
  if (!trimmed) return null;
  return trimmed.length > 200 ? trimmed.slice(0, 200) : trimmed;
}

function resetSessions() {
  for (const session of sessions.values()) {
    if (session.pendingTimer != null) clearTimeout(session.pendingTimer);
  }
  sessions.clear();
}

// ---------------------------------------------------------------------------
// Core logic
// ---------------------------------------------------------------------------

function handleRequestStart(payload: GatewayRequestStartEvent) {
  if (!enabled) return;

  const { cli_key, requested_model, trace_id } = payload;
  const now = Date.now();
  let session = sessions.get(cli_key);

  if (session?.notified) {
    // 如果上一轮已通知，说明用户开始了新任务 → 重置会话
    if (session.pendingTimer != null) clearTimeout(session.pendingTimer);
    sessions.delete(cli_key);
    session = undefined;
  }

  if (!session) {
    session = {
      firstRequestAt: now,
      lastRequestAt: now,
      requestCount: 0,
      inFlightTraceIds: new Set(),
      lastRequestedModel: null,
      pendingTimer: null,
      notified: false,
    };
    sessions.set(cli_key, session);
  }

  // 只要有新请求开始，就应该取消“静默结束”定时器，避免长请求/并发请求误触发通知。
  if (session.pendingTimer != null) {
    clearTimeout(session.pendingTimer);
    session.pendingTimer = null;
  }

  if (trace_id) session.inFlightTraceIds.add(trace_id);

  const model = normalizeModelName(requested_model);
  if (model) session.lastRequestedModel = model;
}

function handleRequestComplete(payload: GatewayRequestEvent) {
  if (!enabled) return;

  const { cli_key, trace_id } = payload;
  const now = Date.now();

  let session = sessions.get(cli_key);
  if (!session) {
    session = {
      firstRequestAt: now,
      lastRequestAt: now,
      requestCount: 0,
      inFlightTraceIds: new Set(),
      lastRequestedModel: null,
      pendingTimer: null,
      notified: false,
    };
    sessions.set(cli_key, session);
  }

  session.lastRequestAt = now;
  session.requestCount += 1;
  session.notified = false;
  if (trace_id) session.inFlightTraceIds.delete(trace_id);

  // 清除旧定时器（若仍有 in-flight 请求，不应开启静默结束倒计时）
  if (session.pendingTimer != null) {
    clearTimeout(session.pendingTimer);
    session.pendingTimer = null;
  }

  if (session.inFlightTraceIds.size === 0) {
    session.pendingTimer = setTimeout(() => {
      void maybeNotify(cli_key);
    }, QUIET_PERIOD_MS_DEFAULT);
  }
}

async function maybeNotify(cliKey: string) {
  const session = sessions.get(cliKey);
  if (!session) return;
  if (session.notified) return;
  if (session.inFlightTraceIds.size > 0) return;

  // 检查 enabled 标志（实时生效）
  if (!enabled) {
    session.pendingTimer = null;
    return;
  }

  session.notified = true;
  session.pendingTimer = null;

  const durationMs = session.lastRequestAt - session.firstRequestAt;
  const durationText = formatDuration(durationMs);
  const displayName = cliKeyDisplayName(cliKey);
  const requestCount = session.requestCount;
  const modelSuffix = session.lastRequestedModel ? `（${session.lastRequestedModel}）` : "";

  const body =
    requestCount === 1
      ? `${displayName} 请求已完成${modelSuffix}`
      : `${displayName} 会话已结束，共 ${requestCount} 次请求，耗时 ${durationText}${modelSuffix}`;

  try {
    await noticeSend({
      level: "info",
      title: "任务完成",
      body,
    });
  } catch (err) {
    logToConsole("warn", "发送任务结束通知失败", { error: String(err) });
  }
}

// ---------------------------------------------------------------------------
// Listener lifecycle
// ---------------------------------------------------------------------------

export async function listenTaskCompleteNotifyEvents(): Promise<() => void> {
  const requestStartSub = subscribeGatewayEvent<GatewayRequestStartEvent>(
    gatewayEventNames.requestStart,
    (payload) => {
      if (!payload) return;
      handleRequestStart(payload);
    }
  );
  const requestSub = subscribeGatewayEvent<GatewayRequestEvent>(
    gatewayEventNames.request,
    (payload) => {
      if (!payload) return;
      handleRequestComplete(payload);
    }
  );
  const readyResults = await Promise.allSettled([requestStartSub.ready, requestSub.ready]);
  const subscribeFailed = readyResults.some((result) => result.status === "rejected");
  if (subscribeFailed) {
    requestStartSub.unsubscribe();
    requestSub.unsubscribe();
    const failedResult = readyResults.find((result) => result.status === "rejected");
    throw failedResult?.reason ?? new Error("task complete notify subscriptions failed");
  }

  return () => {
    requestStartSub.unsubscribe();
    requestSub.unsubscribe();
    resetSessions();
  };
}

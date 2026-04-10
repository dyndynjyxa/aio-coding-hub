import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../../constants/gatewayEvents";
import { clearTauriEventListeners, emitTauriEvent } from "../../../test/mocks/tauri";
import { clearTauriRuntime, setTauriRuntime } from "../../../test/utils/tauriRuntime";

vi.mock("../notice", () => ({ noticeSend: vi.fn() }));

async function importFreshTaskCompleteNotify() {
  vi.resetModules();
  const mod = await import("../taskCompleteNotifyEvents");
  const notice = await import("../notice");
  return { mod, noticeSend: vi.mocked(notice.noticeSend) };
}

function requestStartWithTrace(cliKey: string, traceId: string, model?: string | null) {
  return {
    trace_id: traceId,
    cli_key: cliKey,
    method: "POST",
    path: "/v1/messages",
    query: null,
    requested_model: model,
    ts: 0,
  } as any;
}

function requestEvent(cliKey: string, traceId = "t-1") {
  return {
    trace_id: traceId,
    cli_key: cliKey,
  } as any;
}

describe("services/notification/taskCompleteNotifyEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearTauriEventListeners();
    clearTauriRuntime();
  });

  it("defaults enabled and notifies subscribers", async () => {
    vi.useFakeTimers();

    const { mod } = await importFreshTaskCompleteNotify();

    expect(mod.getTaskCompleteNotifyEnabled()).toBe(true);

    const { result } = renderHook(() => mod.useTaskCompleteNotifyEnabled());
    expect(result.current).toBe(true);

    act(() => mod.setTaskCompleteNotifyEnabled(false));
    expect(result.current).toBe(false);

    act(() => mod.setTaskCompleteNotifyEnabled(true));
    expect(result.current).toBe(true);

    vi.useRealTimers();
  });

  it("ignores gateway events without payload", async () => {
    setTauriRuntime();
    vi.useFakeTimers();

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockResolvedValue(true);

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    emitTauriEvent(gatewayEventNames.requestStart, null);
    emitTauriEvent(gatewayEventNames.request, null);

    await vi.advanceTimersByTimeAsync(200_000);
    expect(noticeSend).not.toHaveBeenCalled();

    cleanup();
    vi.useRealTimers();
  });

  it("sends notification after quiet period when enabled", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockResolvedValue(true);

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    emitTauriEvent(
      gatewayEventNames.requestStart,
      requestStartWithTrace("claude", "t-1", "claude-3-5-sonnet")
    );
    emitTauriEvent(gatewayEventNames.request, requestEvent("claude", "t-1"));

    await vi.advanceTimersByTimeAsync(30_000);

    expect(noticeSend).toHaveBeenCalledTimes(1);
    expect(noticeSend).toHaveBeenCalledWith({
      level: "info",
      title: "任务完成",
      body: expect.stringContaining("Claude Code 请求已完成"),
    });

    cleanup();
    vi.useRealTimers();
  });

  it("covers gemini session end formatting and notice failure handling", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockRejectedValue(new Error("notice boom"));

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    // Intentionally omit request_start to cover session creation in request handler.
    emitTauriEvent(gatewayEventNames.request, requestEvent("gemini", "t-1"));

    // Update the timestamp so duration formatting hits the >= 60s branch.
    vi.setSystemTime(1_700_000_065_000);
    emitTauriEvent(gatewayEventNames.request, requestEvent("gemini", "t-2"));

    await vi.advanceTimersByTimeAsync(30_000);

    expect(noticeSend).toHaveBeenCalledTimes(1);
    expect(noticeSend).toHaveBeenCalledWith({
      level: "info",
      title: "任务完成",
      body: expect.stringContaining("Gemini 会话已结束"),
    });

    cleanup();
    vi.useRealTimers();
  });

  it("does not notify when disabled", async () => {
    setTauriRuntime();
    vi.useFakeTimers();

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockResolvedValue(true);

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    mod.setTaskCompleteNotifyEnabled(false);

    emitTauriEvent(
      gatewayEventNames.requestStart,
      requestStartWithTrace("claude", "t-1", "claude-3-5-sonnet")
    );
    emitTauriEvent(gatewayEventNames.request, requestEvent("claude", "t-1"));

    await vi.advanceTimersByTimeAsync(30_000);

    expect(noticeSend).not.toHaveBeenCalled();

    cleanup();
    vi.useRealTimers();
  });

  it("avoids false positives for overlapping requests and only notifies when idle", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockResolvedValue(true);

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    // Two overlapping requests: should NOT notify after first completion.
    emitTauriEvent(
      gatewayEventNames.requestStart,
      requestStartWithTrace("claude", "t-1", "claude-3-5-sonnet")
    );
    emitTauriEvent(
      gatewayEventNames.requestStart,
      requestStartWithTrace("claude", "t-2", "claude-3-5-sonnet")
    );
    emitTauriEvent(gatewayEventNames.request, requestEvent("claude", "t-1"));

    await vi.advanceTimersByTimeAsync(30_000);
    expect(noticeSend).not.toHaveBeenCalled();

    // Finish the second request; after quiet period, it should notify once.
    emitTauriEvent(gatewayEventNames.request, requestEvent("claude", "t-2"));
    await vi.advanceTimersByTimeAsync(30_000);

    expect(noticeSend).toHaveBeenCalledTimes(1);

    cleanup();
    vi.useRealTimers();
  });

  it("cancels pending notify timer when a new request starts during quiet period", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockResolvedValue(true);

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    emitTauriEvent(
      gatewayEventNames.requestStart,
      requestStartWithTrace("claude", "t-1", "claude-3-5-sonnet")
    );
    emitTauriEvent(gatewayEventNames.request, requestEvent("claude", "t-1"));

    // Quiet timer is scheduled for 30s after completion.
    await vi.advanceTimersByTimeAsync(10_000);

    // A new request starts before the timer fires: should cancel the pending timer.
    emitTauriEvent(
      gatewayEventNames.requestStart,
      requestStartWithTrace("claude", "t-2", "claude-3-5-sonnet")
    );

    // Advance past the original 30s window; should not notify while request is in-flight.
    await vi.advanceTimersByTimeAsync(25_000);
    expect(noticeSend).not.toHaveBeenCalled();

    // Complete the second request; now quiet period should trigger.
    emitTauriEvent("gateway:request", requestEvent("claude", "t-2"));
    await vi.advanceTimersByTimeAsync(30_000);
    expect(noticeSend).toHaveBeenCalledTimes(1);

    cleanup();
    vi.useRealTimers();
  });

  it("falls back to raw cli key display name for unknown clients", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockResolvedValue(true);

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    emitTauriEvent("gateway:request_start", requestStartWithTrace("unknown", "t-1", "some-model"));
    emitTauriEvent("gateway:request", requestEvent("unknown", "t-1"));

    await vi.advanceTimersByTimeAsync(30_000);

    expect(noticeSend).toHaveBeenCalledTimes(1);
    expect(noticeSend).toHaveBeenCalledWith({
      level: "info",
      title: "任务完成",
      body: expect.stringContaining("unknown 请求已完成"),
    });

    cleanup();
    vi.useRealTimers();
  });

  it("uses default quiet period for codex (no extra delay)", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockResolvedValue(true);

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    emitTauriEvent("gateway:request_start", requestStartWithTrace("codex", "t-1", "gpt-4.1"));
    emitTauriEvent("gateway:request", requestEvent("codex", "t-1"));

    // Quiet period is 30s; verify it doesn't fire early.
    await vi.advanceTimersByTimeAsync(25_000);
    expect(noticeSend).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(5_000);
    expect(noticeSend).toHaveBeenCalledTimes(1);
    expect(noticeSend).toHaveBeenCalledWith({
      level: "info",
      title: "任务完成",
      body: expect.stringContaining("Codex 请求已完成（gpt-4.1）"),
    });

    cleanup();
    vi.useRealTimers();
  });

  it("does not decrement other in-flight requests when a request completes without start", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    vi.setSystemTime(1_700_000_000_000);

    const { mod, noticeSend } = await importFreshTaskCompleteNotify();
    noticeSend.mockResolvedValue(true);

    const cleanup = await mod.listenTaskCompleteNotifyEvents();

    // One long-running request in-flight.
    emitTauriEvent(
      "gateway:request_start",
      requestStartWithTrace("claude", "t-1", "claude-3-5-sonnet")
    );

    // Another request completes but its start event was never observed (e.g. early error path).
    emitTauriEvent("gateway:request", requestEvent("claude", "t-2"));

    // Quiet period should NOT trigger because t-1 is still in-flight.
    await vi.advanceTimersByTimeAsync(30_000);
    expect(noticeSend).not.toHaveBeenCalled();

    // Finish the in-flight request; now quiet period should trigger.
    emitTauriEvent("gateway:request", requestEvent("claude", "t-1"));
    await vi.advanceTimersByTimeAsync(30_000);
    expect(noticeSend).toHaveBeenCalledTimes(1);

    cleanup();
    vi.useRealTimers();
  });
});

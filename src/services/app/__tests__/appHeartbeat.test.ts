import { describe, expect, it, vi, beforeEach } from "vitest";
import { appEventNames } from "../../../constants/appEvents";
import { setTauriRuntime, clearTauriRuntime } from "../../../test/utils/tauriRuntime";
import { tauriListen, emitTauriEvent } from "../../../test/mocks/tauri";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      appHeartbeatPong: vi.fn().mockResolvedValue({ status: "ok", data: true }),
    },
  };
});

import { commands } from "../../../generated/bindings";

describe("services/app/appHeartbeat", () => {
  beforeEach(() => {
    clearTauriRuntime();
    vi.mocked(commands.appHeartbeatPong).mockResolvedValue({ status: "ok", data: true } as any);
  });

  async function importFresh() {
    vi.resetModules();
    return await import("../appHeartbeat");
  }

  it("listens to app heartbeat with tauri runtime", async () => {
    setTauriRuntime();
    const { listenAppHeartbeat } = await importFresh();
    const unlisten = await listenAppHeartbeat();

    expect(tauriListen).toHaveBeenCalledWith(appEventNames.heartbeat, expect.any(Function));

    unlisten();
  });

  it("heartbeat event triggers appHeartbeatPong", async () => {
    setTauriRuntime();
    const { listenAppHeartbeat } = await importFresh();
    await listenAppHeartbeat();

    emitTauriEvent(appEventNames.heartbeat, {});

    await vi.waitFor(() => {
      expect(commands.appHeartbeatPong).toHaveBeenCalledWith();
    });
  });

  it("appHeartbeatPong rejection is caught gracefully", async () => {
    setTauriRuntime();
    vi.mocked(commands.appHeartbeatPong).mockRejectedValueOnce(new Error("timeout"));
    const { listenAppHeartbeat } = await importFresh();
    await listenAppHeartbeat();

    emitTauriEvent(appEventNames.heartbeat, {});

    // Should not throw
    await vi.waitFor(() => {
      expect(commands.appHeartbeatPong).toHaveBeenCalled();
    });
  });

  it("skips pong while a recent one is still in flight", async () => {
    setTauriRuntime();
    // A pong that never settles keeps the in-flight guard active.
    vi.mocked(commands.appHeartbeatPong).mockReturnValue(new Promise(() => {}) as any);
    const { listenAppHeartbeat } = await importFresh();
    await listenAppHeartbeat();

    emitTauriEvent(appEventNames.heartbeat, {});
    emitTauriEvent(appEventNames.heartbeat, {});

    expect(commands.appHeartbeatPong).toHaveBeenCalledTimes(1);
  });

  it("stale in-flight pong (stuck IPC) does not block heartbeats forever", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    try {
      // First pong hangs forever (stuck IPC — neither resolves nor rejects).
      vi.mocked(commands.appHeartbeatPong).mockReturnValue(new Promise(() => {}) as any);
      const { listenAppHeartbeat, PONG_IN_FLIGHT_STALE_MS } = await importFresh();
      await listenAppHeartbeat();

      emitTauriEvent(appEventNames.heartbeat, {});
      expect(commands.appHeartbeatPong).toHaveBeenCalledTimes(1);

      // After the stale window the guard is released and pong resumes.
      vi.advanceTimersByTime(PONG_IN_FLIGHT_STALE_MS + 1);
      emitTauriEvent(appEventNames.heartbeat, {});
      expect(commands.appHeartbeatPong).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });

  it("a stale pong settling late does not clear the newer pong's guard", async () => {
    setTauriRuntime();
    vi.useFakeTimers();
    try {
      let resolveFirst: (value: unknown) => void = () => {};
      vi.mocked(commands.appHeartbeatPong)
        .mockReturnValueOnce(
          new Promise((resolve) => {
            resolveFirst = resolve;
          }) as any
        )
        .mockReturnValue(new Promise(() => {}) as any);
      const { listenAppHeartbeat, PONG_IN_FLIGHT_STALE_MS } = await importFresh();
      await listenAppHeartbeat();

      // Pong A goes stale, pong B takes over the guard.
      emitTauriEvent(appEventNames.heartbeat, {});
      vi.advanceTimersByTime(PONG_IN_FLIGHT_STALE_MS + 1);
      emitTauriEvent(appEventNames.heartbeat, {});
      expect(commands.appHeartbeatPong).toHaveBeenCalledTimes(2);

      // A settles late — it must NOT release B's guard.
      resolveFirst({ status: "ok", data: true });
      await vi.advanceTimersByTimeAsync(0);

      // B is still recent, so the next heartbeat is deduped.
      emitTauriEvent(appEventNames.heartbeat, {});
      expect(commands.appHeartbeatPong).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });
});

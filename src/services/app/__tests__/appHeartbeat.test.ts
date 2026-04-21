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
});

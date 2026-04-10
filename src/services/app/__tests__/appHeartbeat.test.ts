import { describe, expect, it, vi, beforeEach } from "vitest";
import { appEventNames } from "../../../constants/appEvents";
import { setTauriRuntime, clearTauriRuntime } from "../../../test/utils/tauriRuntime";
import { tauriListen, emitTauriEvent } from "../../../test/mocks/tauri";

vi.mock("../../tauriInvoke", async () => {
  const actual = await vi.importActual<typeof import("../../tauriInvoke")>("../../tauriInvoke");
  return {
    ...actual,
    invokeTauriOrNull: vi.fn().mockResolvedValue(true),
  };
});

import { invokeTauriOrNull } from "../../tauriInvoke";

describe("services/app/appHeartbeat", () => {
  beforeEach(() => {
    clearTauriRuntime();
    vi.mocked(invokeTauriOrNull).mockResolvedValue(true);
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

  it("heartbeat event triggers invokeTauriOrNull", async () => {
    setTauriRuntime();
    const { listenAppHeartbeat } = await importFresh();
    await listenAppHeartbeat();

    emitTauriEvent(appEventNames.heartbeat, {});

    await vi.waitFor(() => {
      expect(invokeTauriOrNull).toHaveBeenCalledWith("app_heartbeat_pong", undefined, {
        timeoutMs: 3_000,
      });
    });
  });

  it("invokeTauriOrNull rejection is caught gracefully", async () => {
    setTauriRuntime();
    vi.mocked(invokeTauriOrNull).mockRejectedValueOnce(new Error("timeout"));
    const { listenAppHeartbeat } = await importFresh();
    await listenAppHeartbeat();

    emitTauriEvent(appEventNames.heartbeat, {});

    // Should not throw
    await vi.waitFor(() => {
      expect(invokeTauriOrNull).toHaveBeenCalled();
    });
  });
});

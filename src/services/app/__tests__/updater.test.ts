import { beforeEach, describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../../test/mocks/tauri";
import { setTauriRuntime } from "../../../test/utils/tauriRuntime";

describe("services/app/updater", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("parseUpdaterCheckResult rejects invalid values and keeps optional fields", async () => {
    const { parseUpdaterCheckResult } = await import("../updater");

    expect(parseUpdaterCheckResult(null)).toBeNull();
    expect(parseUpdaterCheckResult(false)).toBeNull();
    expect(parseUpdaterCheckResult("x")).toBeNull();
    expect(parseUpdaterCheckResult({})).toBeNull();
    expect(parseUpdaterCheckResult({ rid: "1" })).toBeNull();
    expect(parseUpdaterCheckResult({ rid: -1 })).toBeNull();
    expect(parseUpdaterCheckResult({ rid: 1.5 })).toBeNull();
    expect(parseUpdaterCheckResult({ rid: Number.NaN })).toBeNull();

    expect(
      parseUpdaterCheckResult({
        rid: 1,
        version: "v1",
        currentVersion: "v0",
        date: "2026-02-01",
        body: "notes",
      })
    ).toEqual({
      rid: 1,
      version: "v1",
      currentVersion: "v0",
      date: "2026-02-01",
      body: "notes",
    });
  });

  it("updaterCheck parses tauri result", async () => {
    const { updaterCheck } = await import("../updater");

    setTauriRuntime();

    vi.mocked(tauriInvoke).mockResolvedValueOnce(false as any);
    expect(await updaterCheck()).toBeNull();

    vi.mocked(tauriInvoke).mockResolvedValueOnce({ rid: 2, version: "v2" } as any);
    expect(await updaterCheck()).toEqual({
      rid: 2,
      version: "v2",
      currentVersion: undefined,
      date: undefined,
      body: undefined,
    });
  });

  it("updaterDownloadAndInstall maps events and supports timeout option", async () => {
    const { updaterDownloadAndInstall } = await import("../updater");

    setTauriRuntime();

    const events: any[] = [];
    vi.mocked(tauriInvoke).mockImplementation(async (cmd: string, args?: any) => {
      if (cmd !== "desktop_updater_download_and_install") return null as any;

      const ch = args?.onEvent;
      ch?.__emit?.({ foo: 1 }); // ignored
      ch?.__emit?.({ event: "started", data: { contentLength: 123 } });
      ch?.__emit?.({ event: "progress", data: { chunkLength: 10 } });
      ch?.__emit?.({ event: "progress", data: { chunkLength: "bad" } }); // ignored chunkLength
      ch?.__emit?.({ event: "finished", data: { ok: true } });
      return true as any;
    });

    const ok = await updaterDownloadAndInstall({
      rid: 99,
      timeoutMs: 1234,
      onEvent: (e) => events.push(e),
    });

    expect(ok).toBe(true);
    expect(tauriInvoke).toHaveBeenCalledWith(
      "desktop_updater_download_and_install",
      expect.objectContaining({
        rid: 99,
        timeout: 1234,
        onEvent: expect.anything(),
        confirm: expect.objectContaining({
          confirm: expect.objectContaining({
            action: "desktop_updater_download_and_install",
            resource: "updater:99",
            nonce: expect.any(String),
          }),
        }),
      })
    );

    expect(events).toEqual([
      { event: "started", data: { contentLength: 123 } },
      { event: "progress", data: { chunkLength: 10 } },
      { event: "progress", data: { chunkLength: undefined } },
      { event: "finished", data: { ok: true } },
    ]);
  });

  it("updaterDownloadAndInstall rejects invalid rid and timeout before handwritten IPC", async () => {
    const { updaterDownloadAndInstall } = await import("../updater");
    const { desktopUpdaterCheck } = await import("../../desktop/updater");

    setTauriRuntime();

    await expect(updaterDownloadAndInstall({ rid: -1 })).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(updaterDownloadAndInstall({ rid: 1.5 })).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(updaterDownloadAndInstall({ rid: 1, timeoutMs: 0 })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(desktopUpdaterCheck({ timeoutMs: Number.NaN })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );

    expect(tauriInvoke).not.toHaveBeenCalled();
  });

  it("updaterDownloadAndInstall tolerates missing callback and default timeout branches", async () => {
    const { updaterDownloadAndInstall } = await import("../updater");

    setTauriRuntime();

    vi.mocked(tauriInvoke).mockImplementation(async (cmd: string, args?: any) => {
      if (cmd !== "desktop_updater_download_and_install") return null as any;

      const ch = args?.onEvent;
      ch?.__emit?.({ event: "started", data: "invalid" });
      ch?.__emit?.({ event: "progress", data: null });
      ch?.__emit?.({ event: "finished" });
      return true as any;
    });

    const ok = await updaterDownloadAndInstall({
      rid: 7,
    });

    expect(ok).toBe(true);
    expect(tauriInvoke).toHaveBeenCalledWith(
      "desktop_updater_download_and_install",
      expect.objectContaining({
        rid: 7,
        timeout: null,
        onEvent: expect.anything(),
      })
    );
  });
});

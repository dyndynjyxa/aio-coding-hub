import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../../test/mocks/tauri";
import { setTauriRuntime } from "../../../test/utils/tauriRuntime";

describe("services/app/updater", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
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

  it("updaterCheck replaces GitHub release fallback notes with release body", async () => {
    const { updaterCheck } = await import("../updater");

    setTauriRuntime();

    vi.mocked(tauriInvoke).mockResolvedValueOnce({
      rid: 3,
      version: "0.60.0",
      currentVersion: "0.59.0",
      date: "2026-06-14T15:58:48Z",
      body: "See release: https://github.com/dyndynjyxa/aio-coding-hub/releases/tag/aio-coding-hub-v0.60.0",
    } as any);

    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        body: "## 0.60.0\n\n- 具体更新内容",
      }),
    });
    vi.stubGlobal("fetch", fetchMock);

    await expect(updaterCheck()).resolves.toEqual({
      rid: 3,
      version: "0.60.0",
      currentVersion: "0.59.0",
      date: "2026-06-14T15:58:48Z",
      body: "## 0.60.0\n\n- 具体更新内容",
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.github.com/repos/dyndynjyxa/aio-coding-hub/releases/tags/aio-coding-hub-v0.60.0",
      expect.objectContaining({
        headers: expect.objectContaining({ accept: "application/vnd.github+json" }),
      })
    );
  });

  it("updaterCheck keeps fallback notes when GitHub release body cannot be loaded", async () => {
    const { updaterCheck } = await import("../updater");

    setTauriRuntime();

    const fallbackBody =
      "See release: https://github.com/dyndynjyxa/aio-coding-hub/releases/tag/aio-coding-hub-v0.60.0";
    vi.mocked(tauriInvoke).mockResolvedValueOnce({
      rid: 4,
      version: "0.60.0",
      body: fallbackBody,
    } as any);

    const fetchMock = vi.fn().mockResolvedValue({ ok: false });
    vi.stubGlobal("fetch", fetchMock);

    await expect(updaterCheck()).resolves.toEqual({
      rid: 4,
      version: "0.60.0",
      currentVersion: undefined,
      date: undefined,
      body: fallbackBody,
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

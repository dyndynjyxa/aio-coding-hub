import { describe, expect, it, vi } from "vitest";
import { tauriInvoke } from "../../../test/mocks/tauri";
import { setTauriRuntime } from "../../../test/utils/tauriRuntime";

describe("services/app/updater", () => {
  it("parseUpdaterCheckResult rejects invalid values and keeps optional fields", async () => {
    const { parseUpdaterCheckResult } = await import("../updater");

    expect(parseUpdaterCheckResult(null)).toBeNull();
    expect(parseUpdaterCheckResult(false)).toBeNull();
    expect(parseUpdaterCheckResult("x")).toBeNull();
    expect(parseUpdaterCheckResult({})).toBeNull();
    expect(parseUpdaterCheckResult({ rid: "1" })).toBeNull();

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
      if (cmd !== "plugin:updater|download_and_install") return null as any;

      const ch = args?.onEvent;
      ch?.__emit?.({ foo: 1 }); // ignored
      ch?.__emit?.({ event: "started", data: { contentLength: 123 } });
      ch?.__emit?.({ event: "progress", data: { chunkLength: 10 } });
      ch?.__emit?.({ event: "progress", data: { chunkLength: "bad" } }); // ignored chunkLength
      ch?.__emit?.({ event: "finished", data: { ok: true } });
      return null as any;
    });

    const ok = await updaterDownloadAndInstall({
      rid: 99,
      timeoutMs: 1234,
      onEvent: (e) => events.push(e),
    });

    expect(ok).toBe(true);
    expect(tauriInvoke).toHaveBeenCalledWith(
      "plugin:updater|download_and_install",
      expect.objectContaining({
        rid: 99,
        timeout: 1234,
        onEvent: expect.anything(),
      })
    );

    expect(events).toEqual([
      { event: "started", data: { contentLength: 123 } },
      { event: "progress", data: { chunkLength: 10 } },
      { event: "progress", data: { chunkLength: undefined } },
      { event: "finished", data: { ok: true } },
    ]);
  });
});

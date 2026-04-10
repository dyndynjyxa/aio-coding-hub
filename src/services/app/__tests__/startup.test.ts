import { beforeEach, describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import { modelPricesSyncBasellm, setLastModelPricesSync } from "../../usage/modelPrices";
import { promptsDefaultSyncFromFiles } from "../../workspace/prompts";

vi.mock("../../consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../../usage/modelPrices", async () => {
  const actual =
    await vi.importActual<typeof import("../../usage/modelPrices")>("../../usage/modelPrices");
  return { ...actual, modelPricesSyncBasellm: vi.fn(), setLastModelPricesSync: vi.fn() };
});
vi.mock("../../workspace/prompts", async () => {
  const actual =
    await vi.importActual<typeof import("../../workspace/prompts")>("../../workspace/prompts");
  return { ...actual, promptsDefaultSyncFromFiles: vi.fn() };
});

async function importFreshStartup() {
  vi.resetModules();
  return await import("../startup");
}

describe("services/app/startup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("startupSyncModelPricesOnce always calls modelPricesSyncBasellm", async () => {
    const { startupSyncModelPricesOnce } = await importFreshStartup();

    vi.mocked(modelPricesSyncBasellm).mockResolvedValueOnce({
      status: "updated",
      inserted: 1,
      updated: 2,
      skipped: 3,
      total: 6,
    } as any);

    await startupSyncModelPricesOnce();
    expect(modelPricesSyncBasellm).toHaveBeenCalledWith(false);
    expect(setLastModelPricesSync).toHaveBeenCalledWith({
      status: "updated",
      inserted: 1,
      updated: 2,
      skipped: 3,
      total: 6,
    });
    expect(logToConsole).toHaveBeenCalledWith(
      "info",
      "启动同步：模型定价同步完成",
      expect.objectContaining({ status: "updated", inserted: 1, updated: 2, skipped: 3, total: 6 })
    );
  });

  it("startupSyncModelPricesOnce skips when report is null", async () => {
    const m = await importFreshStartup();
    vi.mocked(modelPricesSyncBasellm).mockResolvedValueOnce(null as any);
    await m.startupSyncModelPricesOnce();
    expect(setLastModelPricesSync).not.toHaveBeenCalled();
    expect(logToConsole).not.toHaveBeenCalledWith("info", expect.anything(), expect.anything());
  });

  it("startupSyncModelPricesOnce only runs once per session", async () => {
    const m = await importFreshStartup();
    vi.mocked(modelPricesSyncBasellm).mockResolvedValueOnce({
      status: "updated",
      inserted: 0,
      updated: 0,
      skipped: 0,
      total: 0,
    } as any);

    await m.startupSyncModelPricesOnce();
    expect(modelPricesSyncBasellm).toHaveBeenCalledTimes(1);

    await m.startupSyncModelPricesOnce();
    expect(modelPricesSyncBasellm).toHaveBeenCalledTimes(1);
  });

  it("startupSyncModelPricesOnce logs errors when sync throws", async () => {
    const m = await importFreshStartup();
    vi.mocked(modelPricesSyncBasellm).mockRejectedValueOnce(new Error("boom"));
    await m.startupSyncModelPricesOnce();
    expect(setLastModelPricesSync).not.toHaveBeenCalled();
    expect(logToConsole).toHaveBeenCalledWith("error", "启动同步：模型定价同步失败", {
      error: "Error: boom",
    });
  });

  it("startupSyncDefaultPromptsFromFilesOncePerSession dedupes and logs action summary", async () => {
    const m = await importFreshStartup();

    vi.mocked(promptsDefaultSyncFromFiles).mockResolvedValueOnce({
      items: [{ action: "add" }, { action: "error" }, { action: "add" }],
    } as any);

    const p1 = m.startupSyncDefaultPromptsFromFilesOncePerSession();
    const p2 = m.startupSyncDefaultPromptsFromFilesOncePerSession();
    expect(p1).toBe(p2);

    await p1;
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "初始化：default 提示词与本机文件同步完成",
      expect.objectContaining({
        summary: { add: 2, error: 1 },
      })
    );
  });

  it("startupSyncDefaultPromptsFromFilesOncePerSession logs errors when sync throws", async () => {
    const m = await importFreshStartup();
    vi.mocked(promptsDefaultSyncFromFiles).mockRejectedValueOnce(new Error("x"));
    await m.startupSyncDefaultPromptsFromFilesOncePerSession();
    expect(logToConsole).toHaveBeenCalledWith("error", "初始化：default 提示词与本机文件同步失败", {
      error: "Error: x",
    });
  });
});

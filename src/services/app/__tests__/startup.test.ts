import { beforeEach, describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import {
  modelPricesSyncBasellm,
  setLastModelPricesSync,
  type ModelPricesSyncReport,
} from "../../usage/modelPrices";
import { promptsDefaultSyncFromFiles, type DefaultPromptSyncReport } from "../../workspace/prompts";

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

function makeModelPricesSyncReport(
  overrides: Partial<ModelPricesSyncReport> = {}
): ModelPricesSyncReport {
  return {
    status: "updated",
    inserted: 1,
    updated: 0,
    skipped: 0,
    total: 1,
    ...overrides,
  };
}

function makeDefaultPromptSyncReport(
  overrides: Partial<DefaultPromptSyncReport> = {}
): DefaultPromptSyncReport {
  return {
    items: [
      {
        cli_key: "claude",
        action: "created",
        message: null,
      },
    ],
    ...overrides,
  };
}

describe("services/app/startup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("startupSyncModelPricesOnce always calls modelPricesSyncBasellm", async () => {
    const { startupSyncModelPricesOnce } = await importFreshStartup();

    const report = makeModelPricesSyncReport({
      inserted: 1,
      updated: 2,
      skipped: 3,
      total: 6,
    });

    vi.mocked(modelPricesSyncBasellm).mockResolvedValueOnce(report);

    await startupSyncModelPricesOnce();
    expect(modelPricesSyncBasellm).toHaveBeenCalledWith(false);
    expect(setLastModelPricesSync).toHaveBeenCalledWith(report);
    expect(logToConsole).toHaveBeenCalledWith(
      "info",
      "启动同步：模型定价同步完成",
      expect.objectContaining({ status: "updated", inserted: 1, updated: 2, skipped: 3, total: 6 })
    );
  });

  it("startupSyncModelPricesOnce only runs once per session", async () => {
    const m = await importFreshStartup();
    vi.mocked(modelPricesSyncBasellm).mockResolvedValueOnce(
      makeModelPricesSyncReport({
        inserted: 0,
        updated: 0,
        skipped: 0,
        total: 0,
      })
    );

    const firstRun = m.startupSyncModelPricesOnce();
    const dedupedRun = m.startupSyncModelPricesOnce();
    expect(dedupedRun).toBe(firstRun);

    await firstRun;
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

    vi.mocked(promptsDefaultSyncFromFiles).mockResolvedValueOnce(
      makeDefaultPromptSyncReport({
        items: [
          { cli_key: "claude", action: "created", message: null },
          { cli_key: "claude", action: "error", message: "broken" },
          { cli_key: "codex", action: "created", message: null },
        ],
      })
    );

    const p1 = m.startupSyncDefaultPromptsFromFilesOncePerSession();
    const p2 = m.startupSyncDefaultPromptsFromFilesOncePerSession();
    expect(p1).toBe(p2);

    await p1;
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "初始化：default 提示词与本机文件同步完成",
      expect.objectContaining({
        summary: { created: 2, error: 1 },
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

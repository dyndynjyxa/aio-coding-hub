import { logToConsole } from "../consoleLog";
import { modelPricesSyncBasellm, setLastModelPricesSync } from "../usage/modelPrices";
import { promptsDefaultSyncFromFiles } from "../workspace/prompts";

let modelPricesSyncPromise: Promise<void> | null = null;
let defaultPromptsSyncPromise: Promise<void> | null = null;

export function startupSyncModelPricesOnce(): Promise<void> {
  if (modelPricesSyncPromise) return modelPricesSyncPromise;

  modelPricesSyncPromise = (async () => {
    try {
      const report = await modelPricesSyncBasellm(false);
      setLastModelPricesSync(report);
      logToConsole("info", "启动同步：模型定价同步完成", {
        status: report.status,
        inserted: report.inserted,
        updated: report.updated,
        skipped: report.skipped,
        total: report.total,
      });
    } catch (err) {
      logToConsole("error", "启动同步：模型定价同步失败", { error: String(err) });
    }
  })();

  return modelPricesSyncPromise;
}

function summarizeDefaultPromptSyncActions(items: { action: string }[]) {
  const summary: Record<string, number> = {};
  for (const item of items) {
    const key = String(item.action || "unknown");
    summary[key] = (summary[key] ?? 0) + 1;
  }
  return summary;
}

export function startupSyncDefaultPromptsFromFilesOncePerSession(): Promise<void> {
  if (defaultPromptsSyncPromise) return defaultPromptsSyncPromise;

  defaultPromptsSyncPromise = (async () => {
    try {
      const report = await promptsDefaultSyncFromFiles();
      const summary = summarizeDefaultPromptSyncActions(report.items);
      const hasError = report.items.some((it) => it.action === "error");

      logToConsole(hasError ? "error" : "info", "初始化：default 提示词与本机文件同步完成", {
        summary,
        items: report.items,
      });
    } catch (err) {
      logToConsole("error", "初始化：default 提示词与本机文件同步失败", {
        error: String(err),
      });
    }
  })();

  return defaultPromptsSyncPromise;
}

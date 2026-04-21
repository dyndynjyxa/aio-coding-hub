import { useStartupTask } from "../hooks/useStartupTask";
import { syncAppStartupStatusSnapshot } from "./startupStatusStore";
import {
  startupSyncDefaultPromptsFromFilesOncePerSession,
  startupSyncModelPricesOnce,
} from "../services/app/startup";

export function useAppStartupTasks() {
  useStartupTask(syncAppStartupStatusSnapshot, "syncAppStartupStatusSnapshot", "启动状态同步失败");
  useStartupTask(startupSyncModelPricesOnce, "startupSyncModelPricesOnce", "启动模型定价同步失败");
  useStartupTask(
    startupSyncDefaultPromptsFromFilesOncePerSession,
    "startupSyncDefaultPromptsFromFilesOncePerSession",
    "启动默认提示词同步失败"
  );
}

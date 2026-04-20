import { useStartupTask } from "../hooks/useStartupTask";
import {
  startupSyncDefaultPromptsFromFilesOncePerSession,
  startupSyncModelPricesOnce,
} from "../services/app/startup";

export function useAppStartupTasks() {
  useStartupTask(startupSyncModelPricesOnce, "startupSyncModelPricesOnce", "启动模型定价同步失败");
  useStartupTask(
    startupSyncDefaultPromptsFromFilesOncePerSession,
    "startupSyncDefaultPromptsFromFilesOncePerSession",
    "启动默认提示词同步失败"
  );
}

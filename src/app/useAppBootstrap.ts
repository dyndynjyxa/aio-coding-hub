import { useAppBackgroundTasks } from "./useAppBackgroundTasks";
import { useAppEventListeners } from "./useAppEventListeners";
import { useAppRuntimeSync } from "./useAppRuntimeSync";
import { useAppStartupTasks } from "./useAppStartupTasks";

export function useAppBootstrap() {
  useAppRuntimeSync();
  useAppEventListeners();
  useAppStartupTasks();
  useAppBackgroundTasks();
}

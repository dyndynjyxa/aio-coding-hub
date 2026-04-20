import { useEffect } from "react";
import { useDocumentVisibility } from "../hooks/useDocumentVisibility";
import { queryClient } from "../query/queryClient";
import { cliProxyKeys, updaterKeys } from "../query/keys";
import { cliProxyStatusAll } from "../services/cli/cliProxy";
import {
  type BackgroundTaskDefinition,
  registerBackgroundTask,
  setBackgroundTaskSchedulerForeground,
  startBackgroundTaskScheduler,
} from "../services/backgroundTasks";
import { updateCheckNow } from "../hooks/useUpdateMeta";
import {
  appBackgroundTaskIds,
  backgroundTaskVisibilityTriggers,
} from "../constants/backgroundTaskContracts";

function buildCliProxyConsistencyTask(): BackgroundTaskDefinition {
  return {
    taskId: appBackgroundTaskIds.cliProxyConsistency,
    intervalMs: 15_000,
    runOnAppStart: true,
    foregroundOnly: true,
    visibilityTriggers: [backgroundTaskVisibilityTriggers.homeOverviewVisible],
    run: async () => {
      await queryClient.fetchQuery({
        queryKey: cliProxyKeys.statusAll(),
        queryFn: () => cliProxyStatusAll(),
        staleTime: 0,
      });
    },
  };
}

function buildAppUpdateCheckTask(): BackgroundTaskDefinition {
  return {
    taskId: appBackgroundTaskIds.appUpdateCheck,
    intervalMs: 300_000,
    runOnAppStart: true,
    foregroundOnly: true,
    visibilityTriggers: [],
    run: async (context) => {
      const options =
        context.trigger === "manual"
          ? {
              silent: false,
              openDialogIfUpdate: true,
            }
          : {
              silent: true,
              openDialogIfUpdate: false,
            };
      await updateCheckNow(options);
      queryClient.invalidateQueries({ queryKey: updaterKeys.check() });
    },
  };
}

export function useAppBackgroundTasks() {
  const documentVisible = useDocumentVisibility();

  useEffect(() => {
    const unregisterCliProxyTask = registerBackgroundTask(buildCliProxyConsistencyTask());
    const unregisterUpdateTask = registerBackgroundTask(buildAppUpdateCheckTask());

    startBackgroundTaskScheduler();

    return () => {
      unregisterCliProxyTask();
      unregisterUpdateTask();
    };
  }, []);

  useEffect(() => {
    setBackgroundTaskSchedulerForeground(documentVisible);
  }, [documentVisible]);
}

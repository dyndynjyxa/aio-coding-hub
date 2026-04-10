import { render } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient } from "../test/utils/reactQuery";
import { createTestAppSettings } from "../test/fixtures/settings";

vi.mock("../services/app/appHeartbeat", () => ({
  listenAppHeartbeat: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../services/gateway/gatewayEvents", () => ({
  listenGatewayEvents: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../services/notification/noticeEvents", () => ({
  listenNoticeEvents: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../services/notification/taskCompleteNotifyEvents", () => ({
  listenTaskCompleteNotifyEvents: vi.fn().mockResolvedValue(() => {}),
  setTaskCompleteNotifyEnabled: vi.fn(),
}));
vi.mock("../services/gateway/cacheAnomalyMonitor", () => ({
  setCacheAnomalyMonitorEnabled: vi.fn(),
}));
vi.mock("../services/app/startup", () => ({
  startupSyncDefaultPromptsFromFilesOncePerSession: vi.fn().mockResolvedValue(undefined),
  startupSyncModelPricesOnce: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../app/AppRoutes", () => ({
  AppRoutes: () => <div data-testid="app-routes" />,
}));
vi.mock("../services/backgroundTasks", () => ({
  registerBackgroundTask: vi.fn(() => vi.fn()),
  startBackgroundTaskScheduler: vi.fn(),
  setBackgroundTaskSchedulerForeground: vi.fn(),
  emitBackgroundTaskVisibilityTrigger: vi.fn(),
}));
vi.mock("../services/cli/cliProxy", () => ({
  cliProxyStatusAll: vi.fn().mockResolvedValue([]),
}));
vi.mock("../hooks/useUpdateMeta", async () => {
  const actual =
    await vi.importActual<typeof import("../hooks/useUpdateMeta")>("../hooks/useUpdateMeta");
  return {
    ...actual,
    updateCheckNow: vi.fn().mockResolvedValue(null),
  };
});
vi.mock("../services/settings/settings", async () => {
  const actual = await vi.importActual<typeof import("../services/settings/settings")>(
    "../services/settings/settings"
  );
  return {
    ...actual,
    settingsGet: vi.fn(),
  };
});

import { listenAppHeartbeat } from "../services/app/appHeartbeat";
import { setCacheAnomalyMonitorEnabled } from "../services/gateway/cacheAnomalyMonitor";
import {
  registerBackgroundTask,
  setBackgroundTaskSchedulerForeground,
  startBackgroundTaskScheduler,
} from "../services/backgroundTasks";
import { listenGatewayEvents } from "../services/gateway/gatewayEvents";
import { listenNoticeEvents } from "../services/notification/noticeEvents";
import { settingsGet } from "../services/settings/settings";
import {
  startupSyncDefaultPromptsFromFilesOncePerSession,
  startupSyncModelPricesOnce,
} from "../services/app/startup";
import {
  listenTaskCompleteNotifyEvents,
  setTaskCompleteNotifyEnabled,
} from "../services/notification/taskCompleteNotifyEvents";
import { updateCheckNow } from "../hooks/useUpdateMeta";
import { cliProxyStatusAll } from "../services/cli/cliProxy";

async function renderApp() {
  const { default: App } = await import("../App");
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>
  );
}

describe("App bootstrap", () => {
  beforeEach(() => {
    vi.mocked(listenAppHeartbeat).mockResolvedValue(() => {});
    vi.mocked(listenGatewayEvents).mockResolvedValue(() => {});
    vi.mocked(listenNoticeEvents).mockResolvedValue(() => {});
    vi.mocked(listenTaskCompleteNotifyEvents).mockResolvedValue(() => {});
    vi.mocked(startupSyncModelPricesOnce).mockResolvedValue(undefined);
    vi.mocked(startupSyncDefaultPromptsFromFilesOncePerSession).mockResolvedValue(undefined);
    vi.mocked(settingsGet).mockResolvedValue(
      createTestAppSettings({
        enable_cache_anomaly_monitor: true,
        enable_task_complete_notify: false,
      })
    );
  });

  it("wires listeners, startup tasks, and settings-driven toggles", async () => {
    await renderApp();

    await vi.waitFor(() => {
      expect(listenAppHeartbeat).toHaveBeenCalledTimes(1);
      expect(listenGatewayEvents).toHaveBeenCalledTimes(1);
      expect(listenNoticeEvents).toHaveBeenCalledTimes(1);
      expect(listenTaskCompleteNotifyEvents).toHaveBeenCalledTimes(1);
      expect(startupSyncModelPricesOnce).toHaveBeenCalledTimes(1);
      expect(startupSyncDefaultPromptsFromFilesOncePerSession).toHaveBeenCalledTimes(1);
      expect(setCacheAnomalyMonitorEnabled).toHaveBeenCalledWith(true);
      expect(setTaskCompleteNotifyEnabled).toHaveBeenCalledWith(false);
      expect(registerBackgroundTask).toHaveBeenCalledTimes(2);
      expect(startBackgroundTaskScheduler).toHaveBeenCalledTimes(1);
      expect(setBackgroundTaskSchedulerForeground).toHaveBeenCalledWith(true);
      expect(updateCheckNow).not.toHaveBeenCalled();
      expect(cliProxyStatusAll).not.toHaveBeenCalled();
    });
  });
});

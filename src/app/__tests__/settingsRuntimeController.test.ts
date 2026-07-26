import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  applySettingsRuntimeSnapshot,
  resetSettingsRuntimeController,
  type SettingsRuntimeSnapshot,
} from "../settingsRuntimeController";
import { setCacheAnomalyMonitorEnabled } from "../../services/gateway/cacheAnomalyMonitor";
import { setCircuitBreakerNoticeEnabled } from "../../services/gateway/circuitNotice";
import { setNotificationSoundEnabled } from "../../services/notification/notificationSound";
import { setTaskCompleteNotifyEnabled } from "../../services/notification/taskCompleteNotifyEvents";

vi.mock("../../services/gateway/cacheAnomalyMonitor", () => ({
  setCacheAnomalyMonitorEnabled: vi.fn(),
}));

vi.mock("../../services/gateway/circuitNotice", () => ({
  setCircuitBreakerNoticeEnabled: vi.fn(),
}));

vi.mock("../../services/notification/notificationSound", () => ({
  setNotificationSoundEnabled: vi.fn(),
}));

vi.mock("../../services/notification/taskCompleteNotifyEvents", () => ({
  setTaskCompleteNotifyEnabled: vi.fn(),
}));

const runtimeSnapshot: SettingsRuntimeSnapshot = {
  enableCacheAnomalyMonitor: true,
  enableTaskCompleteNotify: false,
  enableNotificationSound: true,
  enableCircuitBreakerNotice: true,
};

describe("app/settingsRuntimeController", () => {
  beforeEach(() => {
    resetSettingsRuntimeController();
  });

  it("applies normalized app settings once and skips duplicate snapshots", () => {
    applySettingsRuntimeSnapshot({
      enable_cache_anomaly_monitor: true,
      enable_task_complete_notify: false,
      enable_notification_sound: true,
      enable_circuit_breaker_notice: true,
    } as any);
    applySettingsRuntimeSnapshot({ ...runtimeSnapshot });

    expect(setCacheAnomalyMonitorEnabled).toHaveBeenCalledTimes(1);
    expect(setCacheAnomalyMonitorEnabled).toHaveBeenCalledWith(true);
    expect(setTaskCompleteNotifyEnabled).toHaveBeenCalledTimes(1);
    expect(setTaskCompleteNotifyEnabled).toHaveBeenCalledWith(false);
    expect(setNotificationSoundEnabled).toHaveBeenCalledTimes(1);
    expect(setNotificationSoundEnabled).toHaveBeenCalledWith(true);
    expect(setCircuitBreakerNoticeEnabled).toHaveBeenCalledTimes(1);
    expect(setCircuitBreakerNoticeEnabled).toHaveBeenCalledWith(true);
  });

  it("ignores empty settings and reapplies after reset", () => {
    applySettingsRuntimeSnapshot(null);
    applySettingsRuntimeSnapshot(undefined);
    expect(setCacheAnomalyMonitorEnabled).not.toHaveBeenCalled();

    applySettingsRuntimeSnapshot(runtimeSnapshot);
    resetSettingsRuntimeController();
    applySettingsRuntimeSnapshot(runtimeSnapshot);

    expect(setCacheAnomalyMonitorEnabled).toHaveBeenCalledTimes(2);
    expect(setTaskCompleteNotifyEnabled).toHaveBeenCalledTimes(2);
    expect(setNotificationSoundEnabled).toHaveBeenCalledTimes(2);
    expect(setCircuitBreakerNoticeEnabled).toHaveBeenCalledTimes(2);
  });

  it("applies changes when any runtime value differs", () => {
    applySettingsRuntimeSnapshot(runtimeSnapshot);
    applySettingsRuntimeSnapshot({
      ...runtimeSnapshot,
      enableNotificationSound: false,
      enableCircuitBreakerNotice: false,
    });

    expect(setCacheAnomalyMonitorEnabled).toHaveBeenLastCalledWith(true);
    expect(setTaskCompleteNotifyEnabled).toHaveBeenLastCalledWith(false);
    expect(setNotificationSoundEnabled).toHaveBeenLastCalledWith(false);
    expect(setCircuitBreakerNoticeEnabled).toHaveBeenLastCalledWith(false);
  });
});

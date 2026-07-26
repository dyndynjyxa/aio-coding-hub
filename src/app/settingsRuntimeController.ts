import type { AppSettings } from "../services/settings/settings";
import { setCacheAnomalyMonitorEnabled } from "../services/gateway/cacheAnomalyMonitor";
import { setCircuitBreakerNoticeEnabled } from "../services/gateway/circuitNotice";
import { setNotificationSoundEnabled } from "../services/notification/notificationSound";
import { setTaskCompleteNotifyEnabled } from "../services/notification/taskCompleteNotifyEvents";

export type SettingsRuntimeSnapshot = {
  enableCacheAnomalyMonitor: boolean;
  enableTaskCompleteNotify: boolean;
  enableNotificationSound: boolean;
  enableCircuitBreakerNotice: boolean;
};

let lastAppliedSnapshot: SettingsRuntimeSnapshot | null = null;

function normalizeSettingsRuntimeSnapshot(
  settings: AppSettings | SettingsRuntimeSnapshot | null | undefined
): SettingsRuntimeSnapshot | null {
  if (!settings) {
    return null;
  }

  if ("enableCacheAnomalyMonitor" in settings) {
    return settings;
  }

  return {
    enableCacheAnomalyMonitor: settings.enable_cache_anomaly_monitor,
    enableTaskCompleteNotify: settings.enable_task_complete_notify,
    enableNotificationSound: settings.enable_notification_sound,
    enableCircuitBreakerNotice: settings.enable_circuit_breaker_notice,
  };
}

function sameSnapshot(
  left: SettingsRuntimeSnapshot | null,
  right: SettingsRuntimeSnapshot | null
): boolean {
  return (
    left?.enableCacheAnomalyMonitor === right?.enableCacheAnomalyMonitor &&
    left?.enableTaskCompleteNotify === right?.enableTaskCompleteNotify &&
    left?.enableNotificationSound === right?.enableNotificationSound &&
    left?.enableCircuitBreakerNotice === right?.enableCircuitBreakerNotice
  );
}

export function applySettingsRuntimeSnapshot(
  settings: AppSettings | SettingsRuntimeSnapshot | null | undefined
) {
  const snapshot = normalizeSettingsRuntimeSnapshot(settings);
  if (!snapshot || sameSnapshot(lastAppliedSnapshot, snapshot)) {
    return;
  }

  lastAppliedSnapshot = snapshot;
  setCacheAnomalyMonitorEnabled(snapshot.enableCacheAnomalyMonitor);
  setTaskCompleteNotifyEnabled(snapshot.enableTaskCompleteNotify);
  setNotificationSoundEnabled(snapshot.enableNotificationSound);
  setCircuitBreakerNoticeEnabled(snapshot.enableCircuitBreakerNotice);
}

export function resetSettingsRuntimeController() {
  lastAppliedSnapshot = null;
}

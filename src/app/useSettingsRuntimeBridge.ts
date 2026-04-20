import { useEffect } from "react";
import { useSettingsQuery } from "../query/settings";
import { setCacheAnomalyMonitorEnabled } from "../services/gateway/cacheAnomalyMonitor";
import { setNotificationSoundEnabled } from "../services/notification/notificationSound";
import { setTaskCompleteNotifyEnabled } from "../services/notification/taskCompleteNotifyEvents";

export function useSettingsRuntimeBridge() {
  const settingsQuery = useSettingsQuery();
  const settings = settingsQuery.data ?? null;

  useEffect(() => {
    if (!settings) return;
    setCacheAnomalyMonitorEnabled(settings.enable_cache_anomaly_monitor);
    setTaskCompleteNotifyEnabled(settings.enable_task_complete_notify);
    setNotificationSoundEnabled(settings.enable_notification_sound);
  }, [settings]);
}

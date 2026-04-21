import { useEffect } from "react";
import { useSettingsQuery } from "../query/settings";
import { applySettingsRuntimeSnapshot } from "./settingsRuntimeController";

export function useSettingsRuntimeBridge() {
  const settingsQuery = useSettingsQuery();
  const settings = settingsQuery.data ?? null;

  useEffect(() => {
    if (!settings) return;
    applySettingsRuntimeSnapshot(settings);
  }, [settings]);
}

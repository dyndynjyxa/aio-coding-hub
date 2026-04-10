import { listen } from "@tauri-apps/api/event";

import { appEventNames } from "../../constants/appEvents";
import { invokeTauriOrNull } from "../tauriInvoke";

export type AppHeartbeatPayload = {
  ts_unix_ms: number;
};

export async function listenAppHeartbeat(): Promise<() => void> {
  let inFlight = false;

  const unlisten = await listen<AppHeartbeatPayload>(appEventNames.heartbeat, () => {
    if (inFlight) return;
    inFlight = true;

    invokeTauriOrNull<boolean>("app_heartbeat_pong", undefined, { timeoutMs: 3_000 })
      .catch(() => null)
      .finally(() => {
        inFlight = false;
      });
  });

  return () => {
    unlisten();
  };
}

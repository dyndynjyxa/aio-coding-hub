import { appEventNames } from "../../constants/appEvents";
import { listenDesktopEvent } from "../desktop/event";
import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type AppHeartbeatPayload = {
  ts_unix_ms: number;
};

export async function appHeartbeatPong() {
  return invokeGeneratedIpc<boolean>({
    title: "应用心跳响应失败",
    cmd: "app_heartbeat_pong",
    invoke: () => commands.appHeartbeatPong() as Promise<GeneratedCommandResult<boolean>>,
  });
}

// A pong invoke that neither resolves nor rejects (stuck IPC) must not block
// heartbeats forever — after this long the in-flight guard is considered stale
// and the next heartbeat sends a fresh pong.
export const PONG_IN_FLIGHT_STALE_MS = 10_000;

export async function listenAppHeartbeat(): Promise<() => void> {
  let inFlightSinceMs = 0;

  const unlisten = await listenDesktopEvent<AppHeartbeatPayload>(appEventNames.heartbeat, () => {
    const now = Date.now();
    if (inFlightSinceMs !== 0 && now - inFlightSinceMs < PONG_IN_FLIGHT_STALE_MS) return;
    const startedAtMs = now;
    inFlightSinceMs = startedAtMs;

    appHeartbeatPong()
      .catch(() => null)
      .finally(() => {
        // A stale invoke that settles late must not clear a guard that has
        // since been taken over by a newer in-flight pong.
        if (inFlightSinceMs === startedAtMs) {
          inFlightSinceMs = 0;
        }
      });
  });

  return () => {
    unlisten();
  };
}

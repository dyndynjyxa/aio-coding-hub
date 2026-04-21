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
    invoke: () =>
      commands.appHeartbeatPong() as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function listenAppHeartbeat(): Promise<() => void> {
  let inFlight = false;

  const unlisten = await listenDesktopEvent<AppHeartbeatPayload>(appEventNames.heartbeat, () => {
    if (inFlight) return;
    inFlight = true;

    appHeartbeatPong()
      .catch(() => null)
      .finally(() => {
        inFlight = false;
      });
  });

  return () => {
    unlisten();
  };
}

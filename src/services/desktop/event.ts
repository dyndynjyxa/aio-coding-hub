import { listen as tauriListen } from "@tauri-apps/api/event";
import { observePromiseLikeRejection, type MaybePromiseLike } from "../../utils/promiseLike";
import { logToConsole } from "../consoleLog";

function logDesktopEventHandlerError(event: string, error: unknown) {
  logToConsole("warn", "桌面事件处理失败", { event, error: String(error) }, "desktop:event");
}

export async function listenDesktopEvent<TPayload>(
  event: string,
  handler: (payload: TPayload) => MaybePromiseLike<void>
): Promise<() => void> {
  const unlisten = await tauriListen<TPayload>(event, (evt) => {
    try {
      const result = handler(evt.payload);
      observePromiseLikeRejection(result, (error) => logDesktopEventHandlerError(event, error));
    } catch (error) {
      logDesktopEventHandlerError(event, error);
    }
  });

  return () => {
    unlisten();
  };
}

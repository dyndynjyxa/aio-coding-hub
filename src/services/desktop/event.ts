import { listen as tauriListen } from "@tauri-apps/api/event";

export async function listenDesktopEvent<TPayload>(
  event: string,
  handler: (payload: TPayload) => void | Promise<void>
): Promise<() => void> {
  const unlisten = await tauriListen<TPayload>(event, async (evt) => {
    await handler(evt.payload);
  });

  return () => {
    unlisten();
  };
}

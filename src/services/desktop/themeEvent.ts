import { getCurrentWindow } from "@tauri-apps/api/window";

export type TauriTheme = "light" | "dark";

/**
 * Listen for Tauri native theme change events.
 * The payload is the theme string directly ("light" | "dark"), per Tauri 2's
 * `onThemeChanged` contract — not wrapped in an object.
 */
export async function listenThemeChanged(
  handler: (theme: TauriTheme) => void | Promise<void>
): Promise<() => void> {
  return await getCurrentWindow().onThemeChanged(({ payload }) => {
    handler(payload as TauriTheme);
  });
}

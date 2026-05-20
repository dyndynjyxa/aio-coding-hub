import { getCurrentWindow } from "@tauri-apps/api/window";
import { observePromiseLikeRejection, type MaybePromiseLike } from "../../utils/promiseLike";
import { logToConsole } from "../consoleLog";

export type TauriTheme = "light" | "dark";
const TAURI_THEME_VALUES = ["light", "dark"] as const satisfies readonly TauriTheme[];

function normalizeTauriTheme(value: unknown): TauriTheme | null {
  if (typeof value !== "string") return null;
  return (TAURI_THEME_VALUES as readonly string[]).includes(value) ? (value as TauriTheme) : null;
}

function logThemeHandlerError(error: unknown) {
  logToConsole("warn", "系统主题事件处理失败", { error: String(error) }, "desktop:theme");
}

/**
 * Listen for Tauri native theme change events.
 * The payload is the theme string directly ("light" | "dark"), per Tauri 2's
 * `onThemeChanged` contract — not wrapped in an object.
 */
export async function listenThemeChanged(
  handler: (theme: TauriTheme) => MaybePromiseLike<void>
): Promise<() => void> {
  return await getCurrentWindow().onThemeChanged(({ payload }) => {
    const theme = normalizeTauriTheme(payload);
    if (!theme) return;
    try {
      const result = handler(theme);
      observePromiseLikeRejection(result, logThemeHandlerError);
    } catch (error) {
      logThemeHandlerError(error);
    }
  });
}

import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type DesktopTheme = "light" | "dark" | "system";
const DESKTOP_THEME_VALUES = ["light", "dark", "system"] as const satisfies readonly DesktopTheme[];

function normalizeDesktopTheme(theme: unknown): DesktopTheme {
  if (typeof theme !== "string") {
    throw new Error("SEC_INVALID_INPUT: desktop theme must be a string");
  }
  const normalized = theme.trim();
  if ((DESKTOP_THEME_VALUES as readonly string[]).includes(normalized)) {
    return normalized as DesktopTheme;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid desktop theme=${String(theme)}`);
}

export async function setDesktopWindowTheme(theme: DesktopTheme) {
  const normalizedTheme = normalizeDesktopTheme(theme);

  return invokeGeneratedIpc<boolean>({
    title: "同步窗口主题失败",
    cmd: "desktop_window_set_theme",
    args: { theme: normalizedTheme },
    invoke: () =>
      commands.desktopWindowSetTheme(normalizedTheme) as Promise<GeneratedCommandResult<boolean>>,
  });
}
